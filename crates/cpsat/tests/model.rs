//! The modeling API around the Protobuf message: handles, lookups, names,
//! objectives, and expression arithmetic.

use cpsat::prelude::*;
use cpsat::proto::constraint_proto::Constraint as Kind;
use cpsat::proto::{CpSolverResponse, LinearExpressionProto};

#[test]
fn literals_negate_and_name_their_variable() {
    let mut model = Model::new();
    model.int_var(0..=5);
    let b = model.bool_var();
    assert_eq!((b.index(), (!b).index()), (1, -2));
    assert_eq!(!!b, b);
    assert!((!b).is_negated());
    assert_eq!((!b).variable(), b.variable());
    assert_eq!(b.variable().index(), 1);

    model[!b].name = "b".into();
    assert_eq!(model[b].name, "b");
}

#[test]
fn looks_up_handles_by_index() {
    let mut model = Model::new();
    let x = model.int_var(0..=5);
    let b = model.bool_var();
    let one = model.constant(1);
    let start = model.int_var(0..=10);
    let interval = model.fixed_size_interval_var(start, 2);
    let constraint = model.add(x.le(3));

    assert_eq!(model.int_var_from_index(x.index()), Some(x));
    assert_eq!(model.int_var_from_index(9), None);
    assert_eq!(model.int_var_from_index(-1), None);
    assert_eq!(model.bool_var_from_index(b.index()), Some(b));
    assert_eq!(
        model.bool_var_from_index(one.index()).map(BoolVar::index),
        Some(2)
    );
    assert_eq!(model.bool_var_from_index(x.index()), None);
    assert_eq!(
        model.interval_var_from_index(interval.index()),
        Some(interval)
    );
    assert_eq!(model.interval_var_from_index(constraint.index()), None);
    assert_eq!((interval.index(), constraint.index()), (0, 1));
}

#[test]
fn constraint_builder_sets_enforcement_and_name() {
    let mut model = Model::new();
    let x = model.int_var(0..=5);
    let [a, b] = [(); 2].map(|()| model.bool_var());
    let constraint = model.add(x.ge(1).only_if(a).only_if(!b).with_name("positive"));
    assert_eq!(
        model[constraint].enforcement_literal,
        [a.index(), (!b).index()]
    );
    assert_eq!(model[constraint].name, "positive");

    model[x].name = "x".into();
    model.proto_mut().name = "model".into();
    model.remove_all_names();
    assert_eq!(model.proto().name, "");
    assert_eq!(model[x].name, "");
    assert_eq!(model[constraint].name, "");
}

#[test]
fn replaces_and_clears_the_objective() {
    let mut model = Model::new();
    let x = model.int_var(0..=5);
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
fn constants_and_true_literals_solve_to_their_values() {
    let mut model = Model::new();
    let seven = model.constant(7);
    let yes = model.bool_constant(true);
    let no = model.bool_constant(false);
    let x = model.int_var(0..=10);
    model.add(x.eq(seven + 1));
    model.add(Constraint::reservoir_with_active(
        0,
        5,
        [(x, 2, yes), (seven, -2, no)],
    ));
    let response = solve(model.proto(), &SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert_eq!(response.value(x), Some(8));
    assert_eq!(response.boolean_value(yes), Some(true));
    assert_eq!(response.boolean_value(no), Some(false));
}

#[test]
fn expressions_merge_terms_like_python() {
    let mut model = Model::new();
    let x = model.int_var(0..=5);
    let y = model.int_var(0..=5);
    let mut expr = LinearExpr::from(y);
    expr += 2 * x;
    expr -= y;
    expr *= 3;
    let proto = LinearExpressionProto::from(expr + 4);
    assert_eq!(
        (proto.vars, proto.coeffs, proto.offset),
        (vec![0], vec![6], 4)
    );

    let empty: LinearExpr = std::iter::empty::<IntVar>().sum();
    assert_eq!(
        LinearExpressionProto::from(empty),
        LinearExpressionProto::default()
    );
}

#[test]
fn circuit_takes_arcs_from_an_iterator() {
    let mut model = Model::new();
    let arcs: Vec<(i32, i32, BoolVar)> =
        (0..3).map(|i| (i, (i + 1) % 3, model.bool_var())).collect();
    model.add(Constraint::circuit(arcs));
    let response = solve(model.proto(), &SatParameters::default()).unwrap();
    assert_eq!(response.solution, [1, 1, 1]);
}

#[test]
#[should_panic(expected = "linear expression overflows i64")]
fn coefficient_overflow_panics() {
    let mut model = Model::new();
    let x = model.int_var(0..=1);
    let _ = x * i64::MAX * 2;
}

#[test]
#[should_panic(expected = "linear expression overflows i64")]
fn negating_the_minimum_coefficient_panics() {
    let mut model = Model::new();
    let x = model.int_var(0..=1);
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
    let mut model = Model::new();
    let x = model.int_var(-10..=10);
    let y = model.int_var(0..=5);
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
        // Out-of-range bounds saturate into the limits, as in Python.
        ((x + MIN).ge(0), vec![MAX, MAX]),
        ((x + 5).le(MIN + 2), vec![MIN, MIN]),
    ];
    for (constraint, expected) in cases {
        assert_eq!(linear_domain(constraint), expected);
    }
}

#[test]
fn constants_that_mean_infinity_panic_like_python() {
    let mut model = Model::new();
    let x = model.int_var(0..=1);
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
fn values_without_a_solution() {
    let mut model = Model::new();
    let x = model.int_var(0..=1);
    let response = CpSolverResponse::default();
    assert_eq!(response.value(x), None);
    assert_eq!(response.value(LinearExpr::constant(3)), Some(3));
}

#[test]
#[should_panic(expected = "linear expression overflows i64")]
fn evaluation_overflow_panics() {
    let mut model = Model::new();
    let x = model.int_var(0..=1);
    let response = CpSolverResponse {
        solution: vec![i64::MAX / 2],
        ..Default::default()
    };
    let _ = response.value(x * 4);
}

#[test]
fn adds_and_clears_assumptions() {
    let mut model = Model::new();
    let a = model.bool_var();
    let b = model.bool_var();
    model.add_assumption(a);
    model.add_assumptions([!b]);
    assert_eq!(model.proto().assumptions, [0, -2]);
    model.clear_assumptions();
    assert_eq!(model.proto().assumptions, Vec::<i32>::new());
}
