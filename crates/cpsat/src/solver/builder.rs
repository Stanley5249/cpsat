use core::ffi::c_void;
use std::ops::ControlFlow;
use std::panic::resume_unwind;
use std::sync::LazyLock;

use buffa::Message;

use crate::error::Result;
use crate::proto::{CpModelProto, CpSolverResponse, SatParameters};
use crate::solver::callback::{State, trampoline};
use crate::solver::event::{Event, EventSet};
use crate::solver::ffi::{Buffer, Session};
use crate::solver::stop::StopHandle;

/// The callback type of a [`CpSolver`] without events.
pub type NoEvents = fn(Event<'_>) -> ControlFlow<()>;

impl CpModelProto {
    /// Solves the model and returns the final response without a callback.
    /// [`CpSolver`] adds events and stops.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge fails. An invalid model or invalid
    /// parameters are not an error; the response status is `MODEL_INVALID`.
    #[doc(alias("SolveWithParameters"))]
    pub fn solve(&self, parameters: &SatParameters) -> Result<CpSolverResponse> {
        run(self, parameters, EventSet::NONE, None, |_| {
            ControlFlow::Continue(())
        })
    }
}

/// Configures a solve with events and cancellation.
///
/// ```no_run
/// use std::ops::ControlFlow;
///
/// use cpsat::prelude::*;
/// use cpsat::proto::SatParameters;
///
/// // From `solve_and_print_intermediate_solutions_sample_sat.py`.
/// let mut model = CpModel::new();
/// let [x, y, z] = std::array::from_fn(|_| model.new_var(0..=2));
/// model.add(x.ne(y));
/// model.maximize(x + 2 * y + 3 * z);
///
/// let parameters = SatParameters::default().with_num_workers(8);
/// let response = CpSolver::new(&parameters)
///     .on_event(EventSet::SOLUTION, |event| {
///         if let Event::Solution(view) = event {
///             println!("objective {}", view.objective_value);
///         }
///         ControlFlow::Continue(())
///     })
///     .solve(model.proto())?;
/// # Ok::<(), cpsat::Error>(())
/// ```
#[derive(Debug)]
pub struct CpSolver<'a, F = NoEvents> {
    parameters: &'a SatParameters,
    events: EventSet,
    stop: Option<&'a StopHandle>,
    on_event: F,
}

impl<'a> CpSolver<'a> {
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

impl Default for CpSolver<'_> {
    fn default() -> Self {
        static DEFAULT: LazyLock<SatParameters> = LazyLock::new(SatParameters::default);
        Self::new(&DEFAULT)
    }
}

impl<'a, F> CpSolver<'a, F> {
    /// The parameters.
    #[must_use]
    pub fn parameters(&self) -> &'a SatParameters {
        self.parameters
    }

    /// Reports `events` to `on_event`, which runs on a solver thread, one
    /// event at a time. Returning [`ControlFlow::Break`] stops the search. A
    /// panic in `on_event` stops the search and resumes after the solve
    /// returns.
    #[must_use]
    pub fn on_event<G>(self, events: EventSet, on_event: G) -> CpSolver<'a, G>
    where
        G: FnMut(Event<'_>) -> ControlFlow<()> + Send,
    {
        CpSolver {
            parameters: self.parameters,
            events,
            stop: self.stop,
            on_event,
        }
    }

    /// Lets [`StopHandle::stop`] on `stop` stop the solve from any thread.
    #[must_use]
    pub fn with_stop_handle(self, stop: &'a StopHandle) -> Self {
        Self {
            stop: Some(stop),
            ..self
        }
    }

    /// Solves the model and returns the final response. The solver can solve
    /// again, for example after the model changes.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge fails. An invalid model or invalid
    /// parameters are not an error; the response status is `MODEL_INVALID`.
    pub fn solve<M: AsRef<CpModelProto> + ?Sized>(&mut self, model: &M) -> Result<CpSolverResponse>
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
    crate::solver::ffi::check(code, &error)?;
    Ok(CpSolverResponse::decode_from_slice(response.bytes())?)
}
