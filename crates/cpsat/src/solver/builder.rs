use std::ops::ControlFlow;
use std::panic::resume_unwind;
use std::sync::LazyLock;

use buffa::Message;

use crate::error::Result;
use crate::proto::{CpModelProto, CpSolverResponse, SatParameters};
use crate::solver::callback::{BestBoundCallback, Callbacks, LogCallback, SolutionCallback};
use crate::solver::ffi::Session;
use crate::solver::response::CpSolverResponseRef;
use crate::solver::stop::StopHandle;

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
        run(self, parameters, None, &mut ())
    }
}

/// Configures a solve with callbacks and cancellation.
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
///     .add_solution_callback(|response| {
///         println!("objective {}", response.objective_value());
///         ControlFlow::Continue(())
///     })
///     .solve(model.proto())?;
/// # Ok::<(), cpsat::Error>(())
/// ```
#[derive(Debug)]
pub struct CpSolver<'a, C = ()> {
    parameters: &'a SatParameters,
    stop: Option<&'a StopHandle>,
    callbacks: C,
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
            callbacks: (),
        }
    }
}

impl Default for CpSolver<'_> {
    fn default() -> Self {
        static DEFAULT: LazyLock<SatParameters> = LazyLock::new(SatParameters::default);
        Self::new(&DEFAULT)
    }
}

impl<'a, C> CpSolver<'a, C> {
    /// The parameters.
    #[must_use]
    pub fn parameters(&self) -> &'a SatParameters {
        self.parameters
    }

    /// Adds a callback for each improving solution.
    ///
    /// Every callback runs on a solver thread, one call at a time, and
    /// callbacks of one kind run in the order they were added. Returning
    /// [`ControlFlow::Break`] stops the search. A panic stops the search and
    /// resumes after the solve returns.
    #[doc(alias("AddSolutionCallback", "solution_callback"))]
    #[must_use]
    pub fn add_solution_callback<F>(self, callback: F) -> CpSolver<'a, (C, SolutionCallback<F>)>
    where
        F: FnMut(CpSolverResponseRef<'_>) -> ControlFlow<()> + Send,
    {
        self.add(SolutionCallback::new(callback))
    }

    /// Adds a callback for each improved objective bound. See
    /// [`CpSolver::add_solution_callback`].
    #[doc(alias("AddBestBoundCallback", "best_bound_callback"))]
    #[must_use]
    pub fn add_best_bound_callback<F>(self, callback: F) -> CpSolver<'a, (C, BestBoundCallback<F>)>
    where
        F: FnMut(f64) -> ControlFlow<()> + Send,
    {
        self.add(BestBoundCallback::new(callback))
    }

    /// Adds a callback for each log message, decoded lossily as UTF-8. See
    /// [`CpSolver::add_solution_callback`].
    ///
    /// Messages arrive only when `log_search_progress` is set. Set
    /// `log_to_stdout` to false to avoid printing them twice.
    #[doc(alias("AddLogCallback", "log_callback"))]
    #[must_use]
    pub fn add_log_callback<F>(self, callback: F) -> CpSolver<'a, (C, LogCallback<F>)>
    where
        F: FnMut(&str) -> ControlFlow<()> + Send,
    {
        self.add(LogCallback::new(callback))
    }

    fn add<K>(self, callback: K) -> CpSolver<'a, (C, K)> {
        CpSolver {
            parameters: self.parameters,
            stop: self.stop,
            callbacks: (self.callbacks, callback),
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
        C: Callbacks,
    {
        run(
            model.as_ref(),
            self.parameters,
            self.stop,
            &mut self.callbacks,
        )
    }
}

fn run<C: Callbacks>(
    model: &CpModelProto,
    parameters: &SatParameters,
    stop: Option<&StopHandle>,
    callbacks: &mut C,
) -> Result<CpSolverResponse> {
    let model = model.encode_to_vec();
    let session = Session::new()?;
    session.set_parameters(&parameters.encode_to_vec())?;
    // SAFETY: `callbacks` stays borrowed in place until this function returns,
    // after the solve.
    unsafe { callbacks.register(&session)? };
    // SAFETY: the guard drops at the end of this scope, before the session.
    let _attached = stop.map(|stop| unsafe { crate::solver::stop::attach(stop, session.as_ptr()) });
    let response = session.solve(&model);
    if let Some(payload) = callbacks.take_panic() {
        resume_unwind(payload);
    }
    Ok(CpSolverResponse::decode_from_slice(response?.bytes())?)
}
