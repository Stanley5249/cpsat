//! Accessors on the generated response types, like Python's `CpSolver.value`
//! and `CpSolverSolutionCallback.value`.

use std::fmt;

use buffa::{Enumeration, Message};
use cpsat_sys::CpsatResponseRef;

use crate::error::Result;
use crate::expr::{BoolVar, FloatExpr, IntVar, LinearExpr};
#[cfg(feature = "views")]
use crate::proto::view::CpSolverResponseView;
use crate::proto::{CpSolverResponse, CpSolverStatus};
use crate::solver::ffi::Buffer;

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

/// One value per variable, in the order of `CpModelProto::variables`, as the
/// schema stores a solution in `solution` and in each `additional_solutions`
/// entry.
///
/// The binding trusts OR-Tools' values and decodes them without checks. A
/// literal is true when its variable is nonzero, like Python, so `!b` always
/// reads as the opposite of `b`.
///
/// ```no_run
/// use cpsat::prelude::*;
/// use cpsat::proto::SatParameters;
///
/// let mut model = CpModel::new();
/// let x = model.new_var(0..=10);
/// let b = model.new_var(Bool);
/// model.maximize(x + b);
/// let response = model.solve(&SatParameters::default())?;
/// if let Some(solution) = response.solution() {
///     println!("x = {}, b = {}", solution.value(x), solution.value(b));
/// }
/// # Ok::<(), cpsat::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Solution<'a>(&'a [i64]);

impl<'a> Solution<'a> {
    /// The raw values, one per row of `CpModelProto::variables`.
    #[must_use]
    pub fn values(self) -> &'a [i64] {
        self.0
    }

    /// The value of a variable, literal, or expression, like Python's
    /// `CpSolver.value`, `boolean_value`, and `float_value`.
    ///
    /// # Panics
    ///
    /// If `expr` has a variable that the solution lacks, such as a handle
    /// from a larger model, like indexing a slice, or if an integer value
    /// overflows `i64`.
    #[doc(alias(
        "boolean_value",
        "float_value",
        "SolutionIntegerValue",
        "SolutionBooleanValue"
    ))]
    #[must_use]
    pub fn value<E: Evaluate>(&self, expr: E) -> E::Output {
        expr.evaluate(self.0)
    }
}

/// What [`Solution::value`] reads, and the type it reads as: an integer for a
/// variable, a constant, or an expression, a `bool` for a literal, and an
/// `f64` for a floating-point expression. A handle reads its slot directly,
/// so no expression is built.
pub trait Evaluate: sealed::Sealed {
    /// The type of the value: `i64`, `bool`, or `f64`.
    type Output;

    /// The value under `solution`.
    #[doc(hidden)]
    fn evaluate(&self, solution: &[i64]) -> Self::Output;
}

fn slot(solution: &[i64], var: IntVar) -> i64 {
    solution[usize::try_from(var.index()).expect("a variable index is non-negative")]
}

impl sealed::Sealed for IntVar {}
impl Evaluate for IntVar {
    type Output = i64;

    fn evaluate(&self, solution: &[i64]) -> i64 {
        slot(solution, *self)
    }
}

impl sealed::Sealed for BoolVar {}
impl Evaluate for BoolVar {
    type Output = bool;

    fn evaluate(&self, solution: &[i64]) -> bool {
        (slot(solution, self.to_int_var()) != 0) != self.is_negated()
    }
}

impl sealed::Sealed for i64 {}
impl Evaluate for i64 {
    type Output = i64;

    fn evaluate(&self, _: &[i64]) -> i64 {
        *self
    }
}

impl sealed::Sealed for LinearExpr {}
impl Evaluate for LinearExpr {
    type Output = i64;

    fn evaluate(&self, solution: &[i64]) -> i64 {
        self.value_in(solution)
    }
}

impl sealed::Sealed for f64 {}
impl Evaluate for f64 {
    type Output = f64;

    fn evaluate(&self, _: &[i64]) -> f64 {
        *self
    }
}

impl sealed::Sealed for FloatExpr {}
impl Evaluate for FloatExpr {
    type Output = f64;

    fn evaluate(&self, solution: &[i64]) -> f64 {
        self.value_in(solution)
    }
}

impl<T: Evaluate> sealed::Sealed for &T {}
impl<T: Evaluate> Evaluate for &T {
    type Output = T::Output;

    fn evaluate(&self, solution: &[i64]) -> T::Output {
        T::evaluate(self, solution)
    }
}

impl CpSolverResponse {
    /// The status, with values from a newer schema read as `UNKNOWN`.
    #[doc(alias("status_name"))]
    #[must_use]
    pub fn status(&self) -> CpSolverStatus {
        self.status.as_known().unwrap_or(CpSolverStatus::UNKNOWN)
    }

    /// The solution when the status is `OPTIMAL` or `FEASIBLE`.
    #[must_use]
    pub fn solution(&self) -> Option<Solution<'_>> {
        self.status()
            .has_solution()
            .then_some(Solution(&self.solution))
    }

    /// Each entry of `additional_solutions`, which OR-Tools fills when
    /// `fill_additional_solutions_in_response` is set.
    pub fn additional_solutions(&self) -> impl Iterator<Item = Solution<'_>> {
        self.additional_solutions
            .iter()
            .map(|solution| Solution(&solution.values))
    }
}

#[cfg(feature = "views")]
impl CpSolverResponseView<'_> {
    /// The status, with values from a newer schema read as `UNKNOWN`.
    #[must_use]
    pub fn status(&self) -> CpSolverStatus {
        self.status.as_known().unwrap_or(CpSolverStatus::UNKNOWN)
    }

    /// The solution when the status is `OPTIMAL` or `FEASIBLE`. See
    /// [`CpSolverResponse::solution`].
    #[must_use]
    pub fn solution(&self) -> Option<Solution<'_>> {
        self.status()
            .has_solution()
            .then_some(Solution(&self.solution))
    }
}

/// A [`CpSolverResponse`] that a solution callback borrows from OR-Tools,
/// like the response behind Python's `CpSolverSolutionCallback`. Each method
/// reads the response field of the same name, and
/// [`CpSolverResponseRef::to_owned_message`] copies the whole response.
#[doc(alias("CpSolverSolutionCallback", "SolutionCallback"))]
#[derive(Clone, Copy, Debug)]
pub struct CpSolverResponseRef<'a>(&'a CpsatResponseRef);

impl<'a> CpSolverResponseRef<'a> {
    pub(crate) fn new(response: &'a CpsatResponseRef) -> Self {
        Self(response)
    }

    /// The status, with values from a newer schema read as `UNKNOWN`.
    #[must_use]
    pub fn status(self) -> CpSolverStatus {
        CpSolverStatus::from_i32(self.0.status).unwrap_or(CpSolverStatus::UNKNOWN)
    }

    /// The solution when the status is `OPTIMAL` or `FEASIBLE`, borrowed from
    /// OR-Tools without a copy. See [`CpSolverResponse::solution`].
    #[must_use]
    pub fn solution(self) -> Option<Solution<'a>> {
        // SAFETY: the bridge points solution at solution_len values that live
        // as long as the borrowed response.
        let values = unsafe { crate::solver::ffi::slice(self.0.solution, self.0.solution_len) };
        self.status().has_solution().then_some(Solution(values))
    }

    /// The objective value.
    #[doc(alias("ObjectiveValue"))]
    #[must_use]
    pub fn objective_value(self) -> f64 {
        self.0.objective_value
    }

    /// The best proven objective bound.
    #[doc(alias("BestObjectiveBound"))]
    #[must_use]
    pub fn best_objective_bound(self) -> f64 {
        self.0.best_objective_bound
    }

    /// The number of Boolean variables in the search.
    #[doc(alias("NumBooleans"))]
    #[must_use]
    pub fn num_booleans(self) -> i64 {
        self.0.num_booleans
    }

    /// The number of conflicts so far.
    #[doc(alias("NumConflicts"))]
    #[must_use]
    pub fn num_conflicts(self) -> i64 {
        self.0.num_conflicts
    }

    /// The number of search branches so far.
    #[doc(alias("NumBranches"))]
    #[must_use]
    pub fn num_branches(self) -> i64 {
        self.0.num_branches
    }

    /// The number of Boolean propagations so far.
    #[doc(alias("NumBinaryPropagations"))]
    #[must_use]
    pub fn num_binary_propagations(self) -> i64 {
        self.0.num_binary_propagations
    }

    /// The number of integer propagations so far.
    #[doc(alias("NumIntegerPropagations"))]
    #[must_use]
    pub fn num_integer_propagations(self) -> i64 {
        self.0.num_integer_propagations
    }

    /// The wall time in seconds since the solve started.
    #[doc(alias("WallTime"))]
    #[must_use]
    pub fn wall_time(self) -> f64 {
        self.0.wall_time
    }

    /// The user time in seconds since the solve started.
    #[doc(alias("UserTime"))]
    #[must_use]
    pub fn user_time(self) -> f64 {
        self.0.user_time
    }

    /// The deterministic time since the solve started.
    #[doc(alias("DeterministicTime"))]
    #[must_use]
    pub fn deterministic_time(self) -> f64 {
        self.0.deterministic_time
    }

    /// Copies the whole response, including the fields without a method here.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge fails to serialize the response.
    #[doc(alias("Response", "response_proto"))]
    pub fn to_owned_message(self) -> Result<CpSolverResponse> {
        let mut response = Buffer::new();
        let mut error = Buffer::new();
        // SAFETY: the response is borrowed during its callback, and both
        // buffers are empty bridge buffers.
        let code = unsafe {
            cpsat_sys::cpsat_response_ref_serialize(
                self.0,
                response.as_mut_ptr(),
                error.as_mut_ptr(),
            )
        };
        crate::solver::ffi::check(code, &error)?;
        Ok(CpSolverResponse::decode_from_slice(response.bytes())?)
    }
}
