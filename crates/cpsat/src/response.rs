//! Accessors on the generated response types, like Python's `CpSolver.value`
//! and `CpSolverSolutionCallback.value`.

use std::fmt;

use buffa::Enumeration;

use crate::expr::LinearExpr;
use crate::float_expr::FloatExpr;
use crate::proto::view::CpSolverResponseView;
use crate::proto::{CpSolverResponse, CpSolverStatus};
use crate::variable::{BoolVar, IntVar};

impl fmt::Display for CpSolverStatus {
    /// Writes the schema name, such as `OPTIMAL`, like Python's `status_name`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.proto_name())
    }
}

impl CpSolverStatus {
    /// Whether the response holds a solution.
    #[must_use]
    pub fn has_solution(self) -> bool {
        matches!(self, Self::OPTIMAL | Self::FEASIBLE)
    }
}

mod sealed {
    pub trait Sealed {}
}

/// What `value` reads, and the type it reads as: an integer for a variable,
/// a constant, or an expression, a `bool` for a literal, and an `f64` for a
/// floating-point expression. A handle reads its slot directly, so no
/// expression is built.
pub trait Evaluate: sealed::Sealed {
    type Output;

    /// The value under `solution`, or `None` if a variable has no value.
    #[doc(hidden)]
    fn evaluate(&self, solution: &[i64]) -> Option<Self::Output>;
}

fn slot(solution: &[i64], index: i32) -> Option<i64> {
    solution.get(usize::try_from(index).ok()?).copied()
}

impl sealed::Sealed for IntVar {}
impl Evaluate for IntVar {
    type Output = i64;

    fn evaluate(&self, solution: &[i64]) -> Option<i64> {
        slot(solution, self.index())
    }
}

impl sealed::Sealed for BoolVar {}
impl Evaluate for BoolVar {
    type Output = bool;

    fn evaluate(&self, solution: &[i64]) -> Option<bool> {
        let value = slot(solution, self.variable().index())?;
        Some((value != 0) != self.is_negated())
    }
}

impl sealed::Sealed for i64 {}
impl Evaluate for i64 {
    type Output = i64;

    fn evaluate(&self, _: &[i64]) -> Option<i64> {
        Some(*self)
    }
}

impl sealed::Sealed for LinearExpr {}
impl Evaluate for LinearExpr {
    type Output = i64;

    fn evaluate(&self, solution: &[i64]) -> Option<i64> {
        self.value_in(solution)
    }
}

impl sealed::Sealed for f64 {}
impl Evaluate for f64 {
    type Output = f64;

    fn evaluate(&self, _: &[i64]) -> Option<f64> {
        Some(*self)
    }
}

impl sealed::Sealed for FloatExpr {}
impl Evaluate for FloatExpr {
    type Output = f64;

    fn evaluate(&self, solution: &[i64]) -> Option<f64> {
        self.value_in(solution)
    }
}

impl<T: Evaluate> sealed::Sealed for &T {}
impl<T: Evaluate> Evaluate for &T {
    type Output = T::Output;

    fn evaluate(&self, solution: &[i64]) -> Option<T::Output> {
        T::evaluate(self, solution)
    }
}

impl CpSolverResponse {
    /// The status, with values from a newer schema read as `UNKNOWN`.
    #[must_use]
    pub fn status(&self) -> CpSolverStatus {
        self.status.as_known().unwrap_or(CpSolverStatus::UNKNOWN)
    }

    /// The value of a variable, literal, or expression in the solution, like
    /// Python's `CpSolver.value`, `boolean_value`, and `float_value`, or
    /// `None` if a variable has no value, such as without a solution. A
    /// constant has its value even without a solution, as in Python. Handles
    /// do not know their model, so a handle from another model reads `None`
    /// when out of range and another variable otherwise.
    ///
    /// # Panics
    ///
    /// If an integer value overflows `i64`.
    #[must_use]
    pub fn value<E: Evaluate>(&self, expr: E) -> Option<E::Output> {
        expr.evaluate(&self.solution)
    }
}

impl CpSolverResponseView<'_> {
    /// The status, with values from a newer schema read as `UNKNOWN`.
    #[must_use]
    pub fn status(&self) -> CpSolverStatus {
        self.status.as_known().unwrap_or(CpSolverStatus::UNKNOWN)
    }

    /// The value of a variable, literal, or expression in the solution, like
    /// Python's `CpSolverSolutionCallback.value`, `boolean_value`, and
    /// `float_value`. See [`CpSolverResponse::value`].
    ///
    /// # Panics
    ///
    /// If an integer value overflows `i64`.
    #[must_use]
    pub fn value<E: Evaluate>(&self, expr: E) -> Option<E::Output> {
        expr.evaluate(&self.solution)
    }
}
