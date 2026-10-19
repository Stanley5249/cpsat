//! Each constraint family solves to the answer it forces, which checks its
//! meaning where `equivalence.rs` checks only the message it writes.

use cpsat::prelude::*;

use crate::common::solve_optimal;

fn values(response: &CpSolverResponse, vars: impl IntoIterator<Item = IntVar>) -> Vec<i64> {
    vars.into_iter()
        .map(|var| response.value(var).unwrap())
        .collect()
}

#[test]
fn element_picks_the_index_of_the_target() {
    let mut model = Model::new();
    let index = model.int_var(0..=5);
    model.add(Constraint::element(index, [10, 20, 30], 30));
    model.minimize(index);
    let response = solve_optimal(&model);
    assert_eq!(response.value(index), Some(2));
}

#[test]
fn inverse_inverts_the_permutation() {
    let mut model = Model::new();
    let direct = [(); 3].map(|()| model.int_var(0..=2));
    let inverse = [(); 3].map(|()| model.int_var(0..=2));
    model.add(Constraint::inverse(direct, inverse));
    for (var, value) in direct.into_iter().zip([2, 0, 1]) {
        model.add(var.eq(value));
    }
    model.minimize(inverse.into_iter().sum::<LinearExpr>());
    let response = solve_optimal(&model);
    assert_eq!(values(&response, inverse), [1, 2, 0]);
}

#[test]
fn automaton_rejects_consecutive_ones() {
    let mut model = Model::new();
    let labels = [(); 3].map(|()| model.int_var(0..=1));
    // State 5 follows a 0 and state 7 follows a 1, so no label equals a state.
    let transitions = [(5, 0, 5), (5, 1, 7), (7, 0, 5)];
    model.add(Constraint::automaton(labels, 5, [5, 7], transitions));
    model.maximize(labels.into_iter().sum::<LinearExpr>());
    let response = solve_optimal(&model);
    assert_eq!(values(&response, labels), [1, 0, 1]);
}

#[test]
fn allowed_assignments_pick_a_row() {
    let mut model = Model::new();
    let x = model.int_var(0..=5);
    let y = model.int_var(0..=5);
    model.add(Constraint::allowed_assignments([x, y], [[1, 2], [3, 4]]));
    model.maximize(x + y);
    let response = solve_optimal(&model);
    assert_eq!(values(&response, [x, y]), [3, 4]);
}

#[test]
fn forbidden_assignments_avoid_every_row() {
    let mut model = Model::new();
    let x = model.int_var(0..=1);
    let y = model.int_var(0..=1);
    model.add(Constraint::forbidden_assignments(
        [x, y],
        [[0, 0], [0, 1], [1, 0]],
    ));
    model.minimize(x + y);
    let response = solve_optimal(&model);
    assert_eq!(values(&response, [x, y]), [1, 1]);
}

#[test]
fn reservoir_drains_only_after_filling() {
    let mut model = Model::new();
    let fill = model.int_var(0..=10);
    let drain = model.int_var(0..=10);
    model.add(Constraint::reservoir(0, 5, [(fill, 3), (drain, -3)]));
    model.add(fill.eq(4));
    model.minimize(drain);
    let response = solve_optimal(&model);
    assert_eq!(response.value(drain), Some(4));
}

#[test]
fn reservoir_fills_again_only_after_draining() {
    let mut model = Model::new();
    let first = model.int_var(0..=10);
    let drain = model.int_var(0..=10);
    let second = model.int_var(0..=10);
    model.add(Constraint::reservoir(
        0,
        5,
        [(first, 3), (drain, -3), (second, 3)],
    ));
    model.add(first.eq(0));
    model.add(drain.eq(5));
    model.minimize(second);
    let response = solve_optimal(&model);
    assert_eq!(response.value(second), Some(5));
}

#[test]
fn no_overlap_2d_stacks_the_squares() {
    let mut model = Model::new();
    let mut rectangles = Vec::new();
    let mut y_starts = Vec::new();
    for _ in 0..2 {
        let x_start = model.int_var(0..=0);
        let y_start = model.int_var(0..=2);
        rectangles.push((
            model.fixed_size_interval_var(x_start, 2),
            model.fixed_size_interval_var(y_start, 2),
        ));
        y_starts.push(y_start);
    }
    model.add(Constraint::no_overlap_2d(rectangles));
    model.add(y_starts[0].le(y_starts[1]));
    model.minimize(y_starts[1]);
    let response = solve_optimal(&model);
    assert_eq!(values(&response, y_starts), [0, 2]);
}

#[test]
fn multiple_circuit_takes_two_routes_through_the_depot() {
    let mut model = Model::new();
    // The routes 0 -> 1 -> 0 and 0 -> 2 -> 0 cost 4 in total, and the single
    // circuit that `circuit` would force costs 12.
    let costs = [
        (0, 1, 1),
        (1, 0, 1),
        (0, 2, 1),
        (2, 0, 1),
        (1, 2, 10),
        (2, 1, 10),
    ];
    let arcs = costs.map(|(tail, head, _)| (tail, head, model.bool_var()));
    model.add(Constraint::multiple_circuit(arcs));
    model.minimize(
        arcs.iter()
            .zip(costs)
            .map(|(&(_, _, arc), (_, _, cost))| arc * cost)
            .sum::<LinearExpr>(),
    );
    let response = solve_optimal(&model);
    let used = arcs.map(|(_, _, arc)| response.boolean_value(arc).unwrap());
    assert_eq!(used, [true, true, true, true, false, false]);
}

#[test]
fn cumulative_runs_two_tasks_at_a_time() {
    let mut model = Model::new();
    let starts = [(); 3].map(|()| model.int_var(0..=10));
    let tasks: Vec<(IntervalVar, i64)> = starts
        .iter()
        .map(|&start| (model.fixed_size_interval_var(start, 2), 1))
        .collect();
    model.add(Constraint::cumulative(2, tasks));
    model.minimize(starts.into_iter().sum::<LinearExpr>());
    let response = solve_optimal(&model);
    let mut starts = values(&response, starts);
    starts.sort_unstable();
    assert_eq!(starts, [0, 0, 2]);
}

/// The encoding of Python's `add_map_domain(x, literals, 3)`, which
/// `equivalence.rs::map_domain` compares with Python's model.
fn map_domain(model: &mut Model, x: IntVar, literals: [BoolVar; 2]) {
    for (value, literal) in (3..).zip(literals) {
        model.add(x.eq(value).only_if(literal));
        model.add(x.ne(value).only_if(!literal));
    }
}

#[test]
fn map_domain_sets_only_the_literal_of_the_value() {
    for (fixed, expected) in [(3, [true, false]), (4, [false, true]), (7, [false, false])] {
        let mut model = Model::new();
        let x = model.int_var(0..=10);
        let literals = [model.bool_var(), model.bool_var()];
        map_domain(&mut model, x, literals);
        model.add(x.eq(fixed));
        let response = solve_optimal(&model);
        assert_eq!(
            literals.map(|literal| response.boolean_value(literal).unwrap()),
            expected,
            "x = {fixed}"
        );
    }
}

#[test]
fn map_domain_fixes_the_value_of_a_true_literal() {
    let mut model = Model::new();
    let x = model.int_var(0..=10);
    let literals = [model.bool_var(), model.bool_var()];
    map_domain(&mut model, x, literals);
    model.add(literals[1].eq(1));
    model.maximize(x);
    assert_eq!(solve_optimal(&model).value(x), Some(4));
}

/// Python's `add(True)` and `add(False)` write a constraint on a constant
/// literal, so these compare results rather than messages.
#[test]
fn empty_bool_and_holds_and_empty_bool_or_fails() {
    let mut model = Model::new();
    model.add(Constraint::bool_and([]));
    assert_eq!(solve_optimal(&model).status(), CpSolverStatus::OPTIMAL);

    model.add(Constraint::bool_or([]));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::INFEASIBLE);
}

#[test]
fn enforced_empty_bool_or_forces_its_literal_false() {
    let mut model = Model::new();
    let b = model.bool_var();
    model.add(Constraint::bool_or([]).only_if(b));
    model.maximize(b);
    assert_eq!(solve_optimal(&model).boolean_value(b), Some(false));
}
