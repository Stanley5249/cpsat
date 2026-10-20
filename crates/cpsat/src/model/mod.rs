//! The modeling API, like Python's `CpModel`.

mod interval;
mod lookup;
mod objective;
mod var;

pub use interval::Interval;
pub use objective::Objective;
pub use var::{Bool, Named, NewVar, VarMut, WithName};

use buffa::MessageField;

use crate::constraint::{Constraint, ConstraintIndex};
use crate::error::Result;
use crate::expr::{BoolVar, IntVar, LinearExpr};
use crate::proto::decision_strategy_proto::{DomainReductionStrategy, VariableSelectionStrategy};
use crate::proto::{
    CpModelProto, CpSolverResponse, DecisionStrategyProto, IntegerVariableProto, SatParameters,
};

mod sealed {
    pub trait Sealed {}
}

/// What `add_hint` accepts, and the type of its value.
pub trait Hint: sealed::Sealed {
    /// The type of the hinted value: `i64` or `bool`.
    type Value;

    /// The variable index and the value the schema stores.
    #[doc(hidden)]
    fn hint(self, value: Self::Value) -> (i32, i64);
}

impl sealed::Sealed for IntVar {}
impl Hint for IntVar {
    type Value = i64;

    fn hint(self, value: i64) -> (i32, i64) {
        (self.index(), value)
    }
}

impl sealed::Sealed for BoolVar {}
impl Hint for BoolVar {
    type Value = bool;

    /// A negated literal hints its variable with the opposite value.
    fn hint(self, value: bool) -> (i32, i64) {
        (
            self.to_int_var().index(),
            (value != self.is_negated()).into(),
        )
    }
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

/// Converts a handle into a `Vec` index, which panics for a negative index
/// like an index out of range does.
fn slot(index: i32) -> usize {
    usize::try_from(index).expect("a handle index is non-negative")
}

/// Converts a length into the `i32` index the schema uses.
fn next_index(len: usize, what: &str) -> i32 {
    i32::try_from(len).unwrap_or_else(|_| panic!("the schema indexes {what} with i32"))
}

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

    /// A variable or an interval: an [`IntVar`] for a domain or a bounded
    /// range, a [`BoolVar`] for [`Bool`], a [`BoolVar`] fixed to the value of
    /// a `bool`, and an [`IntervalVar`](crate::prelude::IntervalVar) for an
    /// [`Interval`]. [`WithName::with_name`] names a variable.
    ///
    /// A fixed variable, such as `5..=5` or `true`, serves where the schema
    /// needs a variable or a literal, such as in [`Constraint::inverse`] or
    /// [`Constraint::bool_or`]. An expression takes a plain `i64` or
    /// [`LinearExpr::constant`], which adds no variable.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// // From `interval_sample_sat.py`.
    /// let mut model = CpModel::new();
    /// let start = model.new_var((0..=100).with_name("start"));
    /// let end = model.new_var((0..=100).with_name("end"));
    /// let interval = model.new_var(Interval::new(start, 10, end + 2).with_name("interval"));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the model already holds more than `i32::MAX` variables or
    /// constraints.
    #[doc(alias(
        "new_int_var",
        "new_int_var_from_domain",
        "new_constant",
        "NewIntVar",
        "NewConstant",
        "TrueVar",
        "FalseVar"
    ))]
    pub fn new_var<V: NewVar>(&mut self, var: V) -> V::Output {
        var.insert(self)
    }

    /// Appends a variable and returns its index.
    fn push_var(&mut self, domain: Vec<i64>, name: String) -> i32 {
        let index = next_index(self.0.variables.len(), "variables");
        self.0.variables.push(IntegerVariableProto {
            name,
            domain,
            ..Default::default()
        });
        index
    }

    /// Inserts a constraint.
    ///
    /// # Panics
    ///
    /// Panics if the model already holds more than `i32::MAX` constraints.
    pub fn add(&mut self, constraint: impl Into<Constraint>) -> ConstraintIndex {
        let index = next_index(self.0.constraints.len(), "constraints");
        self.0.constraints.push(constraint.into().into_proto());
        ConstraintIndex(index)
    }

    /// Hints that `var` has `value`: an `i64` for an integer variable and a
    /// `bool` for a literal.
    #[doc(alias("AddHint"))]
    pub fn add_hint<H: Hint>(&mut self, var: H, value: H::Value) {
        let (var, value) = var.hint(value);
        let hint = self.0.solution_hint.get_or_insert_default();
        hint.vars.push(var);
        hint.values.push(value);
    }

    /// Removes every hint.
    #[doc(alias("ClearHints"))]
    pub fn clear_hints(&mut self) {
        self.0.solution_hint = MessageField::none();
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
            exprs: exprs
                .into_iter()
                .map(Into::into)
                .map(LinearExpr::into_proto)
                .collect(),
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
