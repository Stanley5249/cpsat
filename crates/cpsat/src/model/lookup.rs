//! Handle lookups: indexing with a handle, reading an interval, and finding
//! a handle by its schema index.

use std::ops::{Index, IndexMut};

use super::{Model, slot};
use crate::constraint::ConstraintRef;
use crate::domain::Domain;
use crate::expr::LinearExpr;
use crate::proto::constraint_proto::Constraint as Kind;
use crate::proto::{ConstraintProto, IntegerVariableProto, IntervalConstraintProto};
use crate::variable::{BoolVar, IntVar, IntervalVar};

impl Model {
    /// The variable at `index`, like Python's `get_int_var_from_proto_index`.
    #[must_use]
    pub fn int_var_from_index(&self, index: i32) -> Option<IntVar> {
        self.variable(index).map(|_| IntVar(index))
    }

    /// The 0-1 variable at `index`, like Python's
    /// `get_bool_var_from_proto_index`, or `None` if its domain is not within
    /// `[0, 1]`.
    #[must_use]
    pub fn bool_var_from_index(&self, index: i32) -> Option<BoolVar> {
        let variable = self.variable(index)?;
        let domain = Domain::from_flat_intervals(&variable.domain)?;
        let within = domain
            .intervals()
            .iter()
            .all(|&(lower, upper)| lower >= 0 && upper <= 1);
        (within && !domain.is_empty()).then_some(BoolVar(index))
    }

    /// The interval at constraint `index`, like Python's
    /// `get_interval_var_from_proto_index`.
    #[must_use]
    pub fn interval_var_from_index(&self, index: i32) -> Option<IntervalVar> {
        let constraint = self.0.constraints.get(usize::try_from(index).ok()?)?;
        matches!(constraint.constraint, Some(Kind::Interval(_))).then_some(IntervalVar(index))
    }

    /// The start of `interval`, like Python's `IntervalVar.start_expr`.
    ///
    /// # Panics
    ///
    /// If `interval` is not an interval of this model.
    #[must_use]
    pub fn interval_start(&self, interval: IntervalVar) -> LinearExpr {
        LinearExpr::from((*self.interval_proto(interval).start).clone())
    }

    /// The size of `interval`, like Python's `IntervalVar.size_expr`.
    ///
    /// # Panics
    ///
    /// If `interval` is not an interval of this model.
    #[must_use]
    pub fn interval_size(&self, interval: IntervalVar) -> LinearExpr {
        LinearExpr::from((*self.interval_proto(interval).size).clone())
    }

    /// The end of `interval`, like Python's `IntervalVar.end_expr`.
    ///
    /// # Panics
    ///
    /// If `interval` is not an interval of this model.
    #[must_use]
    pub fn interval_end(&self, interval: IntervalVar) -> LinearExpr {
        LinearExpr::from((*self.interval_proto(interval).end).clone())
    }

    /// The literals that make `interval` present, none for an interval that is
    /// always present, like Python's `IntervalVar.presence_literals`.
    ///
    /// # Panics
    ///
    /// If `interval` is not an interval of this model.
    pub fn interval_presence_literals(
        &self,
        interval: IntervalVar,
    ) -> impl Iterator<Item = BoolVar> + '_ {
        self.interval_proto(interval);
        self[interval]
            .enforcement_literal
            .iter()
            .copied()
            .map(BoolVar)
    }

    fn interval_proto(&self, interval: IntervalVar) -> &IntervalConstraintProto {
        match &self[interval].constraint {
            Some(Kind::Interval(proto)) => proto,
            _ => panic!("{interval:?} is not an interval of this model"),
        }
    }

    fn variable(&self, index: i32) -> Option<&IntegerVariableProto> {
        self.0.variables.get(usize::try_from(index).ok()?)
    }
}

impl Index<IntVar> for Model {
    type Output = IntegerVariableProto;

    fn index(&self, var: IntVar) -> &IntegerVariableProto {
        &self.0.variables[slot(var.0)]
    }
}

impl IndexMut<IntVar> for Model {
    fn index_mut(&mut self, var: IntVar) -> &mut IntegerVariableProto {
        &mut self.0.variables[slot(var.0)]
    }
}

impl Index<BoolVar> for Model {
    type Output = IntegerVariableProto;

    /// The literal's variable, ignoring the negation.
    fn index(&self, literal: BoolVar) -> &IntegerVariableProto {
        &self[literal.variable()]
    }
}

impl IndexMut<BoolVar> for Model {
    fn index_mut(&mut self, literal: BoolVar) -> &mut IntegerVariableProto {
        &mut self[literal.variable()]
    }
}

impl Index<IntervalVar> for Model {
    type Output = ConstraintProto;

    fn index(&self, interval: IntervalVar) -> &ConstraintProto {
        &self.0.constraints[slot(interval.0)]
    }
}

impl IndexMut<IntervalVar> for Model {
    fn index_mut(&mut self, interval: IntervalVar) -> &mut ConstraintProto {
        &mut self.0.constraints[slot(interval.0)]
    }
}

impl Index<ConstraintRef> for Model {
    type Output = ConstraintProto;

    fn index(&self, constraint: ConstraintRef) -> &ConstraintProto {
        &self.0.constraints[slot(constraint.0)]
    }
}

impl IndexMut<ConstraintRef> for Model {
    fn index_mut(&mut self, constraint: ConstraintRef) -> &mut ConstraintProto {
        &mut self.0.constraints[slot(constraint.0)]
    }
}
