//! Interval construction, like Python's `new_*interval_var` methods.

use buffa::MessageField;

use super::Model;
use crate::constraint::Constraint;
use crate::expr::{BoolVar, IntervalVar, LinearExpr};
use crate::proto::{ConstraintProto, IntervalConstraintProto};

impl Model {
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
}
