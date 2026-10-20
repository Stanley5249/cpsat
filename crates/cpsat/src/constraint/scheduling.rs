//! Scheduling constraints over intervals and events in time.

use buffa::MessageField;

use super::Constraint;
use crate::expr::{BoolVar, IntervalVar, LinearExpr};
use crate::proto::{
    CumulativeConstraintProto, LinearExpressionProto, NoOverlap2DConstraintProto,
    NoOverlapConstraintProto, ReservoirConstraintProto,
};

impl Constraint {
    /// The present intervals do not overlap.
    #[doc(alias("add_no_overlap", "AddNoOverlap"))]
    #[must_use]
    pub fn no_overlap(intervals: impl IntoIterator<Item = IntervalVar>) -> Self {
        Self::of(NoOverlapConstraintProto {
            intervals: intervals.into_iter().map(IntervalVar::index).collect(),
            ..Default::default()
        })
    }

    /// The present rectangles do not overlap. Each rectangle is an `(x, y)`
    /// pair of intervals, and it is absent when either interval is.
    #[doc(alias("add_no_overlap_2d", "AddNoOverlap2D"))]
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

    /// At every time, the demands of the present intervals that cover it sum to
    /// at most `capacity`. Each task is `(interval, demand)`, and a demand must
    /// be non-negative. CP-SAT accepts affine capacities and demands.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// // From `cumulative_variable_profile_sample_sat.py`, which also
    /// // varies the capacity.
    /// let mut model = Model::new();
    /// let start = model.new_var(0..=1380);
    /// let performed = model.new_var(Bool);
    /// let task = model.new_var(Interval::fixed_size(start, 60).only_enforce_if(performed));
    /// let capacity = model.new_var(0..=12);
    /// model.add(Constraint::cumulative(capacity, [(task, 3)]));
    /// ```
    #[doc(alias("add_cumulative", "AddCumulative"))]
    #[must_use]
    pub fn cumulative(
        capacity: impl Into<LinearExpr>,
        tasks: impl IntoIterator<Item = (IntervalVar, impl Into<LinearExpr>)>,
    ) -> Self {
        let (intervals, demands) = tasks
            .into_iter()
            .map(|(interval, demand)| (interval.index(), demand.into().into()))
            .unzip();
        Self::of(CumulativeConstraintProto {
            capacity: MessageField::some(capacity.into().into()),
            intervals,
            demands,
            ..Default::default()
        })
    }

    /// A level that starts at 0 and stays within `[min_level, max_level]`. Each
    /// event is `(time, change)`: the level changes by `change` at `time`.
    /// OR-Tools requires `min_level <= 0 <= max_level`.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// let mut model = Model::new();
    /// let fill = model.new_var(0..=10);
    /// let drain = model.new_var(0..=10);
    /// model.add(Constraint::reservoir(0, 5, [(fill, 3), (drain, -3)]));
    /// ```
    #[doc(alias("add_reservoir_constraint", "AddReservoirConstraint"))]
    #[must_use]
    pub fn reservoir(
        min_level: i64,
        max_level: i64,
        events: impl IntoIterator<Item = (impl Into<LinearExpr>, impl Into<LinearExpr>)>,
    ) -> Self {
        let (time_exprs, level_changes) = events
            .into_iter()
            .map(|(time, change)| (time.into().into(), change.into().into()))
            .unzip();
        Self::of(reservoir(min_level, max_level, time_exprs, level_changes))
    }

    /// Like [`reservoir`](Self::reservoir), with events `(time, change,
    /// active)` that apply only when `active` is true.
    #[doc(alias("add_reservoir_constraint_with_active", "AddOptionalEvent"))]
    #[must_use]
    pub fn reservoir_with_active(
        min_level: i64,
        max_level: i64,
        events: impl IntoIterator<Item = (impl Into<LinearExpr>, impl Into<LinearExpr>, BoolVar)>,
    ) -> Self {
        let mut time_exprs = Vec::new();
        let mut level_changes = Vec::new();
        let mut active_literals = Vec::new();
        for (time, change, active) in events {
            time_exprs.push(time.into().into());
            level_changes.push(change.into().into());
            active_literals.push(active.index());
        }
        Self::of(ReservoirConstraintProto {
            active_literals,
            ..reservoir(min_level, max_level, time_exprs, level_changes)
        })
    }
}

fn reservoir(
    min_level: i64,
    max_level: i64,
    time_exprs: Vec<LinearExpressionProto>,
    level_changes: Vec<LinearExpressionProto>,
) -> ReservoirConstraintProto {
    ReservoirConstraintProto {
        min_level,
        max_level,
        time_exprs,
        level_changes,
        ..Default::default()
    }
}
