//! The modeling API, like Python's `CpModel`.

mod lookup;

use buffa::MessageField;

use crate::constraint::{Constraint, ConstraintRef};
use crate::domain::Domain;
use crate::expr::LinearExpr;
use crate::proto::decision_strategy_proto::{DomainReductionStrategy, VariableSelectionStrategy};
use crate::proto::{
    ConstraintProto, CpModelProto, CpObjectiveProto, DecisionStrategyProto, IntegerVariableProto,
    IntervalConstraintProto,
};
use crate::variable::{BoolVar, IntVar, IntervalVar};

/// A CP-SAT model, which owns its `CpModelProto` and hands out typed handles.
///
/// The model only appends variables and constraints, so a handle stays valid
/// unless the caller removes entries through [`proto_mut`](Self::proto_mut).
/// Indexing with a handle returns its Protobuf message, for example to set a
/// name.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let x = model.int_var(0..=10);
/// let y = model.int_var(0..=10);
/// model[x].name = "x".into();
/// model.add((x + 2 * y).le(14));
/// model.add(Constraint::all_different([x, y]));
/// model.maximize(x + y);
///
/// let response = solve(model.proto(), &SatParameters::default())?;
/// assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
/// assert_eq!(response.value(x + y), Some(12));
/// # Ok::<(), cpsat::Error>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model(CpModelProto);

/// Converts a length into the `i32` index the schema uses.
fn next_index(len: usize, what: &str) -> i32 {
    i32::try_from(len).unwrap_or_else(|_| panic!("the schema indexes {what} with i32"))
}

impl Model {
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

    /// An integer variable in `domain`, like Python's `new_int_var` and
    /// `new_int_var_from_domain`.
    ///
    /// # Panics
    ///
    /// Panics if the model already holds `i32::MAX` variables.
    pub fn int_var(&mut self, domain: impl Into<Domain>) -> IntVar {
        let index = next_index(self.0.variables.len(), "variables");
        self.0.variables.push(IntegerVariableProto {
            domain: domain.into().into(),
            ..Default::default()
        });
        IntVar(index)
    }

    /// A 0-1 variable, like Python's `new_bool_var`.
    pub fn bool_var(&mut self) -> BoolVar {
        BoolVar(self.int_var(0..=1).0)
    }

    /// A variable fixed to `value`, like Python's `new_constant`. Each call
    /// adds a variable, where Python reuses one per value.
    ///
    /// Use it where the schema needs a variable, such as in
    /// [`Constraint::inverse`]. An expression takes a plain `i64` or
    /// [`LinearExpr::constant`], which adds no variable.
    pub fn constant(&mut self, value: i64) -> IntVar {
        self.int_var(value..=value)
    }

    /// A literal fixed to `value`, which Python writes as `True` or `False`.
    /// Use it where the schema needs a literal, such as in
    /// [`Constraint::only_if`] or [`Constraint::bool_or`].
    pub fn bool_constant(&mut self, value: bool) -> BoolVar {
        BoolVar(self.constant(value.into()).0)
    }

    /// An interval with `start + size == end`, like Python's
    /// `new_interval_var`. CP-SAT accepts affine expressions here.
    pub fn interval_var(
        &mut self,
        start: impl Into<LinearExpr>,
        size: impl Into<LinearExpr>,
        end: impl Into<LinearExpr>,
    ) -> IntervalVar {
        self.interval(start.into(), size.into(), end.into(), None)
    }

    /// An interval of a fixed size, like Python's
    /// `new_fixed_size_interval_var`.
    pub fn fixed_size_interval_var(
        &mut self,
        start: impl Into<LinearExpr>,
        size: i64,
    ) -> IntervalVar {
        let start = start.into();
        let end = start.clone() + size;
        self.interval(start, size.into(), end, None)
    }

    /// An interval that other constraints ignore when `present` is false, like
    /// Python's `new_optional_interval_var`.
    pub fn optional_interval_var(
        &mut self,
        start: impl Into<LinearExpr>,
        size: impl Into<LinearExpr>,
        end: impl Into<LinearExpr>,
        present: BoolVar,
    ) -> IntervalVar {
        self.interval(start.into(), size.into(), end.into(), Some(present))
    }

    /// An optional interval of a fixed size, like Python's
    /// `new_optional_fixed_size_interval_var`.
    pub fn optional_fixed_size_interval_var(
        &mut self,
        start: impl Into<LinearExpr>,
        size: i64,
        present: BoolVar,
    ) -> IntervalVar {
        let start = start.into();
        let end = start.clone() + size;
        self.interval(start, size.into(), end, Some(present))
    }

    fn interval(
        &mut self,
        start: LinearExpr,
        size: LinearExpr,
        end: LinearExpr,
        present: Option<BoolVar>,
    ) -> IntervalVar {
        let interval = IntervalConstraintProto {
            start: MessageField::some(start.into()),
            end: MessageField::some(end.into()),
            size: MessageField::some(size.into()),
            ..Default::default()
        };
        let mut constraint = Constraint::from(ConstraintProto {
            constraint: Some(interval.into()),
            ..Default::default()
        });
        if let Some(present) = present {
            constraint = constraint.only_if(present);
        }
        IntervalVar(self.add(constraint).0)
    }

    /// Inserts a constraint, like Python's `add` and `add_*` methods.
    ///
    /// # Panics
    ///
    /// Panics if the model already holds `i32::MAX` constraints.
    pub fn add(&mut self, constraint: impl Into<Constraint>) -> ConstraintRef {
        let index = next_index(self.0.constraints.len(), "constraints");
        self.0.constraints.push(constraint.into().into_proto());
        ConstraintRef(index)
    }

    /// Minimizes `expr`, replacing any objective.
    pub fn minimize(&mut self, expr: impl Into<LinearExpr>) {
        self.set_objective(expr.into(), 1);
    }

    /// Maximizes `expr`, replacing any objective. Like Python, this minimizes
    /// `-expr` with a scaling factor of -1.
    pub fn maximize(&mut self, expr: impl Into<LinearExpr>) {
        self.set_objective(-expr.into(), -1);
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "the schema stores the offset as a double, like Python"
    )]
    fn set_objective(&mut self, expr: LinearExpr, scaling: i8) {
        self.clear_objective();
        let (vars, coeffs, offset) = expr.into_terms();
        self.0.objective = MessageField::some(CpObjectiveProto {
            vars,
            coeffs,
            offset: offset as f64,
            scaling_factor: scaling.into(),
            ..Default::default()
        });
    }

    #[must_use]
    pub fn has_objective(&self) -> bool {
        self.0.objective.is_set() || self.0.floating_point_objective.is_set()
    }

    /// Removes the integer and the floating-point objectives.
    pub fn clear_objective(&mut self) {
        self.0.objective = MessageField::none();
        self.0.floating_point_objective = MessageField::none();
    }

    /// Hints `var == value` to the solver, like Python's `add_hint`.
    pub fn add_hint(&mut self, var: IntVar, value: i64) {
        let hint = self.0.solution_hint.get_or_insert_default();
        hint.vars.push(var.0);
        hint.values.push(value);
    }

    /// Hints that `literal` has `value`, like Python's `add_hint` on a literal.
    pub fn add_bool_hint(&mut self, literal: BoolVar, value: bool) {
        self.add_hint(literal.variable(), (value != literal.is_negated()).into());
    }

    /// Removes every hint, like Python's `clear_hints`.
    pub fn clear_hints(&mut self) {
        self.0.solution_hint = MessageField::none();
    }

    /// Solves under the assumption that `literal` is true. An infeasible
    /// response lists a subset of the failed assumptions in
    /// `sufficient_assumptions_for_infeasibility`.
    pub fn add_assumption(&mut self, literal: BoolVar) {
        self.0.assumptions.push(literal.0);
    }

    /// Adds each literal as an assumption, like Python's `add_assumptions`.
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

    /// Clears the names of the model, its variables, and its constraints.
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
    /// Wraps an existing model, like Python's `CpModel(model_proto)`.
    fn from(proto: CpModelProto) -> Self {
        Self(proto)
    }
}

impl From<Model> for CpModelProto {
    fn from(model: Model) -> Self {
        model.0
    }
}
