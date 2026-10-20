//! Running a solve and reading its result: the solver, its callbacks and
//! cancellation, response accessors, diagnostics, and the bridge owners.

mod builder;
mod callback;
mod diagnostics;
mod ffi;
mod response;
mod stop;

pub use builder::CpSolver;
pub use callback::{BestBoundCallback, Callbacks, LogCallback, SolutionCallback};
pub use diagnostics::default_parameters;
pub use response::{CpSolverResponseRef, Evaluate, Solution};
pub use stop::StopHandle;
