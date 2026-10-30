//! Handle lookups: indexing with a handle and finding a handle by its
//! schema index.

use std::ops::Index;

use super::CpModel;
use crate::constraint::ConstraintIndex;
use crate::expr::{BoolVar, IntVar, IntervalVar};
use crate::proto::constraint_proto::Constraint as Kind;
use crate::proto::{ConstraintProto, IntegerVariableProto, IntervalConstraintProto};

/// Converts a handle into a `Vec` index, which panics for a negative index
/// like an index out of range does.
pub(super) fn slot(index: i32) -> usize {
    usize::try_from(index).expect("a handle index is non-negative")
}

impl CpModel {
    /// The variable at `index`.
    #[doc(alias("get_int_var_from_proto_index", "GetIntVarFromProtoIndex"))]
    #[must_use]
    pub fn int_var_from_index(&self, index: i32) -> Option<IntVar> {
        self.var_proto(index).map(|_| IntVar(index))
    }

    /// `var` as a literal if its domain is within `[0, 1]`, for the places
    /// where Python accepts a 0-1 integer variable as a literal. With
    /// [`int_var_from_index`](Self::int_var_from_index), it replaces Python's
    /// `get_bool_var_from_proto_index`.
    ///
    /// # Panics
    ///
    /// Panics if the model lacks `var`, like indexing with it.
    #[doc(alias("get_bool_var_from_proto_index", "ToBoolVar"))]
    #[must_use]
    pub fn to_bool_var(&self, var: IntVar) -> Option<BoolVar> {
        // Reads the flat field in place: a nonempty list of pairs with every
        // nonempty pair inside [0, 1].
        let (pairs, rest) = self[var].domain.as_chunks::<2>();
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

    fn var_proto(&self, index: i32) -> Option<&IntegerVariableProto> {
        self.0.variables.get(usize::try_from(index).ok()?)
    }
}

impl Index<IntVar> for CpModel {
    type Output = IntegerVariableProto;

    fn index(&self, var: IntVar) -> &IntegerVariableProto {
        &self.0.variables[slot(var.0)]
    }
}

impl Index<IntervalVar> for CpModel {
    type Output = IntervalConstraintProto;

    /// The interval's message. [`ConstraintIndex::from`] reaches its whole
    /// row, with its name and enforcement literals.
    ///
    /// # Panics
    ///
    /// If the model lacks the row, or if the row does not hold an interval.
    fn index(&self, interval: IntervalVar) -> &IntervalConstraintProto {
        match &self[ConstraintIndex::from(interval)].constraint {
            Some(Kind::Interval(proto)) => proto,
            _ => panic!("{interval:?} is not an interval of this model"),
        }
    }
}

impl Index<ConstraintIndex> for CpModel {
    type Output = ConstraintProto;

    fn index(&self, constraint: ConstraintIndex) -> &ConstraintProto {
        &self.0.constraints[slot(constraint.0)]
    }
}
