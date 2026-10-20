//! `Domain`, checked against what OR-Tools' `Domain` returns for the same sets.

use cpsat::prelude::*;
use cpsat::proto::{CpSolverStatus, SatParameters};

#[test]
fn normalizes_intervals() {
    let domain = Domain::from_intervals([(5, 7), (1, 2), (3, 3), (9, 8), (10, 12)]);
    assert_eq!(domain.intervals(), [(1, 3), (5, 7), (10, 12)]);
    assert!(Domain::new(3, 1).is_empty());
    assert_eq!(
        Domain::from_values([4, 2, 3, 9]).intervals(),
        [(2, 4), (9, 9)]
    );
}

#[test]
fn complements_at_the_i64_limits() {
    assert_eq!(Domain::all().complement(), Domain::default());
    assert_eq!(Domain::default().complement(), Domain::all());
    let edges = Domain::from_values([i64::MIN, i64::MAX]).complement();
    assert_eq!(edges.intervals(), [(i64::MIN + 1, i64::MAX - 1)]);
    let domain = Domain::from_intervals([(0, 3), (8, 9)]);
    assert_eq!(domain.complement().complement(), domain);
}

#[test]
fn unions_and_intersects() {
    let a = Domain::from_intervals([(0, 5), (10, 15)]);
    let b = Domain::from_intervals([(4, 11)]);
    assert_eq!(a.union_with(&b).intervals(), [(0, 15)]);
    assert_eq!(a.intersection_with(&b).intervals(), [(4, 5), (10, 11)]);
    assert!(a.contains(12) && !a.contains(7));
}

#[test]
fn flat_form_round_trips() {
    let domain = Domain::from_intervals([(0, 2), (5, 5)]);
    let flat = domain.flattened_intervals();
    assert_eq!(flat, [0, 2, 5, 5]);
    assert_eq!(Domain::from_flat_intervals(&flat), Some(domain));
    assert_eq!(Domain::from_flat_intervals(&[1, 2, 3]), None);
}

#[test]
fn builds_a_variable_with_holes() {
    let mut model = Model::new();
    let x = model.new_var(Domain::from_values([1, 3, 5]));
    model.maximize(x);
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert_eq!(response.value(x), Some(5));
}

#[test]
fn reports_bounds_and_size() {
    let domain = Domain::from_values([1, 3, 9]);
    assert_eq!(
        (domain.min(), domain.max(), domain.size()),
        (Some(1), Some(9), 3)
    );
    let empty = Domain::default();
    assert_eq!((empty.min(), empty.max(), empty.size()), (None, None, 0));
    assert_eq!(Domain::new(0, 5).size(), 6);
    assert_eq!(Domain::all().size(), u64::MAX);
    assert_eq!(Domain::new(i64::MIN, -1).size(), 1 << 63);
    let edges = Domain::from_values([i64::MIN, i64::MAX]);
    assert_eq!((edges.min(), edges.max()), (Some(i64::MIN), Some(i64::MAX)));
}

#[test]
fn compares_sets() {
    let small = Domain::new(2, 3);
    let large = Domain::from_intervals([(0, 5), (10, 15)]);
    assert!(small.is_included_in(&large) && !large.is_included_in(&small));
    assert!(Domain::default().is_included_in(&small));
    assert!(small.overlaps_with(&large));
    assert!(!small.overlaps_with(&Domain::new(4, 9)));
    assert!(small.overlaps_with(&Domain::new(3, 9)));
    assert!(small.is_included_in(&small));
    assert!(!small.is_included_in(&Domain::default()));
}

#[test]
fn negates_and_adds_like_python() {
    const MIN: i64 = i64::MIN;
    const MAX: i64 = i64::MAX;
    let flat = |domain: Domain| domain.flattened_intervals();
    // The results of Python's OR-Tools 9.15 `Domain` for the same sets.
    let holes = Domain::from_intervals([(1, 3), (7, 9)]);
    assert_eq!(flat(holes.negation()), [-9, -7, -3, -1]);
    assert_eq!(flat(Domain::all().negation()), [MIN + 1, MAX]);
    assert_eq!(flat(Domain::new(MIN, MIN).negation()), [] as [i64; 0]);
    assert_eq!(flat(Domain::new(MIN, 0).negation()), [0, MAX]);

    let a = Domain::from_intervals([(0, 1), (10, 11)]);
    let b = Domain::from_values([0, 5]);
    assert_eq!(flat(a.addition_with(&b)), [0, 1, 5, 6, 10, 11, 15, 16]);
    let five = Domain::new(5, 5);
    assert_eq!(
        flat(Domain::new(MAX - 1, MAX).addition_with(&five)),
        [] as [i64; 0]
    );
    let minus_three = Domain::new(-3, -3);
    assert_eq!(
        flat(Domain::new(0, MAX).addition_with(&minus_three)),
        [-3, MAX - 3]
    );
    let min = Domain::new(MIN, MIN);
    assert_eq!(flat(Domain::new(MIN, 0).addition_with(&min)), [MIN, MIN]);
    assert!(
        Domain::new(0, 1)
            .addition_with(&Domain::default())
            .is_empty()
    );
}

#[test]
fn builds_half_lines_from_ranges() {
    assert_eq!(Domain::from(2..), Domain::new(2, i64::MAX));
    assert_eq!(Domain::from(..=8), Domain::new(i64::MIN, 8));
}

#[test]
fn converts_rust_ranges() {
    assert_eq!(Domain::from(0..3), Domain::new(0, 2));
    assert_eq!(Domain::from(0..0), Domain::default());
    #[expect(
        clippy::reversed_empty_ranges,
        reason = "i64::MIN has no predecessor to end the interval"
    )]
    let below_min = 0..i64::MIN;
    assert_eq!(Domain::from(below_min), Domain::default());

    // A variable domain must stay inside (i64::MIN, i64::MAX). `new_var`
    // rejects `0..` at compile time, so the unbounded domain comes in as a
    // `Domain`.
    let mut model = Model::new();
    model.new_var(Domain::from(0..));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::MODEL_INVALID);
}

/// The linear set operations agree with the definitions on touching,
/// nested, and limit-reaching intervals.
#[test]
fn set_operations_handle_touching_and_limit_intervals() {
    let low = Domain::from_intervals([(i64::MIN, -5), (0, 2)]);
    let high = Domain::from_intervals([(3, 4), (10, i64::MAX)]);
    assert_eq!(
        low.union_with(&high).intervals(),
        [(i64::MIN, -5), (0, 4), (10, i64::MAX)]
    );
    assert!(low.intersection_with(&high).is_empty());
    assert!(!low.overlaps_with(&high));

    let wide = Domain::from_intervals([(i64::MIN, 0), (5, 20)]);
    let parts = Domain::from_intervals([(-3, -1), (6, 7), (19, 25)]);
    assert_eq!(
        wide.intersection_with(&parts).intervals(),
        [(-3, -1), (6, 7), (19, 20)]
    );
    assert!(!parts.is_included_in(&wide));
    assert!(Domain::from_intervals([(-3, -1), (6, 7)]).is_included_in(&wide));
    assert!(!Domain::new(0, 5).is_included_in(&wide), "spans a gap");
    assert_eq!(Domain::all().intersection_with(&wide), wide);
    assert_eq!(wide.union_with(&wide.complement()), Domain::all());
}
