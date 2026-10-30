//! The modeling API, like Python's `CpModel`.

mod hint;
mod interval;
mod lookup;
mod objective;
mod var;

pub use hint::Hint;
pub use interval::Interval;
pub use objective::Objective;
pub use var::{Bool, Named, NewVar, VarMut, WithName};

use lookup::next_index;

use crate::constraint::{Constraint, ConstraintIndex};
use crate::error::Result;
use crate::expr::{BoolVar, IntVar, IntervalVar, LinearExpr};
use crate::proto::decision_strategy_proto::{DomainReductionStrategy, VariableSelectionStrategy};
use crate::proto::{
    ConstraintProto, CpModelProto, CpSolverResponse, DecisionStrategyProto, SatParameters,
};

mod sealed {
    pub trait Sealed {}

    /// Builds a handle from the index that `CpModel` allocated. Code outside
    /// the crate cannot name this trait, so only the model creates handles.
    pub trait FromIndex {
        fn from_index(index: i32) -> Self;
    }

    /// Lowers a `NewVar` input to the row that `CpModel::new_var` appends.
    /// Generic code bound on `NewVar` can still call it, but it only builds
    /// a message, never a handle.
    pub trait LowerVar {
        fn into_proto(self) -> crate::proto::IntegerVariableProto;
    }
}

impl sealed::FromIndex for IntVar {
    fn from_index(index: i32) -> Self {
        Self(index)
    }
}

impl sealed::FromIndex for BoolVar {
    fn from_index(index: i32) -> Self {
        Self(index)
    }
}

impl sealed::FromIndex for IntervalVar {
    fn from_index(index: i32) -> Self {
        Self(index)
    }
}

impl sealed::FromIndex for ConstraintIndex {
    fn from_index(index: i32) -> Self {
        Self(index)
    }
}

/// What [`CpModel::add`] inserts, and the handle it returns: a
/// [`ConstraintIndex`] for a [`Constraint`] or a `ConstraintProto`, and an
/// [`IntervalVar`] for an [`Interval`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be added as a constraint",
    label = "expected a `Constraint`, an `Interval`, or a `ConstraintProto`"
)]
pub trait AddConstraint: Into<ConstraintProto> + sealed::Sealed {
    /// The handle that [`CpModel::add`] returns.
    type Handle: sealed::FromIndex;
}

impl sealed::Sealed for Constraint {}
impl AddConstraint for Constraint {
    type Handle = ConstraintIndex;
}

impl sealed::Sealed for ConstraintProto {}
impl AddConstraint for ConstraintProto {
    type Handle = ConstraintIndex;
}

/// A CP-SAT model, which owns its `CpModelProto` and hands out typed handles.
///
/// The model only appends variables and constraints, so a handle stays valid
/// unless the caller removes entries through [`proto_mut`](Self::proto_mut).
/// Indexing with a handle returns its Protobuf message and panics like a
/// slice when the model lacks the handle.
///
/// ```
/// use cpsat::prelude::*;
///
/// // From `simple_sat_program.py`.
/// let mut model = CpModel::new();
/// let x = model.new_var((0..=2).with_name("x"));
/// let y = model.new_var((0..=2).with_name("y"));
/// let z = model.new_var((0..=2).with_name("z"));
/// model.add(x.ne(y));
/// ```
#[doc(alias("CpModelBuilder"))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CpModel(CpModelProto);

impl CpModel {
    /// An empty model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The model's message.
    #[doc(alias("Build"))]
    #[must_use]
    pub fn proto(&self) -> &CpModelProto {
        &self.0
    }

    /// The model's message, for anything the high-level API does not cover.
    /// Removing variables or constraints here invalidates their handles.
    #[doc(alias("MutableProto"))]
    pub fn proto_mut(&mut self) -> &mut CpModelProto {
        &mut self.0
    }

    /// Unwraps the model's message.
    #[must_use]
    pub fn into_proto(self) -> CpModelProto {
        self.0
    }

    /// Solves the model. See [`CpModelProto::solve`].
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge fails.
    #[doc(alias("SolveWithParameters"))]
    pub fn solve(&self, parameters: &SatParameters) -> Result<CpSolverResponse> {
        self.0.solve(parameters)
    }

    /// Validates the model. See [`CpModelProto::validate`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidModel`](crate::Error::InvalidModel) when the
    /// model is invalid.
    #[doc(alias("ValidateCpModel"))]
    pub fn validate(&self) -> Result<()> {
        self.0.validate()
    }

    /// OR-Tools' statistics for the model.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge call fails.
    #[doc(alias("model_stats", "CpModelStats"))]
    pub fn stats(&self) -> Result<String> {
        self.0.stats()
    }

    /// A variable: an [`IntVar`](crate::prelude::IntVar) for a domain or a
    /// bounded range, a [`BoolVar`] for [`Bool`], and a [`BoolVar`] fixed to
    /// the value of a `bool`. [`WithName::with_name`] names it.
    ///
    /// A fixed variable, such as `5..=5` or `true`, serves where the schema
    /// needs a variable or a literal, such as in [`Constraint::inverse`] or
    /// [`Constraint::bool_or`]. An expression takes a plain `i64` or
    /// [`LinearExpr::constant`], which adds no variable.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// // From `literal_sample_sat.py`.
    /// let mut model = CpModel::new();
    /// let x = model.new_var(Bool.with_name("x"));
    /// let not_x = !x;
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the model already holds more than `i32::MAX` variables.
    #[doc(alias(
        "new_int_var",
        "new_int_var_from_domain",
        "new_constant",
        "NewIntVar",
        "NewConstant",
        "TrueVar",
        "FalseVar"
    ))]
    pub fn new_var<V: NewVar>(&mut self, var: V) -> V::Handle {
        let index = next_index(self.0.variables.len(), "variables");
        self.0.variables.push(sealed::LowerVar::into_proto(var));
        <V::Handle as sealed::FromIndex>::from_index(index)
    }

    /// Inserts a constraint and returns its handle: a [`ConstraintIndex`] for
    /// a [`Constraint`] or a `ConstraintProto`, and an [`IntervalVar`] for an
    /// [`Interval`], which scheduling constraints take as input.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// // From `interval_sample_sat.py`.
    /// let mut model = CpModel::new();
    /// let start = model.new_var((0..=100).with_name("start"));
    /// let end = model.new_var((0..=100).with_name("end"));
    /// let interval = model.add(Interval::new(start, 10, end + 2).with_name("interval"));
    /// model.add(Constraint::no_overlap([interval]));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the model already holds more than `i32::MAX` constraints.
    pub fn add<C: AddConstraint>(&mut self, constraint: C) -> C::Handle {
        let index = next_index(self.0.constraints.len(), "constraints");
        self.0.constraints.push(constraint.into());
        <C::Handle as sealed::FromIndex>::from_index(index)
    }

    /// Solves under the assumption that each literal is true. An infeasible
    /// response lists a subset of the failed assumptions in
    /// `sufficient_assumptions_for_infeasibility`.
    #[doc(alias("add_assumption", "AddAssumptions", "AddAssumption"))]
    pub fn add_assumptions(&mut self, literals: impl IntoIterator<Item = BoolVar>) {
        self.0
            .assumptions
            .extend(literals.into_iter().map(BoolVar::index));
    }

    /// Removes every assumption.
    #[doc(alias("ClearAssumptions"))]
    pub fn clear_assumptions(&mut self) {
        self.0.assumptions.clear();
    }

    /// Adds a search strategy over `exprs`. A literal `!b` becomes the
    /// expression `1 - b`, as in Python.
    #[doc(alias("AddDecisionStrategy"))]
    pub fn add_decision_strategy(
        &mut self,
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
        variable_selection: VariableSelectionStrategy,
        domain_reduction: DomainReductionStrategy,
    ) {
        self.0.search_strategy.push(DecisionStrategyProto {
            exprs: LinearExpr::protos(exprs),
            variable_selection_strategy: variable_selection.into(),
            domain_reduction_strategy: domain_reduction.into(),
            ..Default::default()
        });
    }

    /// Clears the names of the model, its variables, and its constraints.
    #[doc(alias("remove_all_names"))]
    pub fn clear_names(&mut self) {
        self.0.name.clear();
        for var in &mut self.0.variables {
            var.name.clear();
        }
        for constraint in &mut self.0.constraints {
            constraint.name.clear();
        }
    }
}

impl From<CpModelProto> for CpModel {
    /// Wraps an existing model, like Python's `CpModel(model_proto)`.
    fn from(proto: CpModelProto) -> Self {
        Self(proto)
    }
}

impl AsRef<CpModelProto> for CpModel {
    fn as_ref(&self) -> &CpModelProto {
        &self.0
    }
}

impl AsRef<Self> for CpModelProto {
    /// Lets [`CpSolver::solve`](crate::prelude::CpSolver::solve) take a model or its
    /// message.
    fn as_ref(&self) -> &Self {
        self
    }
}

impl From<CpModel> for CpModelProto {
    fn from(model: CpModel) -> Self {
        model.0
    }
}
