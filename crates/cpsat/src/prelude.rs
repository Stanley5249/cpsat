//! The types that typical modeling code needs, for `use cpsat::prelude::*`.

pub use crate::constraint::{Constraint, ConstraintRef};
pub use crate::domain::Domain;
pub use crate::expr::LinearExpr;
pub use crate::model::Model;
pub use crate::proto::{CpSolverStatus, SatParameters};
pub use crate::solver::{Event, EventSet, Solver, StopHandle, solve};
pub use crate::variable::{BoolVar, IntVar, IntervalVar};
