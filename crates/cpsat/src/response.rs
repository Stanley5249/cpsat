//! Accessors on the generated response types, like Python's `CpSolver.value`
//! and `CpSolverSolutionCallback.value`.

use std::fmt;

use buffa::Enumeration;

use crate::expr::LinearExpr;
use crate::float_expr::FloatExpr;
use crate::proto::view::CpSolverResponseView;
use crate::proto::{CpSolverResponse, CpSolverStatus};
use crate::variable::BoolVar;

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

/// The value of `expr` under `solution`, or `None` if a variable has no value.
fn value(solution: &[i64], expr: impl Into<LinearExpr>) -> Option<i64> {
    expr.into().evaluate(solution)
}

/// The value of `literal` under `solution`, or `None` without a solution.
fn boolean_value(solution: &[i64], literal: BoolVar) -> Option<bool> {
    let value = solution.get(usize::try_from(literal.variable().index()).ok()?)?;
    Some((*value != 0) != literal.is_negated())
}

impl CpSolverResponse {
    /// The status, with values from a newer schema read as `UNKNOWN`.
    #[must_use]
    pub fn status(&self) -> CpSolverStatus {
        self.status.as_known().unwrap_or(CpSolverStatus::UNKNOWN)
    }

    /// The value of a variable or expression in the solution, like Python's
    /// `CpSolver.value`, or `None` if a variable has no value, such as without
    /// a solution. A constant has its value even without a solution, as in
    /// Python. Handles do not know their model, so a handle from another
    /// model reads `None` when out of range and another variable otherwise.
    ///
    /// # Panics
    ///
    /// If the value overflows `i64`.
    #[must_use]
    pub fn value(&self, expr: impl Into<LinearExpr>) -> Option<i64> {
        value(&self.solution, expr)
    }

    /// The value of a floating-point expression in the solution, like
    /// Python's `CpSolver.float_value`, or `None` if a variable has no value.
    #[must_use]
    pub fn float_value(&self, expr: impl Into<FloatExpr>) -> Option<f64> {
        expr.into().evaluate(&self.solution)
    }

    /// The value of a literal in the solution, like Python's
    /// `CpSolver.boolean_value`, or `None` without a solution.
    #[must_use]
    pub fn boolean_value(&self, literal: BoolVar) -> Option<bool> {
        boolean_value(&self.solution, literal)
    }
}

impl CpSolverResponseView<'_> {
    /// The status, with values from a newer schema read as `UNKNOWN`.
    #[must_use]
    pub fn status(&self) -> CpSolverStatus {
        self.status.as_known().unwrap_or(CpSolverStatus::UNKNOWN)
    }

    /// The value of a variable or expression in the solution, like Python's
    /// `CpSolverSolutionCallback.value`.
    ///
    /// # Panics
    ///
    /// If the value overflows `i64`.
    #[must_use]
    pub fn value(&self, expr: impl Into<LinearExpr>) -> Option<i64> {
        value(&self.solution, expr)
    }

    /// The value of a floating-point expression in the solution, like
    /// Python's `CpSolverSolutionCallback.float_value`.
    #[must_use]
    pub fn float_value(&self, expr: impl Into<FloatExpr>) -> Option<f64> {
        expr.into().evaluate(&self.solution)
    }

    /// The value of a literal in the solution, like Python's
    /// `CpSolverSolutionCallback.boolean_value`.
    #[must_use]
    pub fn boolean_value(&self, literal: BoolVar) -> Option<bool> {
        boolean_value(&self.solution, literal)
    }
}
