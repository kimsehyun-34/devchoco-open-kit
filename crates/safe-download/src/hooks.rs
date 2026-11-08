//! Caller-supplied hooks: a progress callback and a cancellation token.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

/// A cloneable switch that stops a download from any thread or task.
///
/// Cancelling is sticky and idempotent. A download that sees the token fire stops reading,
/// closes the connection and removes its partial file before returning
/// [`Error::Cancelled`](crate::Error::Cancelled).
#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    inner: Arc<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    cancelled: AtomicBool,
    wake: Notify,
}

impl CancelToken {
    /// A token that has not fired.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fires the token and wakes every download waiting on it.
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        self.inner.wake.notify_waiters();
    }

    /// Whether [`cancel`](Self::cancel) was called on this token or any clone.
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Completes once the token has fired, even if it fired before the call.
    pub async fn cancelled(&self) {
        loop {
            let mut notified = std::pin::pin!(self.inner.wake.notified());
            // Registered before the flag check, so a cancel in between still wakes it.
            notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

/// Progress callback: bytes received so far and the total when the server declared one.
pub(crate) type ProgressFn = Box<dyn FnMut(u64, Option<u64>) + Send>;

/// Optional callbacks handed to [`download`](crate::download).
#[derive(Default)]
pub struct Hooks {
    pub(crate) progress: Option<ProgressFn>,
    pub(crate) cancel: Option<CancelToken>,
}

impl Hooks {
    /// No callbacks and no cancellation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reports `(bytes_received, total)` while the body arrives.
    ///
    /// Called once with `0` when the response starts, then at most once per
    /// `progress_interval`, and a last time with the final size. Values never decrease.
    /// The callback runs on the downloading task, so it should return quickly.
    pub fn on_progress(mut self, callback: impl FnMut(u64, Option<u64>) + Send + 'static) -> Self {
        self.progress = Some(Box::new(callback));
        self
    }

    /// Stops the download when `token` fires.
    pub fn with_cancel(mut self, token: CancelToken) -> Self {
        self.cancel = Some(token);
        self
    }
}

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hooks")
            .field("progress", &self.progress.is_some())
            .field("cancel", &self.cancel)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::Poll;

    fn poll_once<F: Future>(future: F) -> Poll<F::Output> {
        let mut future = std::pin::pin!(future);
        let waker = std::task::Waker::noop();
        future.as_mut().poll(&mut std::task::Context::from_waker(waker))
    }

    #[test]
    fn cancelling_is_sticky_and_shared_between_clones() {
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!token.is_cancelled());
        assert!(poll_once(token.cancelled()).is_pending());
        clone.cancel();
        clone.cancel();
        assert!(token.is_cancelled());
        assert!(poll_once(token.cancelled()).is_ready());
    }

    #[test]
    fn hooks_debug_does_not_print_the_callback() {
        let hooks = Hooks::new().on_progress(|_, _| {}).with_cancel(CancelToken::new());
        let text = format!("{hooks:?}");
        assert!(text.contains("progress: true"), "{text}");
    }
}
