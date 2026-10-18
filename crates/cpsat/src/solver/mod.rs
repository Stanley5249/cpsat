//! Solving, solver events, and cancellation.

mod builder;
mod callback;
mod event;
mod stop;

pub use builder::{NoEvents, Solver, solve};
pub use event::{Event, EventSet};
pub use stop::StopHandle;
