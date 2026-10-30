//! Each constraint family solves to the answer it forces, which checks its
//! meaning where `equivalence.rs` checks only the message it writes.

use cpsat::prelude::*;
use cpsat::proto::{CpSolverResponse, CpSolverStatus, SatParameters};

use crate::common::{map_domain, solve_optimal};

fn values(response: &CpSolverResponse, vars: impl IntoIterator<Item = IntVar>) -> Vec<i64> {
    let solution = response.solution().unwrap();
    vars.into_iter().map(|var| solution.value(var)).collect()
}

#[test]
fn element_picks_the_index_of_the_target() {
    let mut model = CpModel::new();
    let index = model.new_var(0..=5);
    model.add(Constraint::element(index, [10, 20, 30], 30));
    model.minimize(index);
    let response = solve_optimal(&model);
    let solution = response.solution().unwrap();
    assert_eq!(solution.value(index), 2);
}

#[test]
fn inverse_inverts_the_permutation() {
    let mut model = CpModel::new();
    let direct: [IntVar; 3] = std::array::from_fn(|_| model.new_var(0..=2));
    let inverse: [IntVar; 3] = std::array::from_fn(|_| model.new_var(0..=2));
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
    let mut model = CpModel::new();
    let labels: [IntVar; 3] = std::array::from_fn(|_| model.new_var(0..=1));
    // State 5 follows a 0 and state 7 follows a 1, so no label equals a state.
    let transitions = [(5, 0, 5), (5, 1, 7), (7, 0, 5)];
    model.add(Constraint::automaton(labels, 5, [5, 7], transitions));
    model.maximize(labels.into_iter().sum::<LinearExpr>());
    let response = solve_optimal(&model);
    assert_eq!(values(&response, labels), [1, 0, 1]);
}

#[test]
fn allowed_assignments_pick_a_row() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    let y = model.new_var(0..=5);
    model.add(Constraint::allowed_assignments([x, y], [[1, 2], [3, 4]]));
    model.maximize(x + y);
    let response = solve_optimal(&model);
    assert_eq!(values(&response, [x, y]), [3, 4]);
}

#[test]
fn forbidden_assignments_avoid_every_row() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=1);
    let y = model.new_var(0..=1);
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
    let mut model = CpModel::new();
    let fill = model.new_var(0..=10);
    let drain = model.new_var(0..=10);
    model.add(Constraint::reservoir(0, 5, [(fill, 3), (drain, -3)]));
    model.add(fill.eq(4));
    model.minimize(drain);
    let response = solve_optimal(&model);
    let solution = response.solution().unwrap();
    assert_eq!(solution.value(drain), 4);
}

#[test]
fn no_overlap_2d_stacks_the_squares() {
    let mut model = CpModel::new();
    let mut rectangles = Vec::new();
    let mut y_starts = Vec::new();
    for _ in 0..2 {
        let x_start = model.new_var(0..=0);
        let y_start = model.new_var(0..=2);
        rectangles.push((
            model.add(Interval::fixed_size(x_start, 2)),
            model.add(Interval::fixed_size(y_start, 2)),
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
    let mut model = CpModel::new();
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
    let arcs = costs.map(|(tail, head, _)| (tail, head, model.new_var(Bool)));
    model.add(Constraint::multiple_circuit(arcs));
    model.minimize(
        arcs.iter()
            .zip(costs)
            .map(|(&(_, _, arc), (_, _, cost))| arc * cost)
            .sum::<LinearExpr>(),
    );
    let response = solve_optimal(&model);
    let solution = response.solution().unwrap();
    let used = arcs.map(|(_, _, arc)| solution.value(arc));
    assert_eq!(used, [true, true, true, true, false, false]);
}

#[test]
fn cumulative_runs_two_tasks_at_a_time() {
    let mut model = CpModel::new();
    let starts: [IntVar; 3] = std::array::from_fn(|_| model.new_var(0..=10));
    let tasks: Vec<(IntervalVar, i64)> = starts
        .iter()
        .map(|&start| (model.add(Interval::fixed_size(start, 2)), 1))
        .collect();
    model.add(Constraint::cumulative(2, tasks));
    model.minimize(starts.into_iter().sum::<LinearExpr>());
    let response = solve_optimal(&model);
    let mut starts = values(&response, starts);
    starts.sort_unstable();
    assert_eq!(starts, [0, 0, 2]);
}

#[test]
fn map_domain_sets_only_the_literal_of_the_value() {
    for (fixed, expected) in [(3, [true, false]), (4, [false, true]), (7, [false, false])] {
        let mut model = CpModel::new();
        let x = model.new_var(0..=10);
        let literals = [model.new_var(Bool), model.new_var(Bool)];
        map_domain(&mut model, x, literals);
        model.add(x.eq(fixed));
        let response = solve_optimal(&model);
        let solution = response.solution().unwrap();
        assert_eq!(
            literals.map(|literal| solution.value(literal)),
            expected,
            "x = {fixed}"
        );
    }
}

/// Python's `add(True)` and `add(False)` write a constraint on a constant
/// literal, so these compare results rather than messages.
#[test]
fn empty_bool_and_holds_and_empty_bool_or_fails() {
    let mut model = CpModel::new();
    model.add(Constraint::bool_and([]));
    assert_eq!(solve_optimal(&model).status(), CpSolverStatus::OPTIMAL);

    model.add(Constraint::bool_or([]));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::INFEASIBLE);
}

#[test]
fn enforced_empty_bool_or_forces_its_literal_false() {
    let mut model = CpModel::new();
    let b = model.new_var(Bool);
    model.add(Constraint::bool_or([]).only_enforce_if(b));
    model.maximize(b);
    assert!(!solve_optimal(&model).solution().unwrap().value(b));
}

/// The schema stores rows of zero values as no rows at all, so forbidding the
/// empty tuple would silently forbid nothing.
#[test]
#[should_panic(expected = "a table with rows needs an expression")]
fn table_rejects_rows_without_expressions() {
    let _ = Constraint::forbidden_assignments(Vec::<LinearExpr>::new(), [Vec::<i64>::new()]);
}

#[test]
#[should_panic(expected = "a table row has one value per expression")]
fn table_rejects_a_ragged_row() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=5);
    let y = model.new_var(0..=5);
    // Flattened, these rows would read as [1, 2] and [3, 4].
    model.add(Constraint::allowed_assignments(
        [x, y],
        [vec![1, 2, 3], vec![4]],
    ));
}

#[test]
fn min_equality_takes_the_smallest() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=10);
    let y = model.new_var(0..=10);
    let target = model.new_var(-10..=10);
    model.add(x.eq(7));
    model.add(y.eq(4));
    model.add(Constraint::min_equality(target, [x + 1, y - 2]));
    assert_eq!(solve_optimal(&model).solution().unwrap().value(target), 2);
}

#[test]
fn abs_equality_drops_the_sign() {
    let mut model = CpModel::new();
    let x = model.new_var(-10..=10);
    let target = model.new_var(0..=10);
    model.add(x.eq(-6));
    model.add(Constraint::abs_equality(target, x));
    assert_eq!(solve_optimal(&model).solution().unwrap().value(target), 6);
}

#[test]
fn modulo_equality_keeps_the_sign_of_the_dividend() {
    let mut model = CpModel::new();
    let x = model.new_var(-10..=10);
    let target = model.new_var(-10..=10);
    model.add(x.eq(-7));
    model.add(Constraint::modulo_equality(target, x, 3));
    assert_eq!(solve_optimal(&model).solution().unwrap().value(target), -1);
}

#[test]
fn bool_xor_needs_an_odd_count() {
    let mut model = CpModel::new();
    let literals: [BoolVar; 3] = std::array::from_fn(|_| model.new_var(Bool));
    model.add(Constraint::bool_xor(literals));
    model.add(literals[0].eq(1));
    model.add(literals[1].eq(1));
    let response = solve_optimal(&model);
    let solution = response.solution().unwrap();
    assert!(solution.value(literals[2]));
}
