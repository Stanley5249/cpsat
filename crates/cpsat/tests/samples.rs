//! Rust versions of the official OR-Tools v9.15 CP-SAT samples in
//! `ortools/sat/samples`. Each test asserts what the Python sample prints,
//! checked by running the samples with `ortools==9.15.6755`.

use std::ops::ControlFlow;

use cpsat::proto::{CpModelProto, CpSolverStatus, LinearConstraintProto, SatParameters};
use cpsat::{Event, EventSet, Solver};

/// `x != y`, which Python encodes as `x - y` outside `{0}`.
fn add_not_equal(model: &mut CpModelProto, x: i32, y: i32) {
    model.add_constraint(LinearConstraintProto::new(
        &[(x, 1), (y, -1)],
        vec![i64::MIN, -1, 1, i64::MAX],
    ));
}

/// The three variables in `[0, 2]` that most samples share.
fn three_vars(model: &mut CpModelProto) -> [i32; 3] {
    [0; 3].map(|_| model.add_int_var(0, 2))
}

/// `simple_sat_program.py`
#[test]
fn simple_sat_program() {
    let mut model = CpModelProto::default();
    let [x, y, z] = three_vars(&mut model);
    add_not_equal(&mut model, x, y);

    let response = cpsat::solve(&model, &SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert_ne!(response.value(x), response.value(y));
    assert!(response.value(z).is_some_and(|z| (0..=2).contains(&z)));
}

/// `cp_sat_example.py`
#[test]
fn cp_sat_example() {
    let mut model = CpModelProto::default();
    let [x, y, z] = [0; 3].map(|_| model.add_int_var(0, 50));
    model.add_linear(&[(x, 2), (y, 7), (z, 3)], i64::MIN, 50);
    model.add_linear(&[(x, 3), (y, -5), (z, 7)], i64::MIN, 45);
    model.add_linear(&[(x, 5), (y, 2), (z, -6)], i64::MIN, 37);
    model.maximize(&[(x, 2), (y, 2), (z, 3)]);

    let response = cpsat::solve(&model, &SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert!((response.objective_value - 35.0).abs() < 1e-9);
    assert_eq!(response.solution, [7, 3, 5]);
}

/// `solve_and_print_intermediate_solutions_sample_sat.py`
#[test]
fn solve_and_print_intermediate_solutions() {
    let mut model = CpModelProto::default();
    let [x, y, z] = three_vars(&mut model);
    add_not_equal(&mut model, x, y);
    model.maximize(&[(x, 1), (y, 2), (z, 3)]);

    let mut objectives = Vec::new();
    let response = Solver::default()
        .on_event(EventSet::SOLUTION, |event| {
            if let Event::Solution(view) = event {
                objectives.push(view.objective_value);
            }
            ControlFlow::Continue(())
        })
        .solve(&model)
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert_eq!(response.solution, [1, 2, 2]);
    assert_eq!(objectives.last(), Some(&11.0));
    assert!(objectives.is_sorted(), "{objectives:?}");
}

/// `search_for_all_solutions_sample_sat.py`
#[test]
fn search_for_all_solutions() {
    let mut model = CpModelProto::default();
    let [x, y, _] = three_vars(&mut model);
    add_not_equal(&mut model, x, y);
    let params = SatParameters::default().with_enumerate_all_solutions(true);

    let mut solutions = Vec::new();
    let response = Solver::new(params)
        .on_event(EventSet::SOLUTION, |event| {
            if let Event::Solution(view) = event {
                solutions.push(view.solution.to_vec());
            }
            ControlFlow::Continue(())
        })
        .solve(&model)
        .unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    solutions.sort();
    solutions.dedup();
    assert_eq!(solutions.len(), 18);
    assert!(solutions.iter().all(|solution| solution[0] != solution[1]));
}

/// `stop_after_n_solutions_sample_sat.py`
#[test]
fn stop_after_n_solutions() {
    let mut model = CpModelProto::default();
    three_vars(&mut model);
    let params = SatParameters::default().with_enumerate_all_solutions(true);

    let mut count = 0;
    let response = Solver::new(params)
        .on_event(EventSet::SOLUTION, |_| {
            count += 1;
            if count >= 5 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .solve(&model)
        .unwrap();
    assert_eq!(count, 5);
    assert_eq!(response.status(), CpSolverStatus::FEASIBLE);
}

/// `assumptions_sample_sat.py`
#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "mirrors the names in the official sample"
)]
fn assumptions() {
    let mut model = CpModelProto::default();
    let [x, y, z] = [0; 3].map(|_| model.add_int_var(0, 10));
    let [a, b, c] = [0; 3].map(|_| model.add_bool_var());
    for (literal, left, right) in [(a, x, y), (b, y, z), (c, z, x)] {
        model
            .add_linear(&[(left, 1), (right, -1)], 1, i64::MAX)
            .enforcement_literal
            .push(literal);
    }
    model.assumptions = vec![a, b, c];

    let response = cpsat::solve(&model, &SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::INFEASIBLE);
    let mut core = response.sufficient_assumptions_for_infeasibility.clone();
    core.sort_unstable();
    assert_eq!(core, [a, b, c]);
}

/// `solution_hinting_sample_sat.py`
#[test]
fn solution_hinting() {
    let mut model = CpModelProto::default();
    let [x, y, z] = three_vars(&mut model);
    add_not_equal(&mut model, x, y);
    model.maximize(&[(x, 1), (y, 2), (z, 3)]);
    model.add_hint(x, 1);
    model.add_hint(y, 2);

    let response = cpsat::solve(&model, &SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert!((response.objective_value - 11.0).abs() < 1e-9);
    assert_eq!(response.solution, [1, 2, 2]);
}
