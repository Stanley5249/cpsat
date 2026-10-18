//! Accessors on the generated response types, like Python's `CpSolver.value`
//! and `CpSolverSolutionCallback.value`.

use std::fmt;

use buffa::Enumeration;

use crate::proto::view::CpSolverResponseView;
use crate::proto::{CpSolverResponse, CpSolverStatus};

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

/// The value of a variable reference, where `-i - 1` means `-x_i`.
fn value(solution: &[i64], var: i32) -> Option<i64> {
    match usize::try_from(var) {
        Ok(index) => solution.get(index).copied(),
        Err(_) => value(solution, crate::not(var)).map(|value| -value),
    }
}

/// The value of a literal, where `-i - 1` means "not `i`".
fn boolean_value(solution: &[i64], literal: i32) -> Option<bool> {
    match usize::try_from(literal) {
        Ok(index) => solution.get(index).map(|&value| value != 0),
        Err(_) => boolean_value(solution, crate::not(literal)).map(|value| !value),
    }
}

impl CpSolverResponse {
    /// The status, with values from a newer schema read as `UNKNOWN`.
    #[must_use]
    pub fn status(&self) -> CpSolverStatus {
        self.status.as_known().unwrap_or(CpSolverStatus::UNKNOWN)
    }

    /// The value of a variable in the solution, or `None` without a solution.
    #[must_use]
    pub fn value(&self, var: i32) -> Option<i64> {
        value(&self.solution, var)
    }

    /// The value of a literal in the solution, or `None` without a solution.
    #[must_use]
    pub fn boolean_value(&self, literal: i32) -> Option<bool> {
        boolean_value(&self.solution, literal)
    }
}

impl CpSolverResponseView<'_> {
    /// The status, with values from a newer schema read as `UNKNOWN`.
    #[must_use]
    pub fn status(&self) -> CpSolverStatus {
        self.status.as_known().unwrap_or(CpSolverStatus::UNKNOWN)
    }

    /// The value of a variable in the solution.
    #[must_use]
    pub fn value(&self, var: i32) -> Option<i64> {
        value(&self.solution, var)
    }

    /// The value of a literal in the solution.
    #[must_use]
    pub fn boolean_value(&self, literal: i32) -> Option<bool> {
        boolean_value(&self.solution, literal)
    }
}
