//! Solve entry points, cancellation, and the event callback trampoline.

use core::ffi::c_void;
use std::any::Any;
use std::ops::{BitOr, ControlFlow};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use anyhow::{Context, Result};
use buffa::{Message, MessageView};
use cpsat_sys::{
    CPSAT_EVENT_BEST_BOUND, CPSAT_EVENT_LOG, CPSAT_EVENT_SOLUTION, CpsatEvent, CpsatSession,
};

use crate::native::{Buffer, Session};
use crate::proto::view::CpSolverResponseView;
use crate::proto::{CpModelProto, CpSolverResponse, SatParameters};

/// The set of events a solve reports to its callback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events(u32);

impl Events {
    pub const NONE: Self = Self(0);
    pub const SOLUTION: Self = Self(CPSAT_EVENT_SOLUTION);
    pub const BEST_BOUND: Self = Self(CPSAT_EVENT_BEST_BOUND);
    /// Log lines arrive only when `log_search_progress` is set. Set
    /// `log_to_stdout` to false to avoid printing them twice.
    pub const LOG: Self = Self(CPSAT_EVENT_LOG);
    pub const ALL: Self = Self(Self::SOLUTION.0 | Self::BEST_BOUND.0 | Self::LOG.0);

    #[must_use]
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for Events {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// One solver event. Borrowed data is valid only during the callback.
#[derive(Debug)]
pub enum Event<'a> {
    /// A new solution, as a zero-copy view of the intermediate response. Call
    /// [`MessageView::to_owned_message`] to keep it.
    Solution(&'a CpSolverResponseView<'a>),
    /// A new best objective bound.
    BestBound(f64),
    /// One log message, decoded lossily as UTF-8.
    Log(&'a str),
}

/// Solves the model and returns the final response.
///
/// # Errors
///
/// Returns an error if the native bridge fails. An invalid model is not an
/// error; the response status is `MODEL_INVALID`.
pub fn solve(model: &CpModelProto, params: &SatParameters) -> Result<CpSolverResponse> {
    solve_with(model, params, Events::NONE, None, |_| {
        ControlFlow::Continue(())
    })
}

/// Solves the model, reporting the subscribed `events` to `on_event`.
///
/// `on_event` runs on a solver thread, one event at a time. Returning
/// [`ControlFlow::Break`] stops the search, as does `stop` from any thread. A
/// panic in `on_event` stops the search and resumes after the solve returns.
///
/// # Errors
///
/// Returns an error if the native bridge fails. An invalid model is not an
/// error; the response status is `MODEL_INVALID`.
pub fn solve_with<F>(
    model: &CpModelProto,
    params: &SatParameters,
    events: Events,
    stop: Option<&StopHandle>,
    on_event: F,
) -> Result<CpSolverResponse>
where
    F: FnMut(Event<'_>) -> ControlFlow<()> + Send,
{
    let model = model.encode_to_vec();
    let params = params.encode_to_vec();
    let session = Session::new()?;
    // SAFETY: the guard drops at the end of this scope, before the session.
    let _attached = stop.map(|stop| unsafe { stop.attach(session.as_ptr()) });

    let mut state = State {
        on_event,
        panic: None,
    };
    let mut response = Buffer::new();
    let mut error = Buffer::new();
    // SAFETY: the byte slices and state outlive the call, the buffers are
    // empty bridge buffers, and trampoline::<F> matches the state type.
    let code = unsafe {
        cpsat_sys::cpsat_session_solve(
            session.as_ptr(),
            model.as_ptr(),
            model.len(),
            params.as_ptr(),
            params.len(),
            events.0,
            Some(trampoline::<F>),
            (&raw mut state).cast::<c_void>(),
            response.as_mut_ptr(),
            error.as_mut_ptr(),
        )
    };
    if let Some(payload) = state.panic {
        resume_unwind(payload);
    }
    crate::native::check(code, &error)?;
    CpSolverResponse::decode_from_slice(response.bytes())
        .context("failed to decode CpSolverResponse")
}

struct State<F> {
    on_event: F,
    panic: Option<Box<dyn Any + Send>>,
}

/// Returns nonzero to stop the search.
unsafe extern "C" fn trampoline<F>(context: *mut c_void, event: *const CpsatEvent) -> i32
where
    F: FnMut(Event<'_>) -> ControlFlow<()> + Send,
{
    // SAFETY: context is the State<F> that solve_with keeps alive during the
    // solve, and the bridge delivers one event at a time, so this is the only
    // live reference.
    let state = unsafe { &mut *context.cast::<State<F>>() };
    if state.panic.is_some() {
        return 1;
    }
    // SAFETY: the bridge passes a valid event for the duration of the call.
    let event = unsafe { &*event };
    let data = if event.data.is_null() {
        &[][..]
    } else {
        // SAFETY: data points to len bytes valid for the duration of the call.
        unsafe { core::slice::from_raw_parts(event.data, event.len) }
    };
    let on_event = &mut state.on_event;
    match catch_unwind(AssertUnwindSafe(|| {
        dispatch(on_event, event.kind, data, event.bound)
    })) {
        Ok(ControlFlow::Continue(())) => 0,
        Ok(ControlFlow::Break(())) => 1,
        Err(payload) => {
            state.panic = Some(payload);
            1
        }
    }
}

fn dispatch<F>(on_event: &mut F, kind: u32, data: &[u8], bound: f64) -> ControlFlow<()>
where
    F: FnMut(Event<'_>) -> ControlFlow<()>,
{
    match kind {
        CPSAT_EVENT_SOLUTION => {
            let view = CpSolverResponseView::decode_view(data)
                .expect("the bridge sends a serialized CpSolverResponse");
            on_event(Event::Solution(&view))
        }
        CPSAT_EVENT_BEST_BOUND => on_event(Event::BestBound(bound)),
        CPSAT_EVENT_LOG => on_event(Event::Log(&String::from_utf8_lossy(data))),
        _ => unreachable!("the bridge sends only the event kinds in cpsat_sys"),
    }
}

/// Stops a solve from any thread, including from inside its event callback.
///
/// A stop is sticky. Stopping before the solve starts makes it return at once,
/// stopping after it ends does nothing, and a stopped handle stops every later
/// solve it is passed to. Clones share one state.
#[derive(Clone, Debug, Default)]
pub struct StopHandle(Arc<Shared>);

#[derive(Debug, Default)]
struct Shared {
    stopped: AtomicBool,
    session: Mutex<Option<SessionPtr>>,
}

#[derive(Debug)]
struct SessionPtr(*mut CpsatSession);

// SAFETY: cpsat_session_stop is thread-safe, and the pointer is only used
// under the mutex while the session is alive.
unsafe impl Send for SessionPtr {}

impl StopHandle {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stop(&self) {
        // Set the flag before locking, so attach either sees it or registers
        // the session before this thread looks.
        self.0.stopped.store(true, Ordering::SeqCst);
        if let Some(session) = &*self.lock() {
            // SAFETY: the session stays alive while it is registered.
            unsafe { cpsat_sys::cpsat_session_stop(session.0) };
        }
    }

    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.0.stopped.load(Ordering::SeqCst)
    }

    /// Registers a live session until the returned guard drops.
    ///
    /// # Safety
    ///
    /// The session must stay alive until the guard drops.
    unsafe fn attach(&self, session: *mut CpsatSession) -> Attached<'_> {
        let mut slot = self.lock();
        *slot = Some(SessionPtr(session));
        if self.is_stopped() {
            // SAFETY: the caller keeps the session alive.
            unsafe { cpsat_sys::cpsat_session_stop(session) };
        }
        Attached(self)
    }

    fn lock(&self) -> MutexGuard<'_, Option<SessionPtr>> {
        // Nothing panics while holding the lock, so a poisoned state is intact.
        self.0
            .session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Detaches the session from its handle on drop.
struct Attached<'a>(&'a StopHandle);

impl Drop for Attached<'_> {
    fn drop(&mut self) {
        *self.0.lock() = None;
    }
}
