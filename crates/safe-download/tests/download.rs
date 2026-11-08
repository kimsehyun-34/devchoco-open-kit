//! End-to-end tests against a throwaway `std::net` HTTP server on the loopback interface.

mod common;

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use common::{local_options, respond, run, send_head, wait_for_disconnect, Scratch, Server};
use safe_download::{download, CancelToken, Collision, Error, Hooks, Options, RedirectPolicy};

fn body(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// Serves `data` for every path.
fn serve_bytes(data: Vec<u8>) -> Server {
    Server::start(move |_, stream| respond(stream, "200 OK", &[], &data))
}

type Outcome = Result<safe_download::Saved, Error>;

fn fetch_with(
    server: &Server,
    dir: &Scratch,
    name: &str,
    options: &Options,
    hooks: Hooks,
) -> Outcome {
    run(download(&server.url("/file"), dir.path(), name, options, hooks))
}

fn fetch(server: &Server, dir: &Scratch, name: &str, options: &Options) -> Outcome {
    fetch_with(server, dir, name, options, Hooks::new())
}

#[test]
fn a_download_becomes_the_file_and_nothing_else() {
    let data = body(200_000);
    let server = serve_bytes(data.clone());
    let dir = Scratch::new("success");
    let saved = fetch(&server, &dir, "report.pdf", &local_options()).unwrap();
    assert_eq!(saved.path, dir.path().join("report.pdf"));
    assert_eq!(saved.bytes, 200_000);
    assert_eq!(std::fs::read(&saved.path).unwrap(), data);
    assert_eq!(dir.names(), ["report.pdf"]);
    assert_eq!(server.hits(), 1);
}

#[test]
fn an_empty_file_is_saved_as_an_empty_file() {
    let server = serve_bytes(Vec::new());
    let dir = Scratch::new("empty");
    let saved = fetch(&server, &dir, "empty.txt", &local_options()).unwrap();
    assert_eq!(saved.bytes, 0);
    assert_eq!(std::fs::read(&saved.path).unwrap(), b"");
    assert_eq!(dir.names(), ["empty.txt"]);
}

#[test]
fn progress_is_strictly_increasing_and_ends_at_the_size() {
    let data = body(80_000);
    let server = Server::start(move |_, stream| {
        send_head(stream, "200 OK", &[("Content-Length", "80000")]);
        for part in data.chunks(16_000) {
            let _ = stream.write_all(part);
            let _ = stream.flush();
            std::thread::sleep(Duration::from_millis(25));
        }
    });
    let dir = Scratch::new("progress");
    let log = Arc::new(Mutex::new(Vec::new()));
    let hooks = {
        let log = log.clone();
        let watched = dir.path().to_path_buf();
        Hooks::new().on_progress(move |bytes, total| {
            // What is on disk at the moment of the report.
            let mut names: Vec<String> = std::fs::read_dir(&watched)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            log.lock().unwrap().push((bytes, total, names));
        })
    };
    let saved = fetch_with(&server, &dir, "movie.bin", &local_options(), hooks).unwrap();
    assert_eq!(saved.bytes, 80_000);

    let log = log.lock().unwrap();
    assert!(log.len() >= 2, "{log:?}");
    assert_eq!((log[0].0, log[0].1), (0, Some(80_000)));
    let last = log.last().unwrap();
    assert_eq!((last.0, last.1), (80_000, Some(80_000)));
    assert!(log.windows(2).all(|pair| pair[0].0 < pair[1].0), "{log:?}");
    assert!(log.iter().all(|entry| entry.1 == Some(80_000)));
    // While bytes are arriving only the temp file exists, next to the destination.
    for entry in log.iter().filter(|entry| entry.0 > 0) {
        assert_eq!(entry.2, ["movie.bin.part"], "{entry:?}");
    }
    assert_eq!(dir.names(), ["movie.bin"]);
}

#[test]
fn progress_without_a_declared_length_reports_no_total() {
    let server = Server::start(|_, stream| {
        send_head(stream, "200 OK", &[]);
        let _ = stream.write_all(&body(30_000));
    });
    let dir = Scratch::new("no-total");
    let log = Arc::new(Mutex::new(Vec::new()));
    let hooks = {
        let log = log.clone();
        Hooks::new().on_progress(move |bytes, total| log.lock().unwrap().push((bytes, total)))
    };
    let saved = fetch_with(&server, &dir, "stream.bin", &local_options(), hooks).unwrap();
    assert_eq!(saved.bytes, 30_000);
    let log = log.lock().unwrap();
    assert!(log.iter().all(|entry| entry.1.is_none()), "{log:?}");
    assert_eq!(log.last().unwrap().0, 30_000);
}

#[test]
fn cancelling_mid_stream_leaves_no_files_and_closes_the_connection() {
    let disconnected = Arc::new(AtomicBool::new(false));
    let server = {
        let disconnected = disconnected.clone();
        Server::start(move |_, stream| {
            send_head(stream, "200 OK", &[("Content-Length", "1000000")]);
            let _ = stream.write_all(&body(5_000));
            let _ = stream.flush();
            // Then stall, as a hung server would.
            let closed = wait_for_disconnect(stream, Duration::from_secs(10));
            disconnected.store(closed, Ordering::SeqCst);
        })
    };
    let dir = Scratch::new("cancel");
    let cancel = CancelToken::new();
    let (first_bytes, arrived) = mpsc::channel::<()>();
    let canceller = {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            // Bytes are flowing; give the download time to block on the stalled read.
            arrived.recv_timeout(Duration::from_secs(10)).unwrap();
            std::thread::sleep(Duration::from_millis(150));
            cancel.cancel();
        })
    };
    let hooks = Hooks::new()
        .on_progress(move |bytes, _| {
            if bytes > 0 {
                let _ = first_bytes.send(());
            }
        })
        .with_cancel(cancel);

    let mut options = local_options();
    options.read_timeout = Duration::from_secs(30); // a timeout must not be what ends it
    let started = Instant::now();
    let result = fetch_with(&server, &dir, "big.iso.zip", &options, hooks);
    canceller.join().unwrap();

    assert!(matches!(result, Err(Error::Cancelled)), "{result:?}");
    assert!(started.elapsed() < Duration::from_secs(8), "took {:?}", started.elapsed());
    assert!(dir.names().is_empty(), "{:?}", dir.names());
    let deadline = Instant::now() + Duration::from_secs(5);
    while !disconnected.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(disconnected.load(Ordering::SeqCst), "the server never saw the connection close");
}

#[test]
fn a_panicking_callback_still_removes_the_partial_file() {
    let server = serve_bytes(body(10_000));
    let dir = Scratch::new("panic");
    let hooks = Hooks::new().on_progress(|bytes, _| {
        if bytes > 0 {
            panic!("callback failed");
        }
    });
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fetch_with(&server, &dir, "a.bin", &local_options(), hooks)
    }));
    assert!(outcome.is_err(), "the panic should have propagated");
    assert!(dir.names().is_empty(), "{:?}", dir.names());
}

#[test]
fn a_token_that_already_fired_makes_no_request() {
    let server = serve_bytes(body(10));
    let dir = Scratch::new("pre-cancel");
    let cancel = CancelToken::new();
    cancel.cancel();
    let hooks = Hooks::new().with_cancel(cancel);
    let result = fetch_with(&server, &dir, "a.txt", &local_options(), hooks);
    assert!(matches!(result, Err(Error::Cancelled)), "{result:?}");
    assert_eq!(server.hits(), 0);
    assert!(dir.names().is_empty());
}

#[test]
fn a_truncated_body_is_rejected_and_cleaned_up() {
    let server = Server::start(|_, stream| {
        send_head(stream, "200 OK", &[("Content-Length", "100")]);
        let _ = stream.write_all(&body(40));
        // Close with 60 bytes still owed.
    });
    let dir = Scratch::new("truncated");
    let result = fetch(&server, &dir, "short.bin", &local_options());
    match result {
        Err(Error::Truncated { expected: Some(100), received }) => {
            assert!(received <= 40, "{received}");
        }
        other => panic!("expected a truncated transfer, got {other:?}"),
    }
    assert!(dir.names().is_empty(), "{:?}", dir.names());
}

#[test]
fn an_existing_file_is_never_overwritten() {
    let server = serve_bytes(b"new content".to_vec());
    let dir = Scratch::new("collision");
    std::fs::write(dir.path().join("report.pdf"), b"mine").unwrap();
    let options = local_options();

    let first = fetch(&server, &dir, "report.pdf", &options).unwrap();
    let second = fetch(&server, &dir, "report.pdf", &options).unwrap();
    assert_eq!(first.path.file_name().unwrap(), "report (1).pdf");
    assert_eq!(second.path.file_name().unwrap(), "report (2).pdf");
    assert_eq!(std::fs::read(dir.path().join("report.pdf")).unwrap(), b"mine");
    assert_eq!(std::fs::read(&first.path).unwrap(), b"new content");
    assert_eq!(dir.names(), ["report (1).pdf", "report (2).pdf", "report.pdf"]);
}

#[test]
fn the_fail_policy_refuses_before_any_request() {
    let server = serve_bytes(b"new content".to_vec());
    let dir = Scratch::new("collision-fail");
    std::fs::write(dir.path().join("report.pdf"), b"mine").unwrap();
    let mut options = local_options();
    options.collision = Collision::Fail;
    let result = fetch(&server, &dir, "report.pdf", &options);
    assert!(matches!(result, Err(Error::Exists)), "{result:?}");
    assert_eq!(server.hits(), 0);
    assert_eq!(std::fs::read(dir.path().join("report.pdf")).unwrap(), b"mine");
    assert_eq!(dir.names(), ["report.pdf"]);
    // A free name is still fine.
    assert!(fetch(&server, &dir, "other.pdf", &options).is_ok());
}

#[test]
fn blocked_extensions_are_refused_before_any_request() {
    let server = serve_bytes(body(10));
    let dir = Scratch::new("blocked");
    let options = local_options();
    let names =
        ["setup.exe", "RUN.BAT", "a.b.ps1", "x.lnk", "evil.exe ", "..\\sneaky.scr", "r.csv.msi"];
    for name in names {
        match fetch(&server, &dir, name, &options) {
            Err(Error::Blocked(extension)) => {
                assert_eq!(extension, extension.to_ascii_lowercase(), "{name}");
                assert!(!extension.contains('.'), "{name}");
            }
            other => panic!("{name}: expected Blocked, got {other:?}"),
        }
    }
    assert_eq!(server.hits(), 0);
    assert!(dir.names().is_empty());

    // The list is the caller's to change: an empty one allows everything,
    // a custom one blocks exactly what it names.
    let mut open = local_options();
    open.blocked_extensions.clear();
    assert!(fetch(&server, &dir, "setup.exe", &open).is_ok());
    let mut csv_only = local_options();
    csv_only.blocked_extensions = vec![".CSV".into()];
    let csv = fetch(&server, &dir, "table.csv", &csv_only);
    assert!(matches!(csv, Err(Error::Blocked(ext)) if ext == "csv"));
    assert!(fetch(&server, &dir, "tool.cmd", &csv_only).is_ok());
}

#[test]
fn hosts_outside_the_allow_list_are_refused_before_any_request() {
    let server = serve_bytes(body(10));
    let dir = Scratch::new("hosts");
    let mut localhost_only = local_options();
    localhost_only.allowed_hosts = vec!["localhost".into()];
    let result = fetch(&server, &dir, "a.txt", &localhost_only);
    assert!(matches!(&result, Err(Error::HostNotAllowed(host)) if host == "127.0.0.1"));

    let mut nobody = local_options();
    nobody.allowed_hosts.clear();
    assert!(matches!(fetch(&server, &dir, "a.txt", &nobody), Err(Error::HostNotAllowed(_))));

    // Credentials, lookalike hosts and garbage never reach the network either.
    let options = local_options();
    let port = server.url("").rsplit(':').next().unwrap().to_owned();
    for url in [
        format!("http://user:secret@127.0.0.1:{port}/file"),
        format!("http://127.0.0.1.evil.test:{port}/file"),
        format!("http://127.0.0.1@evil.test:{port}/file"),
        "not a url".to_owned(),
        "ftp://127.0.0.1/file".to_owned(),
    ] {
        let result = run(download(&url, dir.path(), "a.txt", &options, Hooks::new()));
        assert!(
            matches!(
                result,
                Err(Error::InvalidUrl | Error::HostNotAllowed(_) | Error::SchemeNotAllowed)
            ),
            "{url}: {result:?}"
        );
    }
    assert_eq!(server.hits(), 0);
    assert!(dir.names().is_empty());
}

#[test]
fn plain_http_is_refused_unless_explicitly_allowed() {
    let server = serve_bytes(body(10));
    let dir = Scratch::new("https-only");
    let mut options = local_options();
    options.https_only = true;
    let result = fetch(&server, &dir, "a.txt", &options);
    assert!(matches!(result, Err(Error::SchemeNotAllowed)), "{result:?}");
    assert_eq!(server.hits(), 0);
}

#[test]
fn unusable_names_are_refused_before_any_request() {
    let server = serve_bytes(body(10));
    let dir = Scratch::new("names");
    for name in ["...", "", "   ", "dir/", "\u{200B}"] {
        let result = fetch(&server, &dir, name, &local_options());
        assert!(matches!(result, Err(Error::InvalidName)), "{name:?}: {result:?}");
    }
    assert_eq!(server.hits(), 0);
}

#[test]
fn names_cannot_escape_the_destination_directory() {
    let server = serve_bytes(b"x".to_vec());
    let dir = Scratch::new("escape");
    let nested = dir.path().join("inside");
    std::fs::create_dir(&nested).unwrap();
    let options = local_options();
    let cases = [
        ("..\\..\\evil.txt", "evil.txt"),
        ("../../up.txt", "up.txt"),
        ("/etc/cron.txt", "cron.txt"),
        ("con.txt", "_con.txt"),
    ];
    for (asked, saved_as) in cases {
        let url = server.url("/file");
        let saved = run(download(&url, &nested, asked, &options, Hooks::new())).unwrap();
        assert_eq!(saved.path, nested.join(saved_as), "{asked}");
    }
    // Nothing was written next to the directory, only inside it.
    assert_eq!(dir.names(), ["inside"]);
}

#[test]
fn a_missing_destination_directory_is_a_local_error_without_a_request() {
    let server = serve_bytes(body(10));
    let dir = Scratch::new("missing-dir");
    let missing = dir.path().join("nope");
    let url = server.url("/file");
    let result = run(download(&url, &missing, "a.txt", &local_options(), Hooks::new()));
    assert!(
        matches!(&result, Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound),
        "{result:?}"
    );
    assert_eq!(server.hits(), 0);
}

fn redirecting_server() -> Server {
    Server::start(|request, stream| match request.path.as_str() {
        "/moved" => respond(stream, "302 Found", &[("Location", "/final")], b""),
        "/away" => {
            let elsewhere = [("Location", "http://localhost/elsewhere")];
            respond(stream, "301 Moved Permanently", &elsewhere, b"");
        }
        "/loop" => respond(stream, "302 Found", &[("Location", "/loop")], b""),
        _ => respond(stream, "200 OK", &[], b"arrived"),
    })
}

fn fetch_path(
    server: &Server,
    path: &str,
    dir: &Scratch,
    name: &str,
    options: &Options,
) -> Outcome {
    run(download(&server.url(path), dir.path(), name, options, Hooks::new()))
}

#[test]
fn redirects_are_refused_by_default() {
    let server = redirecting_server();
    let dir = Scratch::new("redirect-none");
    let result = fetch_path(&server, "/moved", &dir, "a.txt", &local_options());
    assert!(matches!(result, Err(Error::Redirected(302))), "{result:?}");
    // The redirect target was never requested.
    assert_eq!(server.hits(), 1);
    assert!(dir.names().is_empty());
}

#[test]
fn limited_redirects_follow_allowed_hops_only() {
    let server = redirecting_server();
    let dir = Scratch::new("redirect-limited");
    let mut options = local_options();
    options.redirects = RedirectPolicy::Limited(3);

    let saved = fetch_path(&server, "/moved", &dir, "a.txt", &options).unwrap();
    assert_eq!(std::fs::read(saved.path).unwrap(), b"arrived");
    assert_eq!(server.hits(), 2);

    // A hop to a host outside the allow-list is not followed.
    let before = server.hits();
    let away = fetch_path(&server, "/away", &dir, "b.txt", &options);
    assert!(matches!(away, Err(Error::Redirected(301))), "{away:?}");
    assert_eq!(server.hits(), before + 1);

    // A loop ends after the allowed number of hops: the original plus three followed ones.
    let before = server.hits();
    let looped = fetch_path(&server, "/loop", &dir, "c.txt", &options);
    assert!(matches!(looped, Err(Error::Redirected(302))), "{looped:?}");
    assert_eq!(server.hits(), before + 4);
    assert_eq!(dir.names(), ["a.txt"]);
}

#[test]
fn oversize_downloads_are_refused() {
    let dir = Scratch::new("oversize");
    let mut options = local_options();
    options.max_bytes = 1_000;

    // Declared up front: refused before a single byte is stored.
    let declared = serve_bytes(body(5_000));
    let result = fetch(&declared, &dir, "big.bin", &options);
    assert!(matches!(result, Err(Error::TooLarge { limit: 1_000 })), "{result:?}");
    assert!(dir.names().is_empty());

    // Not declared (the body just ends when the server closes): stopped while streaming.
    let undeclared = Server::start(|_, stream| {
        send_head(stream, "200 OK", &[]);
        let _ = stream.write_all(&body(50_000));
    });
    let result = fetch(&undeclared, &dir, "big.bin", &options);
    assert!(matches!(result, Err(Error::TooLarge { limit: 1_000 })), "{result:?}");
    assert!(dir.names().is_empty(), "{:?}", dir.names());

    // Exactly at the limit is accepted.
    let exact = serve_bytes(body(1_000));
    assert_eq!(fetch(&exact, &dir, "ok.bin", &options).unwrap().bytes, 1_000);
}

#[test]
fn only_a_plain_200_is_saved() {
    let dir = Scratch::new("status");
    let options = local_options();
    let statuses = [
        ("404 Not Found", 404),
        ("401 Unauthorized", 401),
        ("500 Internal Server Error", 500),
        ("206 Partial Content", 206),
    ];
    for (status, code) in statuses {
        let server = Server::start(move |_, stream| respond(stream, status, &[], b"hello"));
        let result = fetch(&server, &dir, "a.txt", &options);
        assert!(matches!(result, Err(Error::Status(seen)) if seen == code), "{status}: {result:?}");
    }
    assert!(dir.names().is_empty());
}

#[test]
fn a_stalled_server_ends_in_a_timeout_and_leaves_nothing() {
    let server = Server::start(|_, stream| {
        send_head(stream, "200 OK", &[("Content-Length", "100")]);
        let _ = stream.write_all(&body(10));
        let _ = stream.flush();
        wait_for_disconnect(stream, Duration::from_secs(8));
    });
    let dir = Scratch::new("timeout");
    let mut options = local_options();
    options.read_timeout = Duration::from_millis(300);
    let started = Instant::now();
    let result = fetch(&server, &dir, "slow.bin", &options);
    assert!(matches!(result, Err(Error::Timeout)), "{result:?}");
    assert!(started.elapsed() < Duration::from_secs(6), "took {:?}", started.elapsed());
    assert!(dir.names().is_empty());
}

#[test]
fn an_unreachable_https_host_fails_cleanly_without_a_panic() {
    // Port taken and released, so nothing listens there. This also builds the TLS stack.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let dir = Scratch::new("closed-port");
    let mut options = local_options();
    options.https_only = true;
    options.connect_timeout = Duration::from_secs(3);
    let url = format!("https://127.0.0.1:{port}/file");
    let result = run(download(&url, dir.path(), "a.txt", &options, Hooks::new()));
    assert!(matches!(result, Err(Error::Network | Error::Timeout)), "{result:?}");
    assert!(dir.names().is_empty());
}

#[test]
fn error_messages_never_leak_the_url() {
    let secret = "tok_3f9a1c7e5b2d4a60";
    let server = Server::start(|_, stream| respond(stream, "404 Not Found", &[], b""));
    let dir = Scratch::new("secret");
    let url = server.url(&format!("/download/{secret}"));
    let result = run(download(&url, dir.path(), "a.txt", &local_options(), Hooks::new()));
    let text = format!("{:?} / {}", result, result.as_ref().unwrap_err());
    assert!(!text.contains(secret), "{text}");
}

#[test]
fn the_download_future_can_move_between_threads() {
    fn assert_send<T: Send>(_: &T) {}
    let dir = Scratch::new("send");
    let options = local_options();
    let future = download("http://127.0.0.1:1/x", dir.path(), "a.txt", &options, Hooks::new());
    assert_send(&future);
}
