use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::solver::ffi::Stopper;

/// Stops solves from any thread, including from inside a callback.
///
/// Like a cancellation token, one handle stops every solve it is passed to,
/// including solves running at the same time. A stop is sticky: stopping
/// before a solve starts makes it return at once, and a stopped handle stops
/// every later solve it is passed to. Clones share one state.
#[derive(Clone, Debug, Default)]
pub struct StopHandle(Arc<Shared>);

#[derive(Debug, Default)]
struct Shared {
    stopped: AtomicBool,
    sessions: Mutex<Vec<Stopper>>,
}

impl StopHandle {
    /// A handle that has not stopped.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stops every attached solve and every later one.
    #[doc(alias("stop_search", "StopSearch"))]
    pub fn stop(&self) {
        // Set the flag before locking, so `attach` either sees it or registers
        // its session before this thread looks.
        self.0.stopped.store(true, Ordering::SeqCst);
        for session in self.lock().iter() {
            session.stop();
        }
    }

    /// Whether [`StopHandle::stop`] has been called.
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.0.stopped.load(Ordering::SeqCst)
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Stopper>> {
        // Nothing panics while holding the lock, so a poisoned state is intact.
        self.0
            .sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Lets `handle` stop the session behind `session` until the session drops.
///
/// This is a free function rather than a method so it stays out of the public
/// API. It also drops the stoppers of sessions that are gone, so a handle
/// that serves many solves holds only the live ones.
pub fn attach(handle: &StopHandle, session: Stopper) {
    let mut sessions = handle.lock();
    sessions.retain(Stopper::is_live);
    if handle.is_stopped() {
        session.stop();
    }
    sessions.push(session);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::ffi::Session;

    /// The number of sessions attached to `handle`.
    fn attached_sessions(handle: &StopHandle) -> usize {
        handle.lock().len()
    }

    #[test]
    fn attach_drops_the_stoppers_of_dropped_sessions() {
        let handle = StopHandle::new();
        let session = Session::new().unwrap();
        attach(&handle, session.stopper());
        drop(session);
        let session = Session::new().unwrap();
        attach(&handle, session.stopper());
        assert_eq!(attached_sessions(&handle), 1);
    }

    #[test]
    fn stop_after_the_session_drops_does_nothing() {
        let handle = StopHandle::new();
        let session = Session::new().unwrap();
        attach(&handle, session.stopper());
        drop(session);
        handle.stop();
    }
}
