//! Running a solve and reading its result: the solver, its events and
//! cancellation, response accessors, diagnostics, and the bridge owners.

mod builder;
mod callback;
mod diagnostics;
mod event;
mod ffi;
mod response;
mod stop;

pub use builder::{CpSolver, NoEvents};
pub use diagnostics::default_parameters;
pub use event::{Event, EventSet};
pub use response::Evaluate;
pub use stop::StopHandle;
