//! The modeling API around the Protobuf message: handles, lookups, intervals,
//! names, objectives, and assumptions.

use std::panic::AssertUnwindSafe;

use cpsat::prelude::*;
use cpsat::proto::constraint_proto::Constraint as Kind;
use cpsat::proto::{CpSolverStatus, SatParameters};

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
    let interval = model.add(Interval::fixed_size(start, 2));
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
fn adds_an_interval_as_a_typed_or_plain_constraint() {
    let mut model = CpModel::new();
    let start = model.new_var(0..=10);
    let interval = Interval::fixed_size(start, 2).with_name("task");
    let typed: IntervalVar = model.add(interval.clone());
    let plain: ConstraintIndex = model.add(Constraint::from(interval));
    let raw: ConstraintIndex = model.add(model[plain].clone());

    assert_eq!((typed.index(), plain.index(), raw.index()), (0, 1, 2));
    assert_eq!(ConstraintIndex::from(typed).index(), typed.index());
    assert_eq!(model[ConstraintIndex::from(typed)], model[plain]);
    assert_eq!(model[raw], model[plain]);
    // The plain handle forgets the kind, but the row still holds an interval.
    assert!(model.interval_var_from_index(plain.index()).is_some());
}

#[test]
fn constraint_builder_sets_enforcement_and_name() {
    let mut model = CpModel::new();
    let x = model.new_var((0..=5).with_name("x"));
    let [a, b] = std::array::from_fn(|_| model.new_var(Bool));
    let interval = model.add(Interval::fixed_size(x, 2).with_name("task"));
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
        (
            model[x].name.as_str(),
            model[ConstraintIndex::from(interval)].name.as_str()
        ),
        ("x", "task")
    );

    model.set_name("model");
    assert_eq!(model.proto().name, "model");
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
    let fixed = model.add(Interval::fixed_size(x, 3));
    let optional = model.add(Interval::new(x, 2, x + 2).only_enforce_if(present));
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
fn indexes_an_interval_as_its_message() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=9);
    let interval = model.add(Interval::fixed_size(x, 3));
    assert_eq!(*model[interval].start, LinearExpr::from(x).into_proto());
    assert_eq!(*model[interval].size, LinearExpr::constant(3).into_proto());
    assert_eq!(*model[interval].end, (x + 3).into_proto());
}

#[test]
#[should_panic(expected = "is not an interval of this model")]
fn indexing_a_replaced_interval_panics() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=9);
    let interval = model.add(Interval::fixed_size(x, 3));
    model.proto_mut().constraints[usize::try_from(interval.index()).unwrap()] =
        x.le(5).into_proto();
    let _ = &model[interval];
}

#[test]
#[should_panic(expected = "is not an interval of this model")]
fn interval_parts_panic_on_a_replaced_interval() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=9);
    let interval = model.add(Interval::fixed_size(x, 3));
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
    let interval = model.add(Interval::fixed_size(x, 3));
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
