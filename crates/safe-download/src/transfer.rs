//! The transfer core: stream a byte source into a `.part` file, verify it, move it into place.
//!
//! The source of bytes and the sink file are traits so the failure paths (a stalled
//! connection, a write that is lost late) can be tested without a network or a full disk.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::task::Poll;
use std::time::{Duration, Instant};

use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::error::Error;
use crate::hooks::{CancelToken, ProgressFn};
use crate::names::{candidates, create_exclusive};
use crate::options::{Collision, Options};
use crate::Saved;

/// Appended to the final name to get the temp name, e.g. `report.pdf.part`.
const TEMP_SUFFIX: &str = ".part";

/// Why reading the next chunk failed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ChunkError {
    Timeout,
    Broken,
}

/// Where the bytes come from: an HTTP response, or a scripted source in tests.
pub(crate) trait Chunks {
    fn next_chunk(
        &mut self,
    ) -> impl Future<Output = Result<Option<impl AsRef<[u8]> + Send>, ChunkError>> + Send;
}

impl Chunks for reqwest::Response {
    // Not an `async fn`: the chunk type (`bytes::Bytes`) would have to be named, which needs
    // a direct dependency on `bytes` just for this signature.
    #[allow(clippy::manual_async_fn)]
    fn next_chunk(
        &mut self,
    ) -> impl Future<Output = Result<Option<impl AsRef<[u8]> + Send>, ChunkError>> + Send {
        async move {
            self.chunk().await.map_err(|error| {
                if error.is_timeout() {
                    ChunkError::Timeout
                } else {
                    ChunkError::Broken
                }
            })
        }
    }
}

/// The partial file: a tokio file, or in tests one whose writes fail late.
pub(crate) trait Sink: AsyncWrite + Unpin + Send {
    fn sync(&mut self) -> impl Future<Output = io::Result<()>> + Send;
    fn length(&mut self) -> impl Future<Output = io::Result<u64>> + Send;
}

impl Sink for tokio::fs::File {
    fn sync(&mut self) -> impl Future<Output = io::Result<()>> + Send {
        self.sync_all()
    }

    async fn length(&mut self) -> io::Result<u64> {
        Ok(self.metadata().await?.len())
    }
}

/// `None` as soon as `cancel` fires, even while `work` waits on a stalled connection.
pub(crate) async fn unless_cancelled<F: Future>(
    cancel: &CancelToken,
    work: F,
) -> Option<F::Output> {
    let mut work = std::pin::pin!(work);
    let mut stop = std::pin::pin!(cancel.cancelled());
    std::future::poll_fn(|cx| {
        if stop.as_mut().poll(cx).is_ready() {
            return Poll::Ready(None);
        }
        work.as_mut().poll(cx).map(Some)
    })
    .await
}

/// Throttles and de-duplicates progress callbacks so reported values only ever grow.
pub(crate) struct Reporter {
    callback: Option<ProgressFn>,
    total: Option<u64>,
    interval: Duration,
    last_at: Option<Instant>,
    last_bytes: Option<u64>,
}

impl Reporter {
    pub(crate) fn new(
        callback: Option<ProgressFn>,
        total: Option<u64>,
        interval: Duration,
    ) -> Self {
        Self { callback, total, interval, last_at: None, last_bytes: None }
    }

    fn send(&mut self, bytes: u64) {
        if self.last_bytes == Some(bytes) {
            return;
        }
        self.last_bytes = Some(bytes);
        self.last_at = Some(Instant::now());
        if let Some(callback) = self.callback.as_mut() {
            callback(bytes, self.total);
        }
    }

    /// The response has started: report zero bytes.
    pub(crate) fn start(&mut self) {
        self.send(0);
    }

    /// Reports `bytes` unless the last report is younger than the interval.
    fn tick(&mut self, bytes: u64) {
        let due = self.last_at.is_none_or(|at| at.elapsed() >= self.interval);
        if due {
            self.send(bytes);
        }
    }

    /// Always reports the final size (once).
    fn finish(&mut self, bytes: u64) {
        self.send(bytes);
    }
}

struct Limits {
    expected: Option<u64>,
    max_bytes: u64,
}

/// Reads `source` to its end into `file`, enforcing the limits. Returns the byte count.
async fn write_body<W: AsyncWrite + Unpin, S: Chunks>(
    file: &mut W,
    source: &mut S,
    cancel: &CancelToken,
    limits: &Limits,
    reporter: &mut Reporter,
) -> Result<u64, Error> {
    let mut received: u64 = 0;
    loop {
        let next = unless_cancelled(cancel, source.next_chunk()).await.ok_or(Error::Cancelled)?;
        let chunk = match next {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(ChunkError::Timeout) => return Err(Error::Timeout),
            Err(ChunkError::Broken) => {
                return Err(Error::Truncated { expected: limits.expected, received });
            }
        };
        let bytes = chunk.as_ref();
        received = received.saturating_add(bytes.len() as u64);
        if received > limits.max_bytes {
            return Err(Error::TooLarge { limit: limits.max_bytes });
        }
        // Never write more than the server declared.
        if let Some(expected) = limits.expected {
            if received > expected {
                return Err(Error::SizeMismatch { expected, received });
            }
        }
        file.write_all(bytes).await?;
        reporter.tick(received);
    }
    if let Some(expected) = limits.expected {
        if received < expected {
            return Err(Error::Truncated { expected: Some(expected), received });
        }
    }
    reporter.finish(received);
    Ok(received)
}

/// Every byte on disk, or an error. tokio's `write_all` returns once a chunk is handed to a
/// background write whose failure only comes back from the next write or flush, and
/// `sync_all` does not report it, so the length on disk is compared with the bytes received.
async fn settle<W: Sink>(file: &mut W, received: u64) -> Result<(), Error> {
    file.flush().await?;
    file.sync().await?;
    let length = file.length().await?;
    if length != received {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::WriteZero,
            format!("{length} bytes on disk but {received} received"),
        )));
    }
    Ok(())
}

/// Deletes the partial file when the transfer ends early (error, cancel, a dropped future).
///
/// `Drop` does not run when the process aborts or the machine loses power; such a leftover
/// is harmless because temp names are created exclusively and numbered.
///
/// While the blocking final step runs, the guard lives inside that step (see
/// `stream_with_plan`), so dropping the future cannot delete the file underneath it.
struct TempFile {
    path: PathBuf,
    keep: bool,
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Runs `op` up to `attempts` times (at least once), pausing in between.
fn retry(attempts: u32, pause: Duration, mut op: impl FnMut() -> io::Result<()>) -> io::Result<()> {
    let attempts = attempts.max(1);
    let mut attempt = 1;
    loop {
        match op() {
            Ok(()) => return Ok(()),
            Err(error) if attempt >= attempts => return Err(error),
            Err(_) => {
                attempt += 1;
                std::thread::sleep(pause);
            }
        }
    }
}

/// Runs `rename` up to `attempts` times (at least once), pausing in between.
pub(crate) fn rename_with_retry<F>(
    from: &Path,
    to: &Path,
    attempts: u32,
    pause: Duration,
    mut rename: F,
) -> io::Result<()>
where
    F: FnMut(&Path, &Path) -> io::Result<()>,
{
    retry(attempts, pause, || rename(from, to))
}

/// Tags the file as downloaded from the internet, as browsers do, so SmartScreen and
/// Protected View still apply. Volumes without alternate streams just skip it.
///
/// A file that is not there is left alone: writing the stream of a missing file would create
/// the file itself, empty.
#[cfg(windows)]
fn mark_downloaded(path: &Path) {
    if !path.is_file() {
        return;
    }
    let mut stream = path.as_os_str().to_owned();
    stream.push(":Zone.Identifier");
    let _ = std::fs::write(PathBuf::from(stream), "[ZoneTransfer]\r\nZoneId=3\r\n");
}

#[cfg(not(windows))]
fn mark_downloaded(_path: &Path) {}

/// An operation on two paths. Boxed so tests can hold or fail it.
type PathOp = Box<dyn Fn(&Path, &Path) -> io::Result<()> + Send>;

/// What the blocking final step needs to know.
struct Finish {
    collision: Collision,
    attempts: u32,
    pause: Duration,
    mark: bool,
    /// Gives the finished file its final name, failing if that name is taken (a hard link).
    link: PathOp,
    /// The fallback for file systems without hard links: moves the file over a reserved name.
    rename: PathOp,
}

impl Finish {
    fn new(options: &Options) -> Self {
        Self {
            collision: options.collision,
            attempts: options.rename_attempts,
            pause: options.rename_pause,
            mark: options.mark_of_the_web,
            link: Box::new(|temp, path| std::fs::hard_link(temp, path)),
            rename: Box::new(|from, to| std::fs::rename(from, to)),
        }
    }

    /// Every candidate name was taken.
    fn all_taken(&self) -> Error {
        if self.collision == Collision::Fail {
            Error::Exists
        } else {
            Error::Io(io::Error::new(io::ErrorKind::AlreadyExists, "too many files with this name"))
        }
    }
}

/// Gives the finished temp file its final name and never replaces anything. The tag is
/// written first so it stays with the file.
///
/// The name is claimed with a hard link, which succeeds only if the name is free. No
/// placeholder ever sits under the final name, so a crash cannot leave an empty file that
/// looks like a finished download. File systems without hard links (FAT, some network
/// shares) take the fallback in [`reserve_and_move`].
fn finish(dir: &Path, name: &str, temp: &Path, plan: &Finish) -> Result<PathBuf, Error> {
    if plan.mark {
        mark_downloaded(temp);
    }
    match link_into_place(dir, name, temp, plan)? {
        Some(path) => Ok(path),
        None => reserve_and_move(dir, name, temp, plan),
    }
}

/// Links `temp` to the first free candidate name, then removes the temp name.
///
/// `Ok(None)` when the file system will not link, so the caller can use the fallback.
fn link_into_place(
    dir: &Path,
    name: &str,
    temp: &Path,
    plan: &Finish,
) -> Result<Option<PathBuf>, Error> {
    for candidate in candidates(dir, name, plan.collision) {
        match (plan.link)(temp, &candidate) {
            Ok(()) => {
                // The data has its final name; the temp name is only a second name for it.
                // A virus scanner can hold it for a moment, and a leftover is harmless.
                let _ = retry(plan.attempts, plan.pause, || std::fs::remove_file(temp));
                return Ok(Some(candidate));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Ok(None),
        }
    }
    Err(plan.all_taken())
}

/// Fallback: reserves a free name with `create_new`, then moves the finished temp file over
/// the empty reservation. Until the move is done, an empty file holds the final name.
fn reserve_and_move(dir: &Path, name: &str, temp: &Path, plan: &Finish) -> Result<PathBuf, Error> {
    let (path, reserved) = create_exclusive(dir, name, plan.collision).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists && plan.collision == Collision::Fail {
            Error::Exists
        } else {
            Error::Io(error)
        }
    })?;
    // The handle must be closed before the move; Windows refuses to replace an open file.
    drop(reserved);
    // Replaces the empty file reserved above. A virus scanner can hold the fresh file for a moment.
    match rename_with_retry(temp, &path, plan.attempts, plan.pause, |from, to| {
        (plan.rename)(from, to)
    }) {
        Ok(()) => Ok(path),
        Err(error) => {
            let _ = std::fs::remove_file(&path);
            Err(Error::Io(error))
        }
    }
}

/// What to save and where.
pub(crate) struct Job<'a> {
    pub(crate) dir: &'a Path,
    /// Sanitised final name.
    pub(crate) name: &'a str,
    /// Declared `Content-Length`, if any.
    pub(crate) expected: Option<u64>,
    pub(crate) options: &'a Options,
}

/// Streams `source` into `dir/name` through a temp file; nothing is left behind unless all of
/// it arrived, was written, and was moved to a free name.
pub(crate) async fn stream_to_file<S, W>(
    job: &Job<'_>,
    source: &mut S,
    cancel: &CancelToken,
    reporter: &mut Reporter,
    open: impl FnOnce(std::fs::File) -> W,
) -> Result<Saved, Error>
where
    S: Chunks,
    W: Sink,
{
    let plan = Finish::new(job.options);
    stream_with_plan(job, source, cancel, reporter, open, plan).await
}

/// [`stream_to_file`] with the final step spelled out, so tests can hold or fail it.
async fn stream_with_plan<S, W>(
    job: &Job<'_>,
    source: &mut S,
    cancel: &CancelToken,
    reporter: &mut Reporter,
    open: impl FnOnce(std::fs::File) -> W,
    plan: Finish,
) -> Result<Saved, Error>
where
    S: Chunks,
    W: Sink,
{
    // A numbered temp name, so a file left by a crash never blocks the next try.
    let temp_name = format!("{}{TEMP_SUFFIX}", job.name);
    let (temp_path, file) = create_exclusive(job.dir, &temp_name, Collision::Number)?;
    let temp = TempFile { path: temp_path, keep: false };
    let mut file = open(file);
    let limits = Limits { expected: job.expected, max_bytes: job.options.max_bytes };
    // Both arms wait out a write still in flight, so the handle is closed before the temp
    // file is moved or removed.
    let written = match write_body(&mut file, source, cancel, &limits, reporter).await {
        Ok(received) => settle(&mut file, received).await.map(|()| received),
        Err(error) => {
            let _ = file.flush().await;
            Err(error)
        }
    };
    drop(file);
    let received = written?;
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let (dir, name) = (job.dir.to_owned(), job.name.to_owned());
    // The guard goes into the blocking step. If the future is dropped meanwhile, the step
    // still runs to its end, and a guard left outside would delete the temp file underneath
    // it (on Windows, tagging the vanished file would even re-create it empty). The result
    // is then a whole file under its final name, or nothing, never an empty or partial one.
    let path = tokio::task::spawn_blocking(move || {
        let mut temp = temp;
        let path = finish(&dir, &name, &temp.path, &plan)?;
        // Moved over, or linked and unlinked: the temp name is gone, and no longer ours to
        // delete. The file is the caller's now, even if a cancel arrives late.
        temp.keep = true;
        Ok::<_, Error>(path)
    })
    .await
    .map_err(|_| Error::Io(io::Error::other("the rename task did not finish")))??;
    Ok(Saved { path, bytes: received })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    /// A unique scratch directory that is removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let id = NEXT.fetch_add(1, Ordering::SeqCst);
            let dir = std::env::temp_dir()
                .join(format!("safe-download-core-{label}-{}-{id}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn names(&self) -> Vec<String> {
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

    fn block_on<F: Future>(future: F) -> F::Output {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(30), future).await.expect("test timed out")
        })
    }

    enum Step {
        Data(&'static [u8]),
        Fail,
        TimedOut,
        Stall,
    }

    struct Script(VecDeque<Step>);

    fn script(steps: Vec<Step>) -> Script {
        Script(steps.into())
    }

    impl Chunks for Script {
        fn next_chunk(
            &mut self,
        ) -> impl Future<Output = Result<Option<impl AsRef<[u8]> + Send>, ChunkError>> + Send
        {
            let step = self.0.pop_front();
            async move {
                match step {
                    None => Ok(None),
                    Some(Step::Data(bytes)) => Ok(Some(bytes)),
                    Some(Step::Fail) => Err(ChunkError::Broken),
                    Some(Step::TimedOut) => Err(ChunkError::Timeout),
                    Some(Step::Stall) => std::future::pending().await,
                }
            }
        }
    }

    fn save(
        dir: &Scratch,
        name: &str,
        expected: Option<u64>,
        options: &Options,
        steps: Vec<Step>,
    ) -> Result<Saved, Error> {
        let job = Job { dir: &dir.0, name, expected, options };
        let mut reporter = Reporter::new(None, expected, Duration::ZERO);
        let cancel = CancelToken::new();
        block_on(stream_to_file(
            &job,
            &mut script(steps),
            &cancel,
            &mut reporter,
            tokio::fs::File::from_std,
        ))
    }

    fn plain() -> Options {
        Options { mark_of_the_web: false, ..Options::default() }
    }

    #[test]
    fn a_complete_body_becomes_the_file_and_no_temp_file_is_left() {
        let dir = Scratch::new("complete");
        let saved = save(
            &dir,
            "a.txt",
            Some(11),
            &plain(),
            vec![Step::Data(b"hello "), Step::Data(b"world")],
        )
        .unwrap();
        assert_eq!(saved.path, dir.0.join("a.txt"));
        assert_eq!(saved.bytes, 11);
        assert_eq!(std::fs::read(&saved.path).unwrap(), b"hello world");
        assert_eq!(dir.names(), ["a.txt"]);
    }

    #[test]
    fn a_taken_name_is_numbered_and_the_old_file_kept() {
        let dir = Scratch::new("numbered");
        save(&dir, "a.txt", Some(3), &plain(), vec![Step::Data(b"old")]).unwrap();
        let again = save(&dir, "a.txt", Some(3), &plain(), vec![Step::Data(b"new")]).unwrap();
        assert_eq!(again.path.file_name().unwrap(), "a (1).txt");
        assert_eq!(std::fs::read(dir.0.join("a.txt")).unwrap(), b"old");
        assert_eq!(std::fs::read(&again.path).unwrap(), b"new");
        assert_eq!(dir.names(), ["a (1).txt", "a.txt"]);
    }

    #[test]
    fn the_fail_policy_keeps_the_old_file_and_leaves_no_temp() {
        let dir = Scratch::new("fail-policy");
        let options = Options { collision: Collision::Fail, ..plain() };
        save(&dir, "a.txt", Some(3), &options, vec![Step::Data(b"old")]).unwrap();
        let result = save(&dir, "a.txt", Some(3), &options, vec![Step::Data(b"new")]);
        assert!(matches!(result, Err(Error::Exists)), "{result:?}");
        assert_eq!(std::fs::read(dir.0.join("a.txt")).unwrap(), b"old");
        assert_eq!(dir.names(), ["a.txt"]);
    }

    #[test]
    fn an_empty_body_makes_an_empty_file() {
        let dir = Scratch::new("empty");
        let saved = save(&dir, "empty.txt", Some(0), &plain(), vec![]).unwrap();
        assert_eq!(saved.bytes, 0);
        assert_eq!(std::fs::read(&saved.path).unwrap(), b"");
    }

    #[test]
    fn more_bytes_than_declared_stop_the_transfer_and_leave_nothing() {
        let dir = Scratch::new("longer");
        let result = save(
            &dir,
            "a.txt",
            Some(4),
            &plain(),
            vec![Step::Data(b"abc"), Step::Data(b"de"), Step::Stall],
        );
        assert!(
            matches!(result, Err(Error::SizeMismatch { expected: 4, received: 5 })),
            "{result:?}"
        );
        assert!(dir.names().is_empty());
    }

    #[test]
    fn the_byte_limit_applies_when_no_length_was_declared() {
        let dir = Scratch::new("limit");
        let options = Options { max_bytes: 5, ..plain() };
        let steps = vec![Step::Data(b"abc"), Step::Data(b"def"), Step::Stall];
        let result = save(&dir, "a.txt", None, &options, steps);
        assert!(matches!(result, Err(Error::TooLarge { limit: 5 })), "{result:?}");
        assert!(dir.names().is_empty());
        // Exactly at the limit is fine.
        let ok = save(&dir, "b.txt", None, &options, vec![Step::Data(b"abcde")]).unwrap();
        assert_eq!(ok.bytes, 5);
    }

    #[test]
    fn a_short_or_broken_body_leaves_nothing() {
        let dir = Scratch::new("short");
        let options = plain();
        let data = || Step::Data(b"abc");
        let short = save(&dir, "a.txt", Some(10), &options, vec![data()]);
        assert!(matches!(short, Err(Error::Truncated { expected: Some(10), received: 3 })));
        let broken = save(&dir, "a.txt", Some(10), &options, vec![data(), Step::Fail]);
        assert!(matches!(broken, Err(Error::Truncated { expected: Some(10), received: 3 })));
        let unknown = save(&dir, "a.txt", None, &options, vec![data(), Step::Fail]);
        assert!(matches!(unknown, Err(Error::Truncated { expected: None, received: 3 })));
        let silent = save(&dir, "a.txt", Some(10), &options, vec![data(), Step::TimedOut]);
        assert!(matches!(silent, Err(Error::Timeout)), "{silent:?}");
        assert!(dir.names().is_empty());
    }

    #[test]
    fn cancelling_a_stalled_transfer_stops_it_at_once_and_leaves_nothing() {
        let dir = Scratch::new("cancel");
        let cancel = CancelToken::new();
        let canceller = {
            let cancel = cancel.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                cancel.cancel();
            })
        };
        let options = plain();
        let job = Job { dir: &dir.0, name: "a.txt", expected: Some(10), options: &options };
        let mut reporter = Reporter::new(None, Some(10), Duration::ZERO);
        let started = Instant::now();
        let result = block_on(stream_to_file(
            &job,
            &mut script(vec![Step::Data(b"abc"), Step::Stall]),
            &cancel,
            &mut reporter,
            tokio::fs::File::from_std,
        ));
        canceller.join().unwrap();
        assert!(matches!(result, Err(Error::Cancelled)), "{result:?}");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(dir.names().is_empty());
    }

    #[test]
    fn a_token_that_fired_before_the_start_saves_nothing() {
        let dir = Scratch::new("pre-cancel");
        let cancel = CancelToken::new();
        cancel.cancel();
        let options = plain();
        let job = Job { dir: &dir.0, name: "a.txt", expected: Some(3), options: &options };
        let mut reporter = Reporter::new(None, Some(3), Duration::ZERO);
        let result = block_on(stream_to_file(
            &job,
            &mut script(vec![Step::Data(b"abc")]),
            &cancel,
            &mut reporter,
            tokio::fs::File::from_std,
        ));
        assert!(matches!(result, Err(Error::Cancelled)), "{result:?}");
        assert!(dir.names().is_empty());
    }

    #[test]
    fn dropping_the_transfer_removes_the_partial_file() {
        let dir = Scratch::new("dropped");
        let options = plain();
        let job = Job { dir: &dir.0, name: "a.txt", expected: Some(10), options: &options };
        let mut reporter = Reporter::new(None, Some(10), Duration::ZERO);
        let cancel = CancelToken::new();
        let mut source = script(vec![Step::Data(b"abc"), Step::Stall]);
        block_on(async {
            {
                let transfer = stream_to_file(
                    &job,
                    &mut source,
                    &cancel,
                    &mut reporter,
                    tokio::fs::File::from_std,
                );
                let mut transfer = std::pin::pin!(transfer);
                // One poll: the first chunk is written, then it waits on the stalled source.
                std::future::poll_fn(|cx: &mut Context<'_>| {
                    assert!(transfer.as_mut().poll(cx).is_pending());
                    Poll::Ready(())
                })
                .await;
                assert_eq!(dir.names(), ["a.txt.part"]);
            }
            // The future is gone: the guard removed the partial file.
            assert!(dir.names().is_empty());
        });
    }

    #[test]
    fn a_temp_file_left_by_a_crash_does_not_block_the_next_try() {
        let dir = Scratch::new("leftover");
        std::fs::write(dir.0.join("a.txt.part"), b"partial").unwrap();
        let saved = save(&dir, "a.txt", Some(3), &plain(), vec![Step::Data(b"abc")]).unwrap();
        assert_eq!(std::fs::read(&saved.path).unwrap(), b"abc");
        assert_eq!(dir.names(), ["a.txt", "a.txt.part"]);
        assert_eq!(std::fs::read(dir.0.join("a.txt.part")).unwrap(), b"partial");
    }

    /// Takes writes like tokio's file but loses everything past `room` bytes, as a background
    /// write that failed would. With `report`, flush returns that failure, as tokio's does.
    struct LostWrite {
        file: tokio::fs::File,
        room: usize,
        report: bool,
        lost: bool,
    }

    impl AsyncWrite for LostWrite {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            if self.lost || buf.len() > self.room {
                self.lost = true;
                return Poll::Ready(Ok(buf.len()));
            }
            let written = std::task::ready!(Pin::new(&mut self.file).poll_write(cx, buf))?;
            self.room -= written;
            Poll::Ready(Ok(written))
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            if self.lost && self.report {
                return Poll::Ready(Err(io::Error::other("disk full")));
            }
            Pin::new(&mut self.file).poll_flush(cx)
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.file).poll_shutdown(cx)
        }
    }

    impl Sink for LostWrite {
        fn sync(&mut self) -> impl Future<Output = io::Result<()>> + Send {
            self.file.sync()
        }

        fn length(&mut self) -> impl Future<Output = io::Result<u64>> + Send {
            self.file.length()
        }
    }

    fn save_losing(
        dir: &Scratch,
        size: u64,
        steps: Vec<Step>,
        room: usize,
        report: bool,
    ) -> Result<Saved, Error> {
        let options = plain();
        let job = Job { dir: &dir.0, name: "a.txt", expected: Some(size), options: &options };
        let mut reporter = Reporter::new(None, Some(size), Duration::ZERO);
        let cancel = CancelToken::new();
        let open = |file| {
            let file = tokio::fs::File::from_std(file);
            LostWrite { file, room, report, lost: false }
        };
        block_on(stream_to_file(&job, &mut script(steps), &cancel, &mut reporter, open))
    }

    fn hello_world() -> Vec<Step> {
        vec![Step::Data(b"hello "), Step::Data(b"world")]
    }

    #[test]
    fn a_last_write_that_fails_late_saves_nothing() {
        let dir = Scratch::new("late-write");
        // The whole body fits in one chunk, so only flush can report the failed write.
        let one = save_losing(&dir, 5, vec![Step::Data(b"hello")], 0, true);
        assert!(matches!(one, Err(Error::Io(_))), "{one:?}");
        let two = save_losing(&dir, 11, hello_world(), 6, true);
        assert!(matches!(two, Err(Error::Io(_))), "{two:?}");
        assert!(dir.names().is_empty());
    }

    #[test]
    fn a_file_shorter_on_disk_than_received_saves_nothing() {
        let dir = Scratch::new("short-disk");
        let lost = save_losing(&dir, 11, hello_world(), 6, false);
        match lost {
            Err(Error::Io(error)) => assert_eq!(error.kind(), io::ErrorKind::WriteZero),
            other => panic!("expected a length mismatch, got {other:?}"),
        }
        assert!(dir.names().is_empty());
        // Nothing lost: the same writer saves the whole file.
        let saved = save_losing(&dir, 11, hello_world(), 11, true).unwrap();
        assert_eq!(std::fs::read(&saved.path).unwrap(), b"hello world");
    }

    #[test]
    fn renames_are_retried_a_bounded_number_of_times() {
        let (from, to) = (Path::new("from"), Path::new("to"));
        let calls = AtomicU32::new(0);
        let flaky = |_: &Path, _: &Path| {
            if calls.fetch_add(1, Ordering::SeqCst) < 2 {
                Err(io::Error::other("held by a scanner"))
            } else {
                Ok(())
            }
        };
        assert!(rename_with_retry(from, to, 5, Duration::ZERO, flaky).is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 3);

        let calls = AtomicU32::new(0);
        let never = |_: &Path, _: &Path| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(io::Error::other("still held"))
        };
        assert!(rename_with_retry(from, to, 4, Duration::ZERO, never).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 4);

        // Zero attempts still means one try.
        let calls = AtomicU32::new(0);
        let once = |_: &Path, _: &Path| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(io::Error::other("nope"))
        };
        assert!(rename_with_retry(from, to, 0, Duration::ZERO, once).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_rename_that_keeps_failing_leaves_neither_temp_nor_reservation() {
        let dir = Scratch::new("rename-fails");
        let temp = dir.0.join("missing.part");
        let options = Options {
            rename_attempts: 2,
            rename_pause: Duration::ZERO,
            mark_of_the_web: false,
            ..Options::default()
        };
        // The temp file does not exist, so linking fails and then every rename fails.
        let result = finish(&dir.0, "a.txt", &temp, &Finish::new(&options));
        assert!(matches!(result, Err(Error::Io(_))), "{result:?}");
        assert!(dir.names().is_empty());
    }

    /// Saves "hello" as `a.txt` with the given final step.
    fn save_with(dir: &Scratch, plan: Finish) -> Result<Saved, Error> {
        let options = plain();
        let job = Job { dir: &dir.0, name: "a.txt", expected: Some(5), options: &options };
        let mut reporter = Reporter::new(None, Some(5), Duration::ZERO);
        block_on(stream_with_plan(
            &job,
            &mut script(vec![Step::Data(b"hello")]),
            &CancelToken::new(),
            &mut reporter,
            tokio::fs::File::from_std,
            plan,
        ))
    }

    #[test]
    fn a_file_system_without_hard_links_falls_back_to_the_reservation() {
        let dir = Scratch::new("no-links");
        let no_links = || {
            let mut plan = Finish::new(&plain());
            plan.link = Box::new(|_, _| Err(io::Error::from(io::ErrorKind::Unsupported)));
            plan
        };
        let first = save_with(&dir, no_links()).unwrap();
        assert_eq!(first.path, dir.0.join("a.txt"));
        // The fallback never replaces a file either.
        let second = save_with(&dir, no_links()).unwrap();
        assert_eq!(second.path, dir.0.join("a (1).txt"));
        assert_eq!(dir.names(), ["a (1).txt", "a.txt"]);
        assert_eq!(std::fs::read(&first.path).unwrap(), b"hello");
        assert_eq!(std::fs::read(&second.path).unwrap(), b"hello");
    }

    #[test]
    fn no_placeholder_holds_the_final_name_before_the_file_is_complete() {
        let dir = Scratch::new("no-placeholder");
        let final_name = dir.0.join("a.txt");
        let mut plan = Finish::new(&plain());
        let seen = Arc::new(Mutex::new(Vec::new()));
        plan.link = {
            let seen = seen.clone();
            Box::new(move |temp, path| {
                // The instant before the name is claimed: a crash here must leave no `a.txt`.
                let before = path.exists();
                seen.lock().unwrap().push(before);
                std::fs::hard_link(temp, path)
            })
        };
        save_with(&dir, plan).unwrap();
        assert_eq!(*seen.lock().unwrap(), [false]);
        assert_eq!(std::fs::read(final_name).unwrap(), b"hello");
        assert_eq!(dir.names(), ["a.txt"]);
    }

    #[test]
    fn dropping_the_transfer_during_the_final_step_leaves_a_whole_file_or_nothing() {
        let dir = Scratch::new("dropped-late");
        let options = plain();
        let job = Job { dir: &dir.0, name: "a.txt", expected: Some(5), options: &options };
        let mut reporter = Reporter::new(None, Some(5), Duration::ZERO);
        let cancel = CancelToken::new();
        let mut source = script(vec![Step::Data(b"hello")]);

        // Hold the final step open until the test lets it go.
        let reached = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (release, released) = std::sync::mpsc::channel::<()>();
        let mut plan = Finish::new(&options);
        plan.link = {
            let reached = reached.clone();
            Box::new(move |temp, path| {
                reached.store(true, Ordering::SeqCst);
                released.recv_timeout(Duration::from_secs(10)).expect("the test never released");
                std::fs::hard_link(temp, path)
            })
        };

        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            {
                let transfer = stream_with_plan(
                    &job,
                    &mut source,
                    &cancel,
                    &mut reporter,
                    tokio::fs::File::from_std,
                    plan,
                );
                let mut transfer = std::pin::pin!(transfer);
                let started = Instant::now();
                std::future::poll_fn(|cx: &mut Context<'_>| {
                    assert!(transfer.as_mut().poll(cx).is_pending(), "finished while held");
                    assert!(started.elapsed() < Duration::from_secs(10), "never reached the step");
                    if reached.load(Ordering::SeqCst) {
                        return Poll::Ready(());
                    }
                    // The blocking step cannot wake this task, so keep polling until it starts.
                    cx.waker().wake_by_ref();
                    Poll::Pending
                })
                .await;
                // Inside the final step: complete under the temp name, nothing under the real one.
                assert_eq!(dir.names(), ["a.txt.part"]);
            }
            // The future is gone. The step goes on without it.
            release.send(()).unwrap();
        });
        // Dropping the runtime waits for the blocking step to end.
        drop(runtime);
        assert_eq!(dir.names(), ["a.txt"]);
        assert_eq!(std::fs::read(dir.0.join("a.txt")).unwrap(), b"hello");
    }

    type Log = Arc<Mutex<Vec<(u64, Option<u64>)>>>;

    fn collect() -> (Log, ProgressFn) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let sink = log.clone();
        (log, Box::new(move |bytes, total| sink.lock().unwrap().push((bytes, total))))
    }

    #[test]
    fn progress_starts_at_zero_ends_at_the_size_and_never_repeats() {
        let (log, callback) = collect();
        let mut reporter = Reporter::new(Some(callback), Some(9), Duration::ZERO);
        reporter.start();
        reporter.tick(3);
        reporter.tick(3);
        reporter.tick(6);
        reporter.finish(9);
        reporter.finish(9);
        let log = log.lock().unwrap();
        assert_eq!(*log, [(0, Some(9)), (3, Some(9)), (6, Some(9)), (9, Some(9))]);
    }

    #[test]
    fn progress_is_throttled_but_the_last_value_always_arrives() {
        let (log, callback) = collect();
        let mut reporter = Reporter::new(Some(callback), None, Duration::from_secs(3600));
        reporter.start();
        for bytes in 1..=50 {
            reporter.tick(bytes);
        }
        reporter.finish(50);
        assert_eq!(*log.lock().unwrap(), [(0, None), (50, None)]);
    }

    #[test]
    fn an_empty_body_reports_zero_once() {
        let (log, callback) = collect();
        let mut reporter = Reporter::new(Some(callback), Some(0), Duration::ZERO);
        reporter.start();
        reporter.finish(0);
        assert_eq!(*log.lock().unwrap(), [(0, Some(0))]);
    }

    #[cfg(windows)]
    fn zone_of(path: &Path) -> Option<String> {
        let mut stream = path.as_os_str().to_owned();
        stream.push(":Zone.Identifier");
        std::fs::read_to_string(PathBuf::from(stream)).ok()
    }

    #[cfg(windows)]
    #[test]
    fn tagging_a_missing_file_does_not_create_it() {
        let dir = Scratch::new("zone-missing");
        mark_downloaded(&dir.0.join("gone.part"));
        assert!(dir.names().is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn saved_files_are_marked_as_downloaded_when_asked() {
        let dir = Scratch::new("zone");
        // Probe: some volumes have no alternate streams, and then there is nothing to check.
        let probe = dir.0.join("probe");
        std::fs::write(&probe, b"x").unwrap();
        mark_downloaded(&probe);
        if zone_of(&probe).is_none() {
            return;
        }
        let marked = Options { mark_of_the_web: true, ..Options::default() };
        let saved = save(&dir, "doc.pdf", Some(3), &marked, vec![Step::Data(b"pdf")]).unwrap();
        assert!(zone_of(&saved.path).unwrap().contains("ZoneId=3"));
        let steps = vec![Step::Data(b"pdf")];
        let unmarked = save(&dir, "plain.pdf", Some(3), &plain(), steps).unwrap();
        assert!(zone_of(&unmarked.path).is_none());
    }
}
