//! Handle lookups: indexing with a handle, reading an interval, and finding
//! a handle by its schema index.

use std::ops::Index;

use super::{CpModel, slot};
use crate::constraint::ConstraintIndex;
use crate::expr::{BoolVar, IntVar, IntervalVar, LinearExpr};
use crate::proto::constraint_proto::Constraint as Kind;
use crate::proto::{
    ConstraintProto, IntegerVariableProto, IntervalConstraintProto, LinearExpressionProto,
};

impl CpModel {
    /// The variable at `index`.
    #[doc(alias("get_int_var_from_proto_index", "GetIntVarFromProtoIndex"))]
    #[must_use]
    pub fn int_var_from_index(&self, index: i32) -> Option<IntVar> {
        self.variable(index).map(|_| IntVar(index))
    }

    /// `var` as a literal if its domain is within `[0, 1]`, for the places
    /// where Python accepts a 0-1 integer variable as a literal. With
    /// [`int_var_from_index`](Self::int_var_from_index), it replaces Python's
    /// `get_bool_var_from_proto_index`.
    #[doc(alias("get_bool_var_from_proto_index"))]
    #[must_use]
    pub fn to_literal(&self, var: IntVar) -> Option<BoolVar> {
        // Reads the flat field in place: a nonempty list of pairs with every
        // nonempty pair inside [0, 1].
        let (pairs, rest) = self.variable(var.0)?.domain.as_chunks::<2>();
        let within = pairs
            .iter()
            .all(|&[lower, upper]| lower > upper || (lower >= 0 && upper <= 1));
        let nonempty = pairs.iter().any(|&[lower, upper]| lower <= upper);
        (rest.is_empty() && within && nonempty).then_some(BoolVar(var.0))
    }

    /// The interval at constraint `index`.
    #[doc(alias("get_interval_var_from_proto_index", "GetIntervalVarFromProtoIndex"))]
    #[must_use]
    pub fn interval_var_from_index(&self, index: i32) -> Option<IntervalVar> {
        let constraint = self.0.constraints.get(usize::try_from(index).ok()?)?;
        matches!(constraint.constraint, Some(Kind::Interval(_))).then_some(IntervalVar(index))
    }

    /// The start of `interval`.
    ///
    /// # Panics
    ///
    /// If `interval` is not an interval of this model, or if its start has
    /// `vars` and `coeffs` of unequal lengths, which `validate` rejects.
    #[doc(alias("start_expr", "StartExpr"))]
    #[must_use]
    pub fn interval_start(&self, interval: IntervalVar) -> LinearExpr {
        interval_part(interval, &self.interval_proto(interval).start)
    }

    /// The size of `interval`.
    ///
    /// # Panics
    ///
    /// If `interval` is not an interval of this model, or if its size has
    /// `vars` and `coeffs` of unequal lengths, which `validate` rejects.
    #[doc(alias("size_expr", "SizeExpr"))]
    #[must_use]
    pub fn interval_size(&self, interval: IntervalVar) -> LinearExpr {
        interval_part(interval, &self.interval_proto(interval).size)
    }

    /// The end of `interval`.
    ///
    /// # Panics
    ///
    /// If `interval` is not an interval of this model, or if its end has
    /// `vars` and `coeffs` of unequal lengths, which `validate` rejects.
    #[doc(alias("end_expr", "EndExpr"))]
    #[must_use]
    pub fn interval_end(&self, interval: IntervalVar) -> LinearExpr {
        interval_part(interval, &self.interval_proto(interval).end)
    }

    /// The literals that make `interval` present, none for an interval that is
    /// always present.
    ///
    /// # Panics
    ///
    /// If `interval` is not an interval of this model.
    #[doc(alias("presence_literals", "PresenceBoolVar"))]
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

/// Converts a part of `interval`, which only a hand-edited message leaves
/// ragged.
fn interval_part(interval: IntervalVar, expr: &LinearExpressionProto) -> LinearExpr {
    LinearExpr::from_proto(expr).unwrap_or_else(|| {
        panic!("{interval:?} has an expression with vars and coeffs of unequal lengths")
    })
}

impl Index<IntVar> for CpModel {
    type Output = IntegerVariableProto;

    fn index(&self, var: IntVar) -> &IntegerVariableProto {
        &self.0.variables[slot(var.0)]
    }
}

impl Index<IntervalVar> for CpModel {
    type Output = ConstraintProto;

    fn index(&self, interval: IntervalVar) -> &ConstraintProto {
        &self.0.constraints[slot(interval.0)]
    }
}

impl Index<ConstraintIndex> for CpModel {
    type Output = ConstraintProto;

    fn index(&self, constraint: ConstraintIndex) -> &ConstraintProto {
        &self.0.constraints[slot(constraint.0)]
    }
}
