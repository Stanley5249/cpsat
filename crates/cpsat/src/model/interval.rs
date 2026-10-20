//! Intervals before insertion, like Python's `new_*interval_var` methods.

use buffa::MessageField;

use super::{Model, NewVar, sealed};
use crate::constraint::Constraint;
use crate::expr::{BoolVar, IntervalVar, LinearExpr};
use crate::proto::{ConstraintProto, IntervalConstraintProto};

/// An interval before [`Model::new_var`] inserts it, which returns its
/// [`IntervalVar`].
///
/// The schema stores an interval as a constraint row that scheduling
/// constraints such as [`Constraint::no_overlap`] refer to by index. When the
/// interval is present, the solver enforces `start + size == end`, and the
/// validator requires `size >= 0`. CP-SAT accepts affine expressions with at
/// most one variable here.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let start = model.new_var(0..=10);
/// let present = model.new_var(Bool);
/// let task = model.new_var(
///     Interval::fixed_size(start, 3)
///         .only_enforce_if(present)
///         .with_name("task"),
/// );
/// model.add(Constraint::no_overlap([task]));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Interval(Constraint);

impl Interval {
    /// An interval from `start` to `end`, like Python's `new_interval_var`.
    #[must_use]
    pub fn new(
        start: impl Into<LinearExpr>,
        size: impl Into<LinearExpr>,
        end: impl Into<LinearExpr>,
    ) -> Self {
        Self::of(start.into(), size.into(), end.into())
    }

    /// An interval of a fixed size, like Python's
    /// `new_fixed_size_interval_var`.
    #[must_use]
    pub fn fixed_size(start: impl Into<LinearExpr>, size: i64) -> Self {
        let start = start.into();
        let end = start.clone() + size;
        Self::of(start, size.into(), end)
    }

    /// Makes the interval present only when `literal` is true, like Python's
    /// `new_optional_interval_var`. Scheduling constraints ignore an absent
    /// interval, and nothing links its start, size, and end.
    #[must_use]
    pub fn only_enforce_if(self, literal: BoolVar) -> Self {
        Self(self.0.only_enforce_if(literal))
    }

    /// Names the interval, like the `name` argument of Python's
    /// `new_interval_var`.
    #[must_use]
    pub fn with_name(self, name: impl Into<String>) -> Self {
        Self(self.0.with_name(name))
    }

    fn of(start: LinearExpr, size: LinearExpr, end: LinearExpr) -> Self {
        let interval = IntervalConstraintProto {
            start: MessageField::some(start.into()),
            end: MessageField::some(end.into()),
            size: MessageField::some(size.into()),
            ..Default::default()
        };
        Self(Constraint::from(ConstraintProto {
            constraint: Some(interval.into()),
            ..Default::default()
        }))
    }
}

impl sealed::Sealed for Interval {}

impl NewVar for Interval {
    type Output = IntervalVar;

    fn insert(self, model: &mut Model) -> IntervalVar {
        IntervalVar(model.add(self.0).index())
    }
}
