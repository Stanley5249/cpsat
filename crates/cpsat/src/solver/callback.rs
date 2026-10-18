//! The trampoline that turns bridge events into Rust callbacks.

use core::ffi::c_void;
use std::any::Any;
use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};

use buffa::MessageView;
use cpsat_sys::{CPSAT_EVENT_BEST_BOUND, CPSAT_EVENT_LOG, CPSAT_EVENT_SOLUTION, CpsatEvent};

use crate::proto::view::CpSolverResponseView;
use crate::solver::event::Event;

/// Callback state that the bridge passes back as the context pointer.
pub struct State<F> {
    pub on_event: F,
    /// The first panic from `on_event`, resumed after the solve returns, as
    /// Python's binding reraises the first callback exception.
    pub panic: Option<Box<dyn Any + Send>>,
}

/// Returns nonzero to stop the search.
///
/// # Safety
///
/// `context` must point to a live `State<F>`, and `event` to a valid event.
pub unsafe extern "C" fn trampoline<F>(context: *mut c_void, event: *const CpsatEvent) -> i32
where
    F: FnMut(Event<'_>) -> ControlFlow<()> + Send,
{
    // SAFETY: the bridge's event mutex delivers one event at a time, so this
    // is the only live reference to the state.
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
        CPSAT_EVENT_LOG => {
            let text = String::from_utf8_lossy(data);
            on_event(Event::Log(&text))
        }
        _ => unreachable!("the bridge sends only the event kinds in cpsat_sys"),
    }
}
