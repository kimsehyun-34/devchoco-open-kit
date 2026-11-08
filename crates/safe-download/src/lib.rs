//! Download a URL into a file without ever leaving a half-written or overwritten file behind.
//!
//! [`download`] streams the response body into a `.part` file next to the destination,
//! checks that everything arrived and reached the disk, and only then moves it to its final
//! name. Whatever goes wrong - a bad URL, a blocked extension, a cancel, a dropped
//! connection, a full disk - the destination directory ends up exactly as it was. The name is
//! claimed with a hard link, which only succeeds when it is free, so no empty placeholder ever
//! holds it. The README lists the remaining exceptions: a killed process leaves its `.part`
//! file, a drop during the final move still completes it, and file systems without hard links
//! use a short-lived empty reservation.
//!
//! ```no_run
//! use std::path::Path;
//! use safe_download::{download, CancelToken, Hooks, Options};
//!
//! # async fn demo() -> Result<(), safe_download::Error> {
//! // Only this host may be contacted, over HTTPS, without following redirects.
//! let options = Options::for_hosts(["downloads.example.org"]);
//! let cancel = CancelToken::new(); // hand a clone to a "Cancel" button
//! let hooks = Hooks::new()
//!     .on_progress(|bytes, total| println!("{bytes} of {total:?} bytes"))
//!     .with_cancel(cancel.clone());
//!
//! let saved = download(
//!     "https://downloads.example.org/files/report.pdf",
//!     Path::new("."),
//!     "report.pdf",
//!     &options,
//!     hooks,
//! )
//! .await;
//! match saved {
//!     Ok(saved) => println!("saved {} ({} bytes)", saved.path.display(), saved.bytes),
//!     // A program or script: let the caller hand it to a browser or reject it.
//!     Err(safe_download::Error::Blocked(extension)) => println!("blocked .{extension}"),
//!     Err(other) => return Err(other),
//! }
//! # Ok(()) }
//! ```
//!
//! The crate needs a Tokio runtime, like `reqwest`. It never reads `Content-Disposition`: the
//! file name is whatever the caller passes, sanitised by [`sanitize_file_name`].

mod error;
mod hooks;
mod names;
mod options;
mod rules;
mod transfer;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Once;

pub use error::Error;
pub use hooks::{CancelToken, Hooks};
pub use names::{sanitize_file_name, MAX_NAME_BYTES};
pub use options::{Collision, Options, RedirectPolicy, DEFAULT_BLOCKED_EXTENSIONS};

use names::blocked_extension;
use rules::UrlRules;
use transfer::{stream_to_file, unless_cancelled, Job, Reporter};

/// A finished download.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    /// Where the file ended up. Differs from the requested name when it was sanitised or
    /// numbered (`name (1).ext`).
    pub path: PathBuf,
    /// Size of the file in bytes.
    pub bytes: u64,
}

/// Downloads `url` into `dest_dir` as `preferred_name` and returns the path actually used.
///
/// Everything that can be decided without the network is decided first, in this order: the
/// URL (scheme, credentials, host allow-list), the sanitised name, the blocked extensions, the
/// destination directory and the collision policy. Dropping the returned future, firing the
/// [`CancelToken`] in `hooks` or any error removes the partial file. The final move, a few
/// milliseconds long, cannot be undone by a drop: it completes, and the whole file is kept.
///
/// An existing file is never overwritten; see [`Collision`].
pub async fn download(
    url: &str,
    dest_dir: &Path,
    preferred_name: &str,
    options: &Options,
    hooks: Hooks,
) -> Result<Saved, Error> {
    let rules = UrlRules::new(options);
    let url = rules.parse(url)?;
    let name = sanitize_file_name(preferred_name).ok_or(Error::InvalidName)?;
    if let Some(extension) = blocked_extension(&name, &options.blocked_extensions) {
        return Err(Error::Blocked(extension));
    }
    if !dest_dir.is_dir() {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "the destination directory does not exist",
        )));
    }
    let taken = std::fs::symlink_metadata(dest_dir.join(&name)).is_ok();
    if options.collision == Collision::Fail && taken {
        return Err(Error::Exists);
    }
    let Hooks { progress, cancel } = hooks;
    let cancel = cancel.unwrap_or_default();
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }

    let client = build_client(options, rules)?;
    let sent = unless_cancelled(&cancel, client.get(url).send()).await.ok_or(Error::Cancelled)?;
    let mut response =
        sent.map_err(|error| if error.is_timeout() { Error::Timeout } else { Error::Network })?;

    let status = response.status();
    if status.is_redirection() {
        return Err(Error::Redirected(status.as_u16()));
    }
    if status != reqwest::StatusCode::OK {
        return Err(Error::Status(status.as_u16()));
    }
    let expected = response.content_length();
    if expected.is_some_and(|length| length > options.max_bytes) {
        return Err(Error::TooLarge { limit: options.max_bytes });
    }

    let mut reporter = Reporter::new(progress, expected, options.progress_interval);
    reporter.start();
    let job = Job { dir: dest_dir, name: &name, expected, options };
    stream_to_file(&job, &mut response, &cancel, &mut reporter, tokio::fs::File::from_std).await
}

fn build_client(options: &Options, rules: UrlRules) -> Result<reqwest::Client, Error> {
    install_crypto_provider();
    let redirects = match options.redirects {
        RedirectPolicy::None => reqwest::redirect::Policy::none(),
        RedirectPolicy::Limited(max) => reqwest::redirect::Policy::custom(move |attempt| {
            // A hop that leaves the rules is not followed; its 3xx response is reported.
            // `previous` includes the original request, so it exceeds `max` on the hop
            // after the last allowed one.
            if attempt.previous().len() > max || rules.check(attempt.url()).is_err() {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }),
    };
    reqwest::Client::builder()
        .connect_timeout(options.connect_timeout)
        .read_timeout(options.read_timeout)
        .redirect(redirects)
        .user_agent(options.user_agent.as_str())
        .build()
        .map_err(|_| Error::Network)
}

/// reqwest is built without a crypto provider. Installs `ring` as the process default unless
/// the application already chose one.
fn install_crypto_provider() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
    });
}
