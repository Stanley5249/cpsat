//! Every hand-written item of the crate, for `use cpsat::prelude::*`. The
//! generated types live in [`proto`](crate::proto), and [`Error`](crate::Error)
//! and [`Result`](crate::Result) at the crate root.

pub use crate::constraint::{Constraint, ConstraintIndex};
pub use crate::expr::{BoolVar, Domain, FloatExpr, IntVar, IntervalVar, LinearExpr};
pub use crate::model::{Bool, CpModel, Hint, Interval, Named, NewVar, Objective, VarMut, WithName};
pub use crate::solver::{
    CpSolver, Evaluate, Event, EventSet, NoEvents, Solution, StopHandle, default_parameters,
};
