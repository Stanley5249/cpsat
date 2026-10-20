//! The modeling API, like Python's `CpModel`.

mod interval;
mod lookup;
mod objective;
mod var;

pub use interval::Interval;
pub use objective::Objective;
pub use var::{Bool, Named, NewVar, WithName};

use std::collections::BTreeMap;

use buffa::MessageField;

use crate::constraint::{Constraint, ConstraintIndex};
use crate::error::Result;
use crate::expr::{BoolVar, Domain, IntVar, LinearExpr};
use crate::proto::decision_strategy_proto::{DomainReductionStrategy, VariableSelectionStrategy};
use crate::proto::{
    CpModelProto, CpSolverResponse, DecisionStrategyProto, IntegerVariableProto, SatParameters,
};

mod sealed {
    pub trait Sealed {}
}

/// What `add_hint` accepts, and the type of its value.
pub trait Hint: sealed::Sealed {
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
        (self.variable().index(), (value != self.is_negated()).into())
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
/// use cpsat::proto::{CpSolverStatus, SatParameters};
///
/// let mut model = Model::new();
/// let x = model.new_var((0..=10).with_name("x"));
/// let y = model.new_var(0..=10);
/// model.add((x + 2 * y).le(14));
/// model.add(Constraint::all_different([x, y]));
/// model.maximize(x + y);
///
/// let response = model.solve(&SatParameters::default())?;
/// assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
/// assert_eq!(response.value(x + y), Some(12));
/// # Ok::<(), cpsat::Error>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct Model(CpModelProto, Constants);

/// The variable of each value that [`Model::constant`] created, so a value
/// reuses one variable like Python's constant map.
type Constants = BTreeMap<i64, i32>;

impl PartialEq for Model {
    /// Compares the messages, because the constant map only saves variables.
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Converts a handle into a `Vec` index, which panics for a negative index
/// like an index out of range does.
fn slot(index: i32) -> usize {
    usize::try_from(index).expect("a handle index is non-negative")
}

/// Converts a length into the `i32` index the schema uses.
fn next_index(len: usize, what: &str) -> i32 {
    i32::try_from(len).unwrap_or_else(|_| panic!("the schema indexes {what} with i32"))
}

impl Model {
    /// An empty model, like Python's `CpModel()`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn proto(&self) -> &CpModelProto {
        &self.0
    }

    /// The model's message, for anything the high-level API does not cover.
    /// Removing variables or constraints here invalidates their handles.
    pub fn proto_mut(&mut self) -> &mut CpModelProto {
        &mut self.0
    }

    #[must_use]
    pub fn into_proto(self) -> CpModelProto {
        self.0
    }

    /// Solves the model, like Python's `CpSolver().solve(model)`. See
    /// [`CpModelProto::solve`].
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge fails.
    pub fn solve(&self, parameters: &SatParameters) -> Result<CpSolverResponse> {
        self.0.solve(parameters)
    }

    /// Validates the model, like Python's `validate`. See
    /// [`CpModelProto::validate`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidModel`](crate::Error::InvalidModel) when the
    /// model is invalid.
    pub fn validate(&self) -> Result<()> {
        self.0.validate()
    }

    /// OR-Tools' statistics for the model, like Python's `model_stats`.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge call fails.
    pub fn stats(&self) -> Result<String> {
        self.0.stats()
    }

    /// A variable or an interval, like Python's `new_int_var`,
    /// `new_int_var_from_domain`, `new_bool_var`, and `new_*interval_var`:
    /// an [`IntVar`] for a domain or a bounded range, a [`BoolVar`] for
    /// [`Bool`], and an [`IntervalVar`](crate::prelude::IntervalVar) for an
    /// [`Interval`]. [`WithName::with_name`] names a variable.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// let mut model = Model::new();
    /// let x = model.new_var(0..=10);
    /// let y = model.new_var(Domain::from_values([1, 3, 5]).with_name("y"));
    /// let b = model.new_var(Bool);
    /// let task = model.new_var(Interval::new(x, y, x + y).only_enforce_if(b));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the model already holds `i32::MAX` variables or
    /// constraints.
    pub fn new_var<V: NewVar>(&mut self, var: V) -> V::Output {
        var.insert(self)
    }

    /// Appends a variable and returns its index.
    fn push_variable(&mut self, domain: Vec<i64>, name: String) -> i32 {
        let index = next_index(self.0.variables.len(), "variables");
        self.0.variables.push(IntegerVariableProto {
            name,
            domain,
            ..Default::default()
        });
        index
    }

    /// Replaces the domain of `var`, like Python's `IntVar.with_domain`, for
    /// example to tighten it before solving again. A literal of `var` stays
    /// valid only while the domain stays within `[0, 1]`, which
    /// [`validate`](Self::validate) checks.
    ///
    /// # Panics
    ///
    /// Panics if the model lacks `var`, like indexing with it.
    pub fn set_domain(&mut self, var: IntVar, domain: impl Into<Domain>) {
        self.0.variables[slot(var.0)].domain = domain.into().flattened_intervals();
    }

    /// A variable fixed to `value`, like Python's `new_constant`. Calls with
    /// one value share one variable while its domain stays `[value, value]`.
    ///
    /// Use it where the schema needs a variable, such as in
    /// [`Constraint::inverse`]. An expression takes a plain `i64` or
    /// [`LinearExpr::constant`], which adds no variable.
    ///
    /// # Panics
    ///
    /// Panics if the model already holds `i32::MAX` variables.
    pub fn constant(&mut self, value: i64) -> IntVar {
        if let Some(&index) = self.1.get(&value)
            && self
                .0
                .variables
                .get(slot(index))
                .is_some_and(|variable| variable.domain == [value, value])
        {
            return IntVar(index);
        }
        let index = self.push_variable(vec![value, value], String::new());
        self.1.insert(value, index);
        IntVar(index)
    }

    /// A literal fixed to `value`, which Python writes as `True` or `False`.
    /// Use it where the schema needs a literal, such as in
    /// [`Constraint::only_enforce_if`] or [`Constraint::bool_or`].
    ///
    /// # Panics
    ///
    /// Panics if the model already holds `i32::MAX` variables.
    pub fn bool_constant(&mut self, value: bool) -> BoolVar {
        BoolVar(self.constant(value.into()).0)
    }

    /// Inserts a constraint, like Python's `add` and `add_*` methods.
    ///
    /// # Panics
    ///
    /// Panics if the model already holds `i32::MAX` constraints.
    pub fn add(&mut self, constraint: impl Into<Constraint>) -> ConstraintIndex {
        let index = next_index(self.0.constraints.len(), "constraints");
        self.0.constraints.push(constraint.into().into_proto());
        ConstraintIndex(index)
    }

    /// Hints that `var` has `value`, like Python's `add_hint`: an `i64` for
    /// an integer variable and a `bool` for a literal.
    pub fn add_hint<H: Hint>(&mut self, var: H, value: H::Value) {
        let (var, value) = var.hint(value);
        let hint = self.0.solution_hint.get_or_insert_default();
        hint.vars.push(var);
        hint.values.push(value);
    }

    /// Removes every hint, like Python's `clear_hints`.
    pub fn clear_hints(&mut self) {
        self.0.solution_hint = MessageField::none();
    }

    /// Solves under the assumption that each literal is true, like Python's
    /// `add_assumption` and `add_assumptions`. An infeasible response lists a
    /// subset of the failed assumptions in
    /// `sufficient_assumptions_for_infeasibility`.
    pub fn add_assumptions(&mut self, literals: impl IntoIterator<Item = BoolVar>) {
        self.0
            .assumptions
            .extend(literals.into_iter().map(BoolVar::index));
    }

    /// Removes every assumption, like Python's `clear_assumptions`.
    pub fn clear_assumptions(&mut self) {
        self.0.assumptions.clear();
    }

    /// Adds a search strategy over `exprs`, like Python's
    /// `add_decision_strategy`. A literal `!b` becomes the expression `1 - b`,
    /// as in Python.
    pub fn add_decision_strategy(
        &mut self,
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
        variable_selection: VariableSelectionStrategy,
        domain_reduction: DomainReductionStrategy,
    ) {
        self.0.search_strategy.push(DecisionStrategyProto {
            exprs: exprs.into_iter().map(|expr| expr.into().into()).collect(),
            variable_selection_strategy: variable_selection.into(),
            domain_reduction_strategy: domain_reduction.into(),
            ..Default::default()
        });
    }

    /// Clears the names of the model, its variables, and its constraints, like
    /// Python's `remove_all_names`.
    pub fn remove_all_names(&mut self) {
        self.0.name.clear();
        for variable in &mut self.0.variables {
            variable.name.clear();
        }
        for constraint in &mut self.0.constraints {
            constraint.name.clear();
        }
    }
}

impl From<CpModelProto> for Model {
    /// Wraps an existing model, like Python's `CpModel(model_proto)`. Like
    /// Python, the first variable fixed to each value becomes that value's
    /// constant.
    fn from(proto: CpModelProto) -> Self {
        let mut constants = Constants::new();
        for (index, variable) in proto.variables.iter().enumerate() {
            if let &[lower, upper] = variable.domain.as_slice()
                && lower == upper
            {
                constants
                    .entry(lower)
                    .or_insert_with(|| next_index(index, "variables"));
            }
        }
        Self(proto, constants)
    }
}

impl AsRef<CpModelProto> for Model {
    fn as_ref(&self) -> &CpModelProto {
        &self.0
    }
}

impl AsRef<Self> for CpModelProto {
    /// Lets [`Solver::solve`](crate::prelude::Solver::solve) take a model or its
    /// message.
    fn as_ref(&self) -> &Self {
        self
    }
}

impl From<Model> for CpModelProto {
    fn from(model: Model) -> Self {
        model.0
    }
}
