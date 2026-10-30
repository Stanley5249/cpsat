use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use cpsat_sys::CpsatSession;

use crate::solver::ffi::Session;

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
    sessions: Mutex<Vec<SessionPtr>>,
}

#[derive(Debug, PartialEq, Eq)]
struct SessionPtr(*mut CpsatSession);

// SAFETY: cpsat_session_stop is thread-safe, and the pointer is only used
// under the mutex while the session is alive.
unsafe impl Send for SessionPtr {}

impl StopHandle {
    /// A handle that has not stopped.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stops every attached solve and every later one.
    #[doc(alias("stop_search", "StopSearch"))]
    pub fn stop(&self) {
        // Set the flag before locking, so `attached` either sees it or registers
        // its session before this thread looks.
        self.0.stopped.store(true, Ordering::SeqCst);
        for session in self.lock().iter() {
            // SAFETY: a session stays alive while it is registered.
            unsafe { cpsat_sys::cpsat_session_stop(session.0) };
        }
    }

    /// Whether [`StopHandle::stop`] has been called.
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.0.stopped.load(Ordering::SeqCst)
    }

    fn lock(&self) -> MutexGuard<'_, Vec<SessionPtr>> {
        // Nothing panics while holding the lock, so a poisoned state is intact.
        self.0
            .sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Runs `solve` while `handle`, if any, can stop `session`.
///
/// This is a free function rather than a method so it stays out of the public
/// API. It takes a closure instead of returning a guard, because a forgotten
/// guard would leave a dangling session in the handle.
pub fn attached<R>(handle: Option<&StopHandle>, session: &Session, solve: impl FnOnce() -> R) -> R {
    let Some(handle) = handle else {
        return solve();
    };
    let mut sessions = handle.lock();
    sessions.push(SessionPtr(session.as_ptr()));
    if handle.is_stopped() {
        session.stop();
    }
    drop(sessions);
    let _detach = Detach { handle, session };
    solve()
}

/// Detaches one session from its handle on drop, including on unwind.
struct Detach<'a> {
    handle: &'a StopHandle,
    session: &'a Session,
}

impl Drop for Detach<'_> {
    fn drop(&mut self) {
        let mut sessions = self.handle.lock();
        let session = self.session.as_ptr();
        if let Some(index) = sessions.iter().position(|s| s.0 == session) {
            sessions.swap_remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;

    /// The number of sessions attached to `handle`.
    fn attached_sessions(handle: &StopHandle) -> usize {
        handle.lock().len()
    }

    #[test]
    fn detaches_after_the_solve_returns() {
        let handle = StopHandle::new();
        let session = Session::new().unwrap();
        let during = attached(Some(&handle), &session, || attached_sessions(&handle));
        assert_eq!(during, 1);
        assert_eq!(attached_sessions(&handle), 0);
    }

    #[test]
    fn detaches_after_the_solve_fails() {
        let handle = StopHandle::new();
        let session = Session::new().unwrap();
        // The bridge rejects bytes that do not parse as a CpModelProto.
        let response = attached(Some(&handle), &session, || session.solve(&[0xff]));
        assert!(response.is_err());
        assert_eq!(attached_sessions(&handle), 0);
    }

    #[test]
    fn detaches_when_the_solve_panics() {
        let handle = StopHandle::new();
        let session = Session::new().unwrap();
        let panic = catch_unwind(AssertUnwindSafe(|| {
            attached(Some(&handle), &session, || panic!("the solve failed"))
        }));
        assert!(panic.is_err());
        assert_eq!(attached_sessions(&handle), 0);
        // A later stop must not reach the session, which the caller may free.
        handle.stop();
    }
}
