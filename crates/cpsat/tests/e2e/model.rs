//! The modeling API around the Protobuf message: handles, lookups, names,
//! objectives, and expression arithmetic.

use std::panic::AssertUnwindSafe;

use cpsat::prelude::*;
use cpsat::proto::constraint_proto::Constraint as Kind;
use cpsat::proto::{CpSolverResponse, CpSolverStatus, LinearExpressionProto, SatParameters};

#[test]
fn literals_negate_and_name_their_variable() {
    let mut model = CpModel::new();
    model.new_var(0..=5);
    let b = model.new_var(Bool.with_name("b"));
    assert_eq!((b.index(), (!b).index()), (1, -2));
    assert_eq!(!!b, b);
    assert!((!b).is_negated());
    assert_eq!((!b).to_int_var(), b.to_int_var());
    assert_eq!(b.to_int_var().index(), 1);

    assert_eq!(model[(!b).to_int_var()].name, "b");
}

#[test]
fn looks_up_handles_by_index() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    let b = model.new_var(Bool);
    let one = model.new_var(1..=1);
    let start = model.new_var(0..=10);
    let interval = model.new_var(Interval::fixed_size(start, 2));
    let constraint = model.add(x.le(3));

    assert_eq!(model.int_var_from_index(x.index()), Some(x));
    assert_eq!(model.int_var_from_index(9), None);
    assert_eq!(model.int_var_from_index(-1), None);
    let literal = |index| {
        model
            .int_var_from_index(index)
            .and_then(|var| model.to_bool_var(var))
    };
    assert_eq!(literal(b.index()), Some(b));
    assert_eq!(literal(one.index()).map(BoolVar::index), Some(2));
    assert_eq!(literal(x.index()), None);
    assert_eq!(
        model.interval_var_from_index(interval.index()),
        Some(interval)
    );
    assert_eq!(model.interval_var_from_index(constraint.index()), None);
    assert_eq!((interval.index(), constraint.index()), (0, 1));
}

#[test]
fn constraint_builder_sets_enforcement_and_name() {
    let mut model = CpModel::new();
    let x = model.new_var((0..=5).with_name("x"));
    let [a, b] = std::array::from_fn(|_| model.new_var(Bool));
    let interval = model.new_var(Interval::fixed_size(x, 2).with_name("task"));
    let constraint = model.add(
        x.ge(1)
            .only_enforce_if(a)
            .only_enforce_if(!b)
            .with_name("positive"),
    );
    assert_eq!(
        model[constraint].enforcement_literal,
        [a.index(), (!b).index()]
    );
    assert_eq!(model[constraint].name, "positive");
    assert_eq!(
        (model[x].name.as_str(), model[interval].name.as_str()),
        ("x", "task")
    );

    model.proto_mut().name = "model".into();
    model.clear_names();
    assert_eq!(model.proto().name, "");
    assert_eq!(model[x].name, "");
    assert_eq!(model[constraint].name, "");
}

#[test]
fn replaces_and_clears_the_objective() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    assert!(!model.has_objective());
    model.maximize(x);
    model.minimize(2 * x + 1);
    let objective = model.proto().objective.as_option().unwrap();
    assert_eq!(
        (objective.coeffs.as_slice(), objective.offset),
        (&[2][..], 1.0)
    );
    assert!((objective.scaling_factor - 1.0).abs() < f64::EPSILON);
    model.clear_objective();
    assert!(!model.has_objective());
}

#[test]
fn a_panicking_objective_keeps_the_previous_one() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    model.minimize(2 * x + 1);
    let overflowing = x * i64::MAX + x;
    let failed = std::panic::catch_unwind(AssertUnwindSafe(|| model.minimize(overflowing)));
    assert!(failed.is_err());
    assert!(model.has_objective());
    let objective = model.proto().objective.as_option().unwrap();
    assert_eq!(
        (objective.coeffs.as_slice(), objective.offset),
        (&[2][..], 1.0)
    );
}

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
fn constants_and_true_literals_solve_to_their_values() {
    let mut model = CpModel::new();
    let seven = model.new_var(7..=7);
    let yes = model.new_var(true);
    let no = model.new_var(false);
    let x = model.new_var(0..=10);
    model.add(x.eq(seven + 1));
    model.add(Constraint::reservoir_with_active(
        0,
        5,
        [(x, 2, yes), (seven, -2, no)],
    ));
    let response = model.solve(&SatParameters::default()).unwrap();
    let solution = response.solution().unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert_eq!(solution.value(x), 8);
    assert!(solution.value(yes));
    assert!(!solution.value(no));
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
fn circuit_takes_arcs_from_an_iterator() {
    let mut model = CpModel::new();
    let arcs: Vec<(i32, i32, BoolVar)> = (0..3)
        .map(|i| (i, (i + 1) % 3, model.new_var(Bool)))
        .collect();
    model.add(Constraint::circuit(arcs));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.solution, [1, 1, 1]);
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
fn a_response_without_a_solution_has_none() {
    assert_eq!(CpSolverResponse::default().solution(), None);
}

#[test]
#[should_panic(expected = "linear expression overflows i64")]
fn evaluation_overflow_panics() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=1);
    let response = CpSolverResponse {
        status: CpSolverStatus::OPTIMAL.into(),
        solution: vec![i64::MAX / 2],
        ..Default::default()
    };
    let solution = response.solution().unwrap();
    let _ = solution.value(x * 4);
}

#[test]
fn evaluation_cancels_terms_before_overflow() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=3_000_000_000_000_000_000);
    let response = CpSolverResponse {
        status: CpSolverStatus::OPTIMAL.into(),
        solution: vec![3_000_000_000_000_000_000],
        ..Default::default()
    };
    let solution = response.solution().unwrap();
    assert_eq!(solution.value(4 * x - 3 * x), 3_000_000_000_000_000_000);
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
fn adds_and_clears_assumptions() {
    let mut model = CpModel::new();
    let a = model.new_var(Bool);
    let b = model.new_var(Bool);
    model.add_assumptions([a]);
    model.add_assumptions([!b]);
    assert_eq!(model.proto().assumptions, [0, -2]);
    model.clear_assumptions();
    assert_eq!(model.proto().assumptions, Vec::<i32>::new());
}

#[test]
fn reads_interval_parts_like_python() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=9);
    let present = model.new_var(Bool);
    let fixed = model.new_var(Interval::fixed_size(x, 3));
    let optional = model.new_var(Interval::new(x, 2, x + 2).only_enforce_if(present));
    let parts = |interval| {
        [
            model.interval_start(interval),
            model.interval_size(interval),
            model.interval_end(interval),
        ]
        .map(LinearExpr::into_proto)
    };
    let expected =
        |size: i64| [x.into(), LinearExpr::constant(size), x + size].map(LinearExpr::into_proto);
    assert_eq!(parts(fixed), expected(3));
    assert_eq!(parts(optional), expected(2));
    assert_eq!(model.interval_presence_literals(fixed).count(), 0);
    assert_eq!(
        model
            .interval_presence_literals(optional)
            .collect::<Vec<_>>(),
        [present]
    );
}

#[test]
#[should_panic(expected = "is not an interval of this model")]
fn interval_parts_panic_on_a_replaced_interval() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=9);
    let interval = model.new_var(Interval::fixed_size(x, 3));
    model.proto_mut().constraints[usize::try_from(interval.index()).unwrap()] =
        x.le(5).into_proto();
    let _ = model.interval_start(interval);
}

#[test]
#[should_panic(expected = "vars and coeffs of unequal lengths")]
fn interval_parts_panic_on_a_ragged_expression() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=9);
    let y = model.new_var(0..=9);
    let interval = model.new_var(Interval::fixed_size(x, 3));
    let constraint = &mut model.proto_mut().constraints[usize::try_from(interval.index()).unwrap()];
    let Some(Kind::Interval(proto)) = &mut constraint.constraint else {
        unreachable!("Interval::fixed_size writes an interval");
    };
    proto.start.get_or_insert_default().vars.push(y.index());
    assert!(model.validate().is_err());
    let _ = model.interval_start(interval);
}

#[test]
fn integer_and_float_objectives_replace_each_other() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    model.maximize(x);
    model.minimize(0.5 * x + 1.0 - x * 2.0);
    assert!(!model.proto().objective.is_set());
    let objective = model.proto().floating_point_objective.as_option().unwrap();
    assert_eq!(
        (objective.coeffs.as_slice(), objective.offset),
        (&[-1.5][..], 1.0)
    );
    model.minimize(x);
    assert!(!model.proto().floating_point_objective.is_set());
    assert!(model.proto().objective.is_set());
    model.clear_objective();
    assert!(!model.has_objective());
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

#[test]
fn a_zero_one_variable_serves_as_a_literal() {
    let mut model = CpModel::new();
    let flag = model.new_var(0..=1);
    let wide = model.new_var(0..=2);
    let literal = model.to_bool_var(flag).unwrap();
    assert_eq!(literal.to_int_var(), flag);
    assert_eq!(model.to_bool_var(wide), None);
    model.add(Constraint::bool_or([literal]));
    let response = model.solve(&SatParameters::default()).unwrap();
    let solution = response.solution().unwrap();
    assert_eq!(solution.value(flag), 1);
}

#[test]
fn only_domains_within_zero_and_one_are_literals() {
    let mut model = CpModel::new();
    for domain in [Domain::new(0, 0), Domain::new(1, 1)] {
        let fixed = model.new_var(domain);
        assert!(model.to_bool_var(fixed).is_some());
    }
    let empty = model.new_var(Domain::default());
    assert_eq!(model.to_bool_var(empty), None);
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn to_bool_var_panics_without_the_variable() {
    let mut larger = CpModel::new();
    let _ = larger.new_var(Bool);
    let x = larger.new_var(Bool).to_int_var();
    let _ = CpModel::new().to_bool_var(x);
}

#[test]
fn a_bool_creates_a_fixed_literal() {
    let mut model = CpModel::new();
    let yes = model.new_var(true.with_name("yes"));
    let no = model.new_var(false);
    assert_eq!(model[yes.to_int_var()].domain, [1, 1]);
    assert_eq!(model[yes.to_int_var()].name, "yes");
    assert_eq!(model[no.to_int_var()].domain, [0, 0]);
}

/// Handles order by index, so they key sorted collections, while the
/// comparison methods still build constraints.
#[test]
fn handles_order_by_index_and_compare_into_constraints() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    let y = model.new_var(0..=5);
    let mut vars = [y, x];
    vars.sort_unstable();
    assert_eq!(vars, [x, y]);
    let keyed: std::collections::BTreeMap<IntVar, i64> = [(y, 2), (x, 1)].into();
    assert_eq!(keyed.keys().copied().collect::<Vec<_>>(), [x, y]);

    let constraint: Constraint = x.lt(y);
    model.add(constraint);
    model.maximize(x);
    let response = model.solve(&SatParameters::default()).unwrap();
    let solution = response.solution().unwrap();
    assert_eq!(solution.value(x), 4);
}
