use core::ffi::c_void;
use std::ops::ControlFlow;
use std::panic::resume_unwind;
use std::sync::LazyLock;

use buffa::Message;

use crate::error::Result;
use crate::ffi::{Buffer, Session};
use crate::proto::{CpModelProto, CpSolverResponse, SatParameters};
use crate::solver::callback::{State, trampoline};
use crate::solver::event::{Event, EventSet};
use crate::solver::stop::StopHandle;

/// The callback type of a [`Solver`] without events.
pub type NoEvents = fn(Event<'_>) -> ControlFlow<()>;

impl CpModelProto {
    /// Solves the model and returns the final response, like Python's
    /// `CpSolver.solve` without a callback. [`Solver`] adds events and stops.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge fails. An invalid model is not an
    /// error; the response status is `MODEL_INVALID`.
    pub fn solve(&self, parameters: &SatParameters) -> Result<CpSolverResponse> {
        run(self, parameters, EventSet::NONE, None, |_| {
            ControlFlow::Continue(())
        })
    }
}

/// Configures a solve with events and cancellation, like Python's `CpSolver`.
///
/// ```no_run
/// use std::ops::ControlFlow;
///
/// use cpsat::prelude::*;
/// use cpsat::proto::{CpModelProto, SatParameters};
///
/// let model = CpModelProto::default();
/// let parameters = SatParameters::default().with_num_workers(8);
/// let response = Solver::new(&parameters)
///     .on_event(EventSet::SOLUTION, |event| {
///         if let Event::Solution(view) = event {
///             println!("objective {}", view.objective_value);
///         }
///         ControlFlow::Continue(())
///     })
///     .solve(&model)?;
/// # Ok::<(), cpsat::Error>(())
/// ```
#[derive(Debug)]
pub struct Solver<'a, F = NoEvents> {
    parameters: &'a SatParameters,
    events: EventSet,
    stop: Option<&'a StopHandle>,
    on_event: F,
}

impl<'a> Solver<'a> {
    /// A solver that borrows `parameters`, like [`CpModelProto::solve`] does. Build a new solver
    /// to solve with other parameters, and bind the parameters to a variable
    /// first to keep the solver for several solves.
    #[must_use]
    pub fn new(parameters: &'a SatParameters) -> Self {
        Self {
            parameters,
            events: EventSet::NONE,
            stop: None,
            on_event: |_| ControlFlow::Continue(()),
        }
    }
}

impl Default for Solver<'_> {
    fn default() -> Self {
        static DEFAULT: LazyLock<SatParameters> = LazyLock::new(SatParameters::default);
        Self::new(&DEFAULT)
    }
}

impl<'a, F> Solver<'a, F> {
    /// The parameters, like Python's `solver.parameters`.
    #[must_use]
    pub fn parameters(&self) -> &'a SatParameters {
        self.parameters
    }

    /// Reports `events` to `on_event`, which runs on a solver thread, one
    /// event at a time. Returning [`ControlFlow::Break`] stops the search. A
    /// panic in `on_event` stops the search and resumes after the solve
    /// returns.
    #[must_use]
    pub fn on_event<G>(self, events: EventSet, on_event: G) -> Solver<'a, G>
    where
        G: FnMut(Event<'_>) -> ControlFlow<()> + Send,
    {
        Solver {
            parameters: self.parameters,
            events,
            stop: self.stop,
            on_event,
        }
    }

    /// Lets [`StopHandle::stop`] on `stop` stop the solve from any thread, like Python's
    /// `CpSolver.stop_search`.
    #[must_use]
    pub fn with_stop_handle(self, stop: &'a StopHandle) -> Self {
        Self {
            stop: Some(stop),
            ..self
        }
    }

    /// Solves the model and returns the final response. The solver can solve
    /// again, for example after the model changes. Pass `&model` to keep a
    /// [`Model`](crate::prelude::Model), because a `Model` by value moves into the call.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge fails. An invalid model is not an
    /// error; the response status is `MODEL_INVALID`.
    pub fn solve(&mut self, model: impl AsRef<CpModelProto>) -> Result<CpSolverResponse>
    where
        F: FnMut(Event<'_>) -> ControlFlow<()> + Send,
    {
        run(
            model.as_ref(),
            self.parameters,
            self.events,
            self.stop,
            &mut self.on_event,
        )
    }
}

fn run<F>(
    model: &CpModelProto,
    parameters: &SatParameters,
    events: EventSet,
    stop: Option<&StopHandle>,
    on_event: F,
) -> Result<CpSolverResponse>
where
    F: FnMut(Event<'_>) -> ControlFlow<()> + Send,
{
    let model = model.encode_to_vec();
    let parameters = parameters.encode_to_vec();
    let session = Session::new()?;
    // SAFETY: the guard drops at the end of this scope, before the session.
    let _attached = stop.map(|stop| unsafe { crate::solver::stop::attach(stop, session.as_ptr()) });

    let mut state = State {
        on_event,
        panic: None,
    };
    let mut response = Buffer::new();
    let mut error = Buffer::new();
    // SAFETY: the byte slices and state outlive the call, the buffers are
    // empty bridge buffers, and the trampoline matches the state type.
    let code = unsafe {
        cpsat_sys::cpsat_session_solve(
            session.as_ptr(),
            model.as_ptr(),
            model.len(),
            parameters.as_ptr(),
            parameters.len(),
            events.bits(),
            Some(trampoline::<F>),
            (&raw mut state).cast::<c_void>(),
            response.as_mut_ptr(),
            error.as_mut_ptr(),
        )
    };
    if let Some(payload) = state.panic {
        resume_unwind(payload);
    }
    crate::ffi::check(code, &error)?;
    Ok(CpSolverResponse::decode_from_slice(response.bytes())?)
}
