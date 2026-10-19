//! A set of integers in the interval form that the schema uses.

use std::ops::{Range, RangeFrom, RangeInclusive, RangeToInclusive};

/// A set of integers as sorted, disjoint, non-adjacent closed intervals, like
/// OR-Tools' `Domain`.
///
/// It converts into the flat `[min_0, max_0, min_1, max_1, ...]` list of the
/// schema's `domain` fields, so it can build a variable or a linear constraint.
///
/// ```
/// use cpsat::Domain;
///
/// // x != 0, the domain Python builds for `x != 0`.
/// let domain = Domain::from_values([0]).complement();
/// assert_eq!(domain.flattened_intervals(), [i64::MIN, -1, 1, i64::MAX]);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Domain(Vec<(i64, i64)>);

impl Domain {
    /// The interval `[lower, upper]`, or the empty domain if `lower > upper`.
    #[must_use]
    pub fn new(lower: i64, upper: i64) -> Self {
        Self::from_intervals([(lower, upper)])
    }

    /// Every `i64`.
    #[must_use]
    pub fn all() -> Self {
        Self(vec![(i64::MIN, i64::MAX)])
    }

    #[must_use]
    pub fn from_values(values: impl IntoIterator<Item = i64>) -> Self {
        Self::from_intervals(values.into_iter().map(|value| (value, value)))
    }

    /// Normalizes any intervals: empty ones are dropped, and overlapping or
    /// adjacent ones merge.
    #[must_use]
    pub fn from_intervals(intervals: impl IntoIterator<Item = (i64, i64)>) -> Self {
        let mut intervals: Vec<_> = intervals
            .into_iter()
            .filter(|&(lower, upper)| lower <= upper)
            .collect();
        intervals.sort_unstable();
        let mut merged = Vec::with_capacity(intervals.len());
        for interval in intervals {
            push_merged(&mut merged, interval);
        }
        Self(merged)
    }

    /// Reads a schema `domain` field, or `None` if it has an odd length.
    #[must_use]
    pub fn from_flat_intervals(flat: &[i64]) -> Option<Self> {
        let (pairs, rest) = flat.as_chunks::<2>();
        if !rest.is_empty() {
            return None;
        }
        Some(Self::from_intervals(
            pairs.iter().map(|&[lower, upper]| (lower, upper)),
        ))
    }

    #[must_use]
    pub fn intervals(&self) -> &[(i64, i64)] {
        &self.0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The smallest value, or `None` if the domain is empty.
    #[must_use]
    pub fn min(&self) -> Option<i64> {
        self.0.first().map(|&(lower, _)| lower)
    }

    /// The largest value, or `None` if the domain is empty.
    #[must_use]
    pub fn max(&self) -> Option<i64> {
        self.0.last().map(|&(_, upper)| upper)
    }

    /// The number of values. It saturates at `u64::MAX` for [`Domain::all`],
    /// where Python's saturates at `i64::MAX` for any larger domain.
    #[must_use]
    pub fn size(&self) -> u64 {
        self.0.iter().fold(0, |size: u64, &(lower, upper)| {
            size.saturating_add(upper.abs_diff(lower).saturating_add(1))
        })
    }

    #[must_use]
    pub fn contains(&self, value: i64) -> bool {
        self.0
            .iter()
            .any(|&(lower, upper)| (lower..=upper).contains(&value))
    }

    /// Every `i64` outside this domain.
    #[must_use]
    pub fn complement(&self) -> Self {
        let mut gaps = Vec::with_capacity(self.0.len() + 1);
        let mut next = Some(i64::MIN);
        for &(lower, upper) in &self.0 {
            if let Some(start) = next
                && start < lower
            {
                gaps.push((start, lower - 1));
            }
            next = upper.checked_add(1);
        }
        if let Some(start) = next {
            gaps.push((start, i64::MAX));
        }
        Self(gaps)
    }

    /// Whether every value of this domain is in `other`. Each interval must
    /// fit in one interval of `other`, because `other` has gaps between its
    /// intervals.
    #[must_use]
    pub fn is_included_in(&self, other: &Self) -> bool {
        let mut rest = other.0.as_slice();
        self.0.iter().all(|&(lower, upper)| {
            while rest
                .first()
                .is_some_and(|&(_, other_upper)| other_upper < lower)
            {
                rest = &rest[1..];
            }
            rest.first().is_some_and(|&(other_lower, other_upper)| {
                other_lower <= lower && upper <= other_upper
            })
        })
    }

    /// Whether this domain and `other` share a value.
    #[must_use]
    pub fn overlaps_with(&self, other: &Self) -> bool {
        Overlaps {
            left: &self.0,
            right: &other.0,
        }
        .next()
        .is_some()
    }

    #[must_use]
    pub fn union_with(&self, other: &Self) -> Self {
        let mut merged = Vec::with_capacity(self.0.len() + other.0.len());
        let (mut left, mut right) = (self.0.iter().peekable(), other.0.iter().peekable());
        // Both sides are sorted, so taking the smaller start keeps the output
        // sorted and lets each interval merge into the last one.
        while let Some(&interval) = match (left.peek(), right.peek()) {
            (Some(a), Some(b)) if b < a => right.next(),
            (Some(_), _) => left.next(),
            (None, _) => right.next(),
        } {
            push_merged(&mut merged, interval);
        }
        Self(merged)
    }

    #[must_use]
    pub fn intersection_with(&self, other: &Self) -> Self {
        Self(
            Overlaps {
                left: &self.0,
                right: &other.0,
            }
            .collect(),
        )
    }

    /// `-x` for every `x` in this domain, like Python's `negation`. `i64::MIN`
    /// drops out, because its negation does not fit in `i64`.
    #[must_use]
    pub fn negation(&self) -> Self {
        Self::from_exact(
            self.0
                .iter()
                .map(|&(lower, upper)| (-i128::from(upper), -i128::from(lower))),
        )
    }

    /// `x + y` for every `x` in this domain and `y` in `other`, like Python's
    /// `addition_with`. Sums outside `i64` drop out.
    #[must_use]
    pub fn addition_with(&self, other: &Self) -> Self {
        Self::from_exact(self.0.iter().flat_map(|&(lower, upper)| {
            other.0.iter().map(move |&(other_lower, other_upper)| {
                (
                    i128::from(lower) + i128::from(other_lower),
                    i128::from(upper) + i128::from(other_upper),
                )
            })
        }))
    }

    /// Exact intervals cut to the `i64` range, as OR-Tools' `Domain` cuts
    /// its arithmetic.
    fn from_exact(intervals: impl IntoIterator<Item = (i128, i128)>) -> Self {
        let (min, max) = (i128::from(i64::MIN), i128::from(i64::MAX));
        let cut = |bound: i128| {
            i64::try_from(bound.clamp(min, max)).expect("the bound is clamped into i64")
        };
        Self::from_intervals(
            intervals
                .into_iter()
                .filter(|&(lower, upper)| lower <= max && upper >= min)
                .map(|(lower, upper)| (cut(lower), cut(upper))),
        )
    }

    /// The schema's flat `[min_0, max_0, ...]` form.
    #[must_use]
    pub fn flattened_intervals(&self) -> Vec<i64> {
        self.0
            .iter()
            .flat_map(|&(lower, upper)| [lower, upper])
            .collect()
    }
}

impl FromIterator<i64> for Domain {
    fn from_iter<I: IntoIterator<Item = i64>>(values: I) -> Self {
        Self::from_values(values)
    }
}

impl From<RangeInclusive<i64>> for Domain {
    /// The interval `[start, end]`, so `model.int_var(0..=10)` reads like
    /// Rust.
    fn from(range: RangeInclusive<i64>) -> Self {
        Self::new(*range.start(), *range.end())
    }
}

impl From<Range<i64>> for Domain {
    /// The interval `[start, end - 1]`, so `model.int_var(0..n)` reads like
    /// Rust.
    fn from(range: Range<i64>) -> Self {
        range
            .end
            .checked_sub(1)
            .map_or_else(Self::default, |end| Self::new(range.start, end))
    }
}

impl From<RangeFrom<i64>> for Domain {
    /// Every value from `start` on, like Python's `greater_or_equal`. The
    /// solver rejects a variable domain that reaches `i64::MAX`, so bound it
    /// for a variable.
    fn from(range: RangeFrom<i64>) -> Self {
        Self::new(range.start, i64::MAX)
    }
}

impl From<RangeToInclusive<i64>> for Domain {
    /// Every value up to `end`, like Python's `lower_or_equal`. The solver
    /// rejects a variable domain that reaches `i64::MIN`, so bound it for a
    /// variable.
    fn from(range: RangeToInclusive<i64>) -> Self {
        Self::new(i64::MIN, range.end)
    }
}

/// Appends `interval`, which starts no earlier than the last one, merging it
/// into the last one when they overlap or touch.
fn push_merged(merged: &mut Vec<(i64, i64)>, (lower, upper): (i64, i64)) {
    match merged.last_mut() {
        Some(last) if last.1.checked_add(1).is_none_or(|next| lower <= next) => {
            last.1 = last.1.max(upper);
        }
        _ => merged.push((lower, upper)),
    }
}

/// The nonempty intersections of two normalized interval lists, in order.
/// Each step drops the interval that ends first, because it meets nothing
/// further on the other side.
struct Overlaps<'a> {
    left: &'a [(i64, i64)],
    right: &'a [(i64, i64)],
}

impl Iterator for Overlaps<'_> {
    type Item = (i64, i64);

    fn next(&mut self) -> Option<(i64, i64)> {
        loop {
            let (&(a_lower, a_upper), &(b_lower, b_upper)) =
                (self.left.first()?, self.right.first()?);
            if a_upper < b_upper {
                self.left = &self.left[1..];
            } else {
                self.right = &self.right[1..];
            }
            let (lower, upper) = (a_lower.max(b_lower), a_upper.min(b_upper));
            if lower <= upper {
                return Some((lower, upper));
            }
        }
    }
}
