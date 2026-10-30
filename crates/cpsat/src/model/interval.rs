//! Intervals, like Python's `new_*interval_var` methods, and reading them
//! back from a model.

use buffa::MessageField;

use super::{CpModel, NewVar, sealed};
use crate::constraint::Constraint;
use crate::expr::{BoolVar, IntervalVar, LinearExpr};
use crate::proto::constraint_proto::Constraint as Kind;
use crate::proto::{ConstraintProto, IntervalConstraintProto, LinearExpressionProto};

/// An interval before [`CpModel::new_var`] inserts it, which returns its
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
/// // From `optional_interval_sample_sat.py`.
/// let mut model = CpModel::new();
/// let start = model.new_var((0..=100).with_name("start"));
/// let end = model.new_var((0..=100).with_name("end"));
/// let presence = model.new_var(Bool.with_name("presence"));
/// let interval = model.new_var(
///     Interval::new(start, 10, end + 2)
///         .only_enforce_if(presence)
///         .with_name("interval"),
/// );
/// let fixed = model.new_var(Interval::fixed_size(5, 10).only_enforce_if(presence));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Interval(Constraint);

impl Interval {
    /// An interval from `start` to `end`.
    ///
    /// # Panics
    ///
    /// On an `i64` overflow, as [`LinearExpr`] describes.
    #[doc(alias("new_interval_var", "NewIntervalVar"))]
    #[must_use]
    pub fn new(
        start: impl Into<LinearExpr>,
        size: impl Into<LinearExpr>,
        end: impl Into<LinearExpr>,
    ) -> Self {
        Self::of(start.into(), size.into(), end.into())
    }

    /// An interval of a fixed size.
    ///
    /// # Panics
    ///
    /// On an `i64` overflow, including in `start + size`, as [`LinearExpr`] describes.
    #[doc(alias("new_fixed_size_interval_var", "NewFixedSizeIntervalVar"))]
    #[must_use]
    pub fn fixed_size(start: impl Into<LinearExpr>, size: i64) -> Self {
        let start = start.into();
        let end = start.clone() + size;
        Self::of(start, size.into(), end)
    }

    /// Makes the interval present only when `literal` is true. Scheduling
    /// constraints ignore an absent interval, and nothing links its start,
    /// size, and end. Several calls make the interval present only when every
    /// literal is true.
    #[doc(alias(
        "new_optional_interval_var",
        "new_optional_fixed_size_interval_var",
        "NewOptionalIntervalVar",
        "NewOptionalFixedSizeIntervalVar"
    ))]
    #[must_use]
    pub fn only_enforce_if(self, literal: BoolVar) -> Self {
        Self(self.0.only_enforce_if(literal))
    }

    /// Names the interval, like the `name` argument of Python's
    /// `new_interval_var`.
    #[doc(alias("WithName"))]
    #[must_use]
    pub fn with_name(self, name: impl Into<String>) -> Self {
        Self(self.0.with_name(name))
    }

    fn of(start: LinearExpr, size: LinearExpr, end: LinearExpr) -> Self {
        let interval = IntervalConstraintProto {
            start: MessageField::some(start.into_proto()),
            end: MessageField::some(end.into_proto()),
            size: MessageField::some(size.into_proto()),
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

    /// `name` is always empty, because `Interval` names itself through its
    /// own `with_name` instead of [`WithName`](crate::prelude::WithName).
    fn insert(self, model: &mut CpModel, _: String) -> IntervalVar {
        IntervalVar(model.add(self.0).index())
    }
}

impl CpModel {
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
}

/// Converts a part of `interval`, which only a hand-edited message leaves
/// ragged.
fn interval_part(interval: IntervalVar, expr: &LinearExpressionProto) -> LinearExpr {
    LinearExpr::from_proto(expr).unwrap_or_else(|| {
        panic!("{interval:?} has an expression with vars and coeffs of unequal lengths")
    })
}
