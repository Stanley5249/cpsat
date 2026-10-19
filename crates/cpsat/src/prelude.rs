//! Every hand-written item of the crate, for `use cpsat::prelude::*`. The
//! generated types live in [`proto`](crate::proto), and [`Error`](crate::Error)
//! and [`Result`](crate::Result) at the crate root.

pub use crate::constraint::{Constraint, ConstraintRef};
pub use crate::diagnostics::default_parameters;
pub use crate::domain::Domain;
pub use crate::expr::LinearExpr;
pub use crate::float_expr::FloatExpr;
pub use crate::model::{Hint, Model, Objective};
pub use crate::response::Evaluate;
pub use crate::solver::{Event, EventSet, NoEvents, Solver, StopHandle};
pub use crate::variable::{BoolVar, IntVar, IntervalVar};
