use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use cpsat_sys::CpsatSession;

/// Stops solves from any thread, including from inside an event callback.
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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stops every attached solve and every later one, like Python's
    /// `CpSolver.stop_search`.
    pub fn stop(&self) {
        // Set the flag before locking, so attach either sees it or registers
        // its session before this thread looks.
        self.0.stopped.store(true, Ordering::SeqCst);
        for session in self.lock().iter() {
            // SAFETY: a session stays alive while it is registered.
            unsafe { cpsat_sys::cpsat_session_stop(session.0) };
        }
    }

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

/// Registers a live session with `handle` until the returned guard drops.
///
/// This is a free function rather than a method so it stays out of the public
/// API.
///
/// # Safety
///
/// The session must stay alive until the guard drops.
pub unsafe fn attach(handle: &StopHandle, session: *mut CpsatSession) -> Attached<'_> {
    let mut sessions = handle.lock();
    sessions.push(SessionPtr(session));
    if handle.is_stopped() {
        // SAFETY: the caller keeps the session alive.
        unsafe { cpsat_sys::cpsat_session_stop(session) };
    }
    Attached { handle, session }
}

/// Detaches one session from its handle on drop.
pub struct Attached<'a> {
    handle: &'a StopHandle,
    session: *mut CpsatSession,
}

impl Drop for Attached<'_> {
    fn drop(&mut self) {
        let mut sessions = self.handle.lock();
        if let Some(index) = sessions.iter().position(|s| s.0 == self.session) {
            sessions.swap_remove(index);
        }
    }
}
