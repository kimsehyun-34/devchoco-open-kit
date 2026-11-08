//! Test support: a tiny HTTP/1.1 server on `std::net`, scratch directories and a runtime helper.
#![allow(dead_code)]

use std::future::Future;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use safe_download::Options;

/// The request line of what a client sent. Headers are read and ignored.
pub struct Request {
    pub method: String,
    pub path: String,
}

/// A server that runs `handler` on its own thread for every request it receives.
///
/// The handler owns the connection: it writes whatever bytes it likes (including broken
/// ones) and may keep the socket open to stall the client. Dropping the server stops
/// accepting; handler threads end when their client goes away or after the read timeout.
pub struct Server {
    port: u16,
    hits: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

impl Server {
    pub fn start(handler: impl Fn(&Request, &mut TcpStream) + Send + Sync + 'static) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(handler);
        let accept = {
            let (hits, stop) = (hits.clone(), stop.clone());
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(mut stream) = stream else { continue };
                    let (handler, hits) = (handler.clone(), hits.clone());
                    std::thread::spawn(move || {
                        let _ = stream.set_nodelay(true);
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                        let Some(request) = read_request(&mut stream) else { return };
                        hits.fetch_add(1, Ordering::SeqCst);
                        handler(&request, &mut stream);
                    });
                }
            })
        };
        Server { port, hits, stop, accept: Some(accept) }
    }

    /// `http://127.0.0.1:<port><path>`
    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    /// Requests received so far.
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocked accept so the thread can see the flag.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(accept) = self.accept.take() {
            let _ = accept.join();
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut seen = Vec::new();
    let mut buffer = [0u8; 1024];
    while !seen.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).ok()?;
        if read == 0 {
            return None;
        }
        seen.extend_from_slice(&buffer[..read]);
    }
    let head = String::from_utf8_lossy(&seen).into_owned();
    let mut line = head.lines().next()?.split_whitespace();
    Some(Request { method: line.next()?.to_owned(), path: line.next()?.to_owned() })
}

/// Sends a complete response: `Content-Length` and `Connection: close` are added.
pub fn respond(stream: &mut TcpStream, status: &str, headers: &[(&str, &str)], body: &[u8]) {
    let mut head = format!("HTTP/1.1 {status}\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", body.len()));
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

/// Sends only a status line and the given headers (plus `Connection: close`), then the
/// blank line. The caller writes the body, as little or as broken as the test needs.
pub fn send_head(stream: &mut TcpStream, status: &str, headers: &[(&str, &str)]) {
    let mut head = format!("HTTP/1.1 {status}\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("Connection: close\r\n\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.flush();
}

/// Waits until the client closes the connection. False if it is still open after `limit`.
pub fn wait_for_disconnect(stream: &mut TcpStream, limit: Duration) -> bool {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
    let deadline = Instant::now() + limit;
    let mut buffer = [0u8; 256];
    while Instant::now() < deadline {
        match stream.read(&mut buffer) {
            Ok(0) => return true,
            Ok(_) => {}
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            // A reset also means the client is gone.
            Err(_) => return true,
        }
    }
    false
}

/// A unique directory under the system temp dir, removed on drop.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(label: &str) -> Scratch {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let id = NEXT.fetch_add(1, Ordering::SeqCst);
        let name = format!("safe-download-it-{label}-{}-{id}", std::process::id());
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Sorted names of everything directly inside.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Options for the loopback server: plain HTTP to `127.0.0.1`, every progress step reported.
pub fn local_options() -> Options {
    let mut options = Options::for_hosts(["127.0.0.1"]);
    options.https_only = false;
    options.progress_interval = Duration::ZERO;
    options.read_timeout = Duration::from_secs(10);
    options.rename_pause = Duration::from_millis(20);
    options
}

/// Runs a future to completion on a fresh runtime, failing the test if it hangs.
pub fn run<F: Future>(future: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(60), future).await.expect("test timed out")
    })
}
