//! Linear and floating-point expressions: arithmetic, conversion, overflow, and comparisons at the i64 limits.

use std::panic::AssertUnwindSafe;

use cpsat::prelude::*;
use cpsat::proto::constraint_proto::Constraint as Kind;
use cpsat::proto::{CpSolverResponse, CpSolverStatus, LinearExpressionProto};

#[test]
#[expect(clippy::cast_precision_loss, reason = "the test compares in doubles")]
fn float_conversion_merges_in_doubles() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    let expr = FloatExpr::from(x * i64::MAX + x);
    model.minimize(expr);
    let objective = model.proto().floating_point_objective.as_option().unwrap();
    assert_eq!(objective.vars, [0]);
    assert_eq!(objective.coeffs, [i64::MAX as f64 + 1.0]);

    // Python 9.15 writes these objectives: 2^53 + 1 rounds to 2^53 before the
    // merge, so the sum is not 2^53 + 2 and the difference drops `x`.
    let big = 1 << 53;
    let y = model.new_var(0..=5);
    model.minimize(FloatExpr::from(x * (big + 1) + x) + 0.5 * y);
    let objective = model.proto().floating_point_objective.as_option().unwrap();
    assert_eq!(objective.vars, [0, 1]);
    assert_eq!(objective.coeffs, [big as f64, 0.5]);
    model.minimize(FloatExpr::from(x * (big + 1) - x * big) + 0.5 * y);
    let objective = model.proto().floating_point_objective.as_option().unwrap();
    assert_eq!(objective.vars, [1]);
    assert_eq!(objective.coeffs, [0.5]);
}

#[test]
fn borrowed_float_conversion_matches_the_owned_one() {
    let mut model = CpModel::new();
    let [x, y] = std::array::from_fn(|_| model.new_var(0..=5));
    let big = 1 << 53;
    // Repeated terms stay unmerged and the offset rounds, in both forms.
    let expr = x * (big + 1) + 3 * y - x + (big + 1);
    assert_eq!(FloatExpr::from(&expr), FloatExpr::from(expr.clone()));
}

#[test]
fn expressions_merge_terms_like_python() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    let y = model.new_var(0..=5);
    let mut expr = LinearExpr::from(y);
    expr += 2 * x;
    expr -= y;
    expr *= 3;
    let proto = LinearExpr::into_proto(expr + 4);
    assert_eq!(
        (proto.vars, proto.coeffs, proto.offset),
        (vec![0], vec![6], 4)
    );

    let empty: LinearExpr = std::iter::empty::<IntVar>().sum();
    assert_eq!(
        LinearExpr::into_proto(empty),
        LinearExpressionProto::default()
    );
}

#[test]
#[should_panic(expected = "linear expression overflows i64")]
fn coefficient_overflow_panics() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=1);
    let _ = x * i64::MAX * 2;
}

#[test]
#[should_panic(expected = "linear expression overflows i64")]
fn negating_the_minimum_coefficient_panics() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=1);
    let _ = -(x * i64::MIN);
}

fn linear_domain(constraint: Constraint) -> Vec<i64> {
    match constraint.into_proto().constraint {
        Some(Kind::Linear(linear)) => linear.domain,
        other => panic!("expected a linear constraint, got {other:?}"),
    }
}

#[test]
fn comparisons_at_the_i64_limits_match_python() {
    const MIN: i64 = i64::MIN;
    const MAX: i64 = i64::MAX;
    let mut model = CpModel::new();
    let x = model.new_var(-10..=10);
    let y = model.new_var(0..=5);
    // The domains that Python's OR-Tools 9.15 writes for the same comparisons.
    let cases = [
        (x.le(5), vec![MIN, 5]),
        ((x + 5).le(MAX), vec![MIN, MAX]),
        ((x - 5).ge(MIN), vec![MIN, MAX]),
        (x.ge(MIN), vec![MIN, MAX]),
        (x.gt(MIN), vec![MIN + 1, MAX]),
        (x.lt(MAX), vec![MIN, MAX - 1]),
        ((x + 5).ge(0), vec![-5, MAX]),
        ((x + 5).ne(0), vec![MIN, -6, -4, MAX]),
        ((x - y + 4).gt(MAX - 3), vec![MAX - 6, MAX]),
        ((x + MAX).le(0), vec![MIN, MIN + 1]),
        (x.gt(y + MAX), vec![MAX, MAX]),
        (x.ne(MAX), vec![MIN, MAX - 1]),
        (x.ne(MIN), vec![MIN + 1, MAX]),
        ((x + MAX).le(-2), vec![MIN, MIN]),
        // Out-of-range bounds saturate into the limits, as in Python.
        ((x + MIN).ge(0), vec![MAX, MAX]),
        ((x + 5).le(MIN + 2), vec![MIN, MIN]),
    ];
    for (constraint, expected) in cases {
        assert_eq!(linear_domain(constraint), expected);
    }
    // Intervals that saturate into touching merge.
    let gap = Domain::from_intervals([(MIN, MAX - 4), (MAX - 2, MAX - 1)]);
    assert_eq!(linear_domain((x - 5).in_domain(gap)), [MIN, MAX]);
}

#[test]
fn constants_that_mean_infinity_panic_like_python() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=1);
    let comparisons: [fn(IntVar) -> Constraint; 6] = [
        |x| x.gt(i64::MAX),
        |x| x.ge(i64::MAX),
        |x| x.lt(i64::MIN),
        |x| x.le(i64::MIN),
        |x| x.eq(i64::MAX),
        |x| x.eq(i64::MIN),
    ];
    for compare in comparisons {
        let panic = std::panic::catch_unwind(|| compare(x)).unwrap_err();
        let message = panic.downcast_ref::<String>().unwrap();
        assert!(message.contains("is not supported"), "{message}");
    }
}

#[test]
fn merging_cancels_coefficients_before_overflow() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=1);
    let big = 1 << 62;
    let proto = LinearExpr::into_proto(x * big + x * big - x * big);
    assert_eq!((proto.vars, proto.coeffs), (vec![0], vec![big]));
}

#[test]
fn overflowing_arithmetic_leaves_the_expression_intact() {
    let mut model = CpModel::new();
    let [x, y, z, w] = std::array::from_fn(|_| model.new_var(0..=1));
    let mut expr = LinearExpr::from(x);
    let failed = std::panic::catch_unwind(AssertUnwindSafe(|| expr -= w + y * i64::MIN));
    assert!(failed.is_err());
    expr += z;
    assert_eq!(LinearExpr::into_proto(expr), LinearExpr::into_proto(x + z));

    let mut expr = x + y * i64::MAX + 1;
    let failed = std::panic::catch_unwind(AssertUnwindSafe(|| expr *= 2));
    assert!(failed.is_err());
    assert_eq!(
        LinearExpr::into_proto(expr),
        LinearExpr::into_proto(x + y * i64::MAX + 1)
    );

    let mut expr = LinearExpr::constant(i64::MAX);
    let flag = model.new_var(Bool);
    let failed = std::panic::catch_unwind(AssertUnwindSafe(|| expr += !flag));
    assert!(failed.is_err());
    assert_eq!(
        expr.into_proto(),
        LinearExpr::constant(i64::MAX).into_proto()
    );
}

#[test]
fn accumulating_in_place_keeps_the_value_of_each_operand() {
    let mut model = CpModel::new();
    let [x, y] = std::array::from_fn(|_| model.new_var(0..=1));
    let b = model.new_var(Bool);
    let other = 2 * x + y + 3;
    let mut expr = LinearExpr::default();
    expr += other.clone();
    expr -= &other;
    expr += &other;
    expr -= !b;
    expr += 4;
    // other - (1 - b) + 4
    let proto = expr.into_proto();
    assert_eq!(
        (proto.vars, proto.coeffs, proto.offset),
        (vec![0, 1, 2], vec![2, 1, 1], 6)
    );

    let doubled: LinearExpr = [&other, &other].into_iter().sum();
    assert_eq!(doubled.into_proto(), (2 * other.clone()).into_proto());

    let mut float: FloatExpr = [x, y].into_iter().sum();
    float -= !b;
    float += &other;
    assert_eq!(
        float,
        1.0 * x + 1.0 * y + 1.0 * b - 1.0 + 2.0 * x + 1.0 * y + 3.0
    );
}

#[test]
fn float_operators_mix_constants_and_expressions() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    let mut expr = 0.5 - x + (x + 0.25);
    expr += 2 * x;
    expr *= 2.0;
    let response = CpSolverResponse {
        status: CpSolverStatus::OPTIMAL.into(),
        solution: vec![3],
        ..Default::default()
    };
    let solution = response.solution().unwrap();
    assert_eq!(solution.value(&expr), 13.5);
    assert_eq!(solution.value(x - 0.5), 2.5);
}
