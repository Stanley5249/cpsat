//! Every hand-written item of the crate, for `use cpsat::prelude::*`. The
//! generated types live in [`proto`](crate::proto), and [`Error`](crate::Error)
//! and [`Result`](crate::Result) at the crate root.

pub use crate::constraint::{Constraint, ConstraintRef};
pub use crate::expr::{BoolVar, Domain, FloatExpr, IntVar, IntervalVar, LinearExpr};
pub use crate::model::{Hint, Model, Objective};
pub use crate::solver::{
    Evaluate, Event, EventSet, NoEvents, Solver, StopHandle, default_parameters,
};
