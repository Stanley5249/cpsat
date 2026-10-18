//! Scheduling constraints over intervals and events in time.

use buffa::MessageField;

use super::Constraint;
use crate::expr::LinearExpr;
use crate::proto::{
    CumulativeConstraintProto, NoOverlap2DConstraintProto, NoOverlapConstraintProto,
    ReservoirConstraintProto,
};
use crate::variable::{BoolVar, IntervalVar};

impl Constraint {
    /// The present intervals do not overlap.
    #[must_use]
    pub fn no_overlap(intervals: impl IntoIterator<Item = IntervalVar>) -> Self {
        Self::of(NoOverlapConstraintProto {
            intervals: intervals.into_iter().map(IntervalVar::index).collect(),
            ..Default::default()
        })
    }

    /// The present rectangles do not overlap. Each rectangle is an `(x, y)`
    /// pair of intervals, and it is absent when either interval is.
    #[must_use]
    pub fn no_overlap_2d(rectangles: impl IntoIterator<Item = (IntervalVar, IntervalVar)>) -> Self {
        let (x_intervals, y_intervals) = rectangles
            .into_iter()
            .map(|(x, y)| (x.index(), y.index()))
            .unzip();
        Self::of(NoOverlap2DConstraintProto {
            x_intervals,
            y_intervals,
            ..Default::default()
        })
    }
}

/// At every time, the demands of the present intervals that cover it sum to
/// at most the capacity, like Python's `add_cumulative`.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let start = model.int_var(0..=10);
/// let task = model.fixed_size_interval_var(start, 3);
/// model.add(Cumulative::new(2).task(task, 1));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cumulative(CumulativeConstraintProto);

impl Cumulative {
    /// CP-SAT accepts an affine capacity `a * x + b`.
    #[must_use]
    pub fn new(capacity: impl Into<LinearExpr>) -> Self {
        Self(CumulativeConstraintProto {
            capacity: MessageField::some(capacity.into().into()),
            ..Default::default()
        })
    }

    /// An interval that uses `demand`, which must be non-negative and may be
    /// affine.
    #[must_use]
    pub fn task(mut self, interval: IntervalVar, demand: impl Into<LinearExpr>) -> Self {
        self.0.intervals.push(interval.index());
        self.0.demands.push(demand.into().into());
        self
    }
}

impl From<Cumulative> for Constraint {
    fn from(cumulative: Cumulative) -> Self {
        Self::of(cumulative.0)
    }
}

/// A level that starts at 0 and stays within `[min_level, max_level]` while
/// events change it, like Python's `add_reservoir_constraint`.
///
/// Use either [`event`](Self::event) or [`active_event`](Self::active_event)
/// for every event, because the schema needs an active literal for all events
/// or for none. An always-active event takes
/// [`Model::bool_constant(true)`](crate::Model::bool_constant).
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let fill = model.int_var(0..=10);
/// let drain = model.int_var(0..=10);
/// model.add(Reservoir::new(0, 5).event(fill, 3).event(drain, -3));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reservoir(ReservoirConstraintProto);

impl Reservoir {
    /// OR-Tools requires `min_level <= 0 <= max_level`.
    #[must_use]
    pub fn new(min_level: i64, max_level: i64) -> Self {
        Self(ReservoirConstraintProto {
            min_level,
            max_level,
            ..Default::default()
        })
    }

    /// Changes the level by `change` at `time`. CP-SAT accepts affine
    /// expressions for both.
    #[must_use]
    pub fn event(mut self, time: impl Into<LinearExpr>, change: impl Into<LinearExpr>) -> Self {
        self.0.time_exprs.push(time.into().into());
        self.0.level_changes.push(change.into().into());
        self
    }

    /// Changes the level by `change` at `time` when `active` is true, like
    /// Python's `add_reservoir_constraint_with_active`.
    #[must_use]
    pub fn active_event(
        self,
        time: impl Into<LinearExpr>,
        change: impl Into<LinearExpr>,
        active: BoolVar,
    ) -> Self {
        let mut reservoir = self.event(time, change);
        reservoir.0.active_literals.push(active.index());
        reservoir
    }
}

impl From<Reservoir> for Constraint {
    fn from(reservoir: Reservoir) -> Self {
        Self::of(reservoir.0)
    }
}
