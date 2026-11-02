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

/// Converts a schema index into a `Vec` index, which panics for a negative
/// index like an index out of range does.
pub fn slot(index: i32) -> usize {
    usize::try_from(index).expect("an index is non-negative")
}
