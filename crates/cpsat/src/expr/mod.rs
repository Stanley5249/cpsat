//! The vocabulary a model is written in: variable handles, integer and
//! floating-point linear expressions, and domains.

mod domain;
mod float;
mod linear;
mod var;

pub use domain::Domain;
pub use float::FloatExpr;
pub use linear::LinearExpr;
pub use var::{BoolVar, IntVar, IntervalVar};
