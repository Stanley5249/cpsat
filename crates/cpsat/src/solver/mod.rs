//! Running a solve and reading its result: the solver, its callbacks and
//! cancellation, response accessors, diagnostics, and the bridge owners.

mod diagnostics;
#[expect(unsafe_code, reason = "the only module that calls cpsat_sys")]
mod ffi;
mod response;
mod stop;

pub use diagnostics::default_parameters;
pub use response::{Evaluate, Solution};
pub use stop::StopHandle;

use std::fmt;
use std::ops::ControlFlow;
use std::sync::LazyLock;

use buffa::Message;

use self::ffi::{Callbacks, Session};
use crate::error::Result;
use crate::proto::view::CpSolverResponseView;
use crate::proto::{CpModelProto, CpSolverResponse, SatParameters};

impl CpModelProto {
    /// Solves the model and returns the final response without a callback.
    /// [`CpSolver`] adds callbacks and stops.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge fails. An invalid model or invalid
    /// parameters are not an error; the response status is `MODEL_INVALID`.
    #[doc(alias("SolveWithParameters"))]
    pub fn solve(&self, parameters: &SatParameters) -> Result<CpSolverResponse> {
        run(self, parameters, None, &mut Callbacks::default())
    }
}

/// Configures a solve with callbacks and cancellation.
///
/// The solver borrows each callback mutably, so a closure is readable again,
/// with everything it captured, after the solver's last use.
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
///     .add_solution_callback(&mut |response| {
///         println!("objective {}", response.objective_value);
///         ControlFlow::Continue(())
///     })
///     .solve(&model)?;
/// # Ok::<(), cpsat::Error>(())
/// ```
pub struct CpSolver<'a> {
    parameters: &'a SatParameters,
    stop: Option<&'a StopHandle>,
    callbacks: Callbacks<'a>,
}

impl<'a> CpSolver<'a> {
    /// A solver that borrows `parameters`, like [`CpModelProto::solve`] does. Build a new solver
    /// to solve with other parameters, and bind the parameters to a variable
    /// first to keep the solver for several solves.
    #[must_use]
    pub fn new(parameters: &'a SatParameters) -> Self {
        Self {
            parameters,
            stop: None,
            callbacks: Callbacks::default(),
        }
    }

    /// The parameters.
    #[must_use]
    pub fn parameters(&self) -> &'a SatParameters {
        self.parameters
    }

    /// Adds a callback for each improving solution, which reads it as a
    /// zero-copy view. Call [`buffa::MessageView::to_owned_message`] to keep it.
    ///
    /// Every callback runs on a solver thread, one call at a time, and
    /// callbacks of one kind run in the order they were added. Returning
    /// [`ControlFlow::Break`] stops the search, and later callbacks still
    /// receive the same event. The first panic stops the
    /// search, skips every later callback call, and resumes after the solve
    /// returns.
    #[doc(alias("AddSolutionCallback", "solution_callback"))]
    #[must_use]
    pub fn add_solution_callback<F>(mut self, callback: &'a mut F) -> Self
    where
        F: FnMut(&CpSolverResponseView<'_>) -> ControlFlow<()> + Send,
    {
        self.callbacks.solution.push(callback);
        self
    }

    /// Adds a callback for each improved objective bound. See
    /// [`CpSolver::add_solution_callback`].
    #[doc(alias("AddBestBoundCallback", "best_bound_callback"))]
    #[must_use]
    pub fn add_best_bound_callback<F>(mut self, callback: &'a mut F) -> Self
    where
        F: FnMut(f64) -> ControlFlow<()> + Send,
    {
        self.callbacks.best_bound.push(callback);
        self
    }

    /// Adds a callback for each log message, decoded lossily as UTF-8. See
    /// [`CpSolver::add_solution_callback`].
    ///
    /// Messages arrive only when `log_search_progress` is set. Set
    /// `log_to_stdout` to false to avoid printing them twice.
    #[doc(alias("AddLogCallback", "log_callback"))]
    #[must_use]
    pub fn add_log_callback<F>(mut self, callback: &'a mut F) -> Self
    where
        F: FnMut(&str) -> ControlFlow<()> + Send,
    {
        self.callbacks.log.push(callback);
        self
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
    pub fn solve<M: AsRef<CpModelProto> + ?Sized>(
        &mut self,
        model: &M,
    ) -> Result<CpSolverResponse> {
        run(
            model.as_ref(),
            self.parameters,
            self.stop,
            &mut self.callbacks,
        )
    }
}

impl Default for CpSolver<'_> {
    fn default() -> Self {
        static DEFAULT: LazyLock<SatParameters> = LazyLock::new(SatParameters::default);
        Self::new(&DEFAULT)
    }
}

impl fmt::Debug for CpSolver<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpSolver")
            .field("parameters", self.parameters)
            .field("stop", &self.stop)
            .finish_non_exhaustive()
    }
}

/// Solves `model` once with a fresh session.
fn run(
    model: &CpModelProto,
    parameters: &SatParameters,
    stop: Option<&StopHandle>,
    callbacks: &mut Callbacks<'_>,
) -> Result<CpSolverResponse> {
    let model = model.encode_to_vec();
    let mut session = Session::new()?;
    session.set_parameters(&parameters.encode_to_vec())?;
    if let Some(stop) = stop {
        stop::attach(stop, session.stopper());
    }
    let response = session.solve(&model, callbacks)?;
    Ok(CpSolverResponse::decode_from_slice(response.bytes())?)
}
