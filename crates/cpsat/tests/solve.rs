//! Model and solver-control capabilities, mirroring Python's `CpModel` and
//! `CpSolver`.

mod common;

use buffa::{Message, MessageField};
use common::{linear, maximize, single_worker, small_model, var};
use cpsat::proto::{CpModelProto, CpSolverStatus, PartialVariableAssignment, SatParameters};

#[test]
fn solves_to_optimality() {
    let response = cpsat::solve(&small_model(), &SatParameters::default()).unwrap();
    assert_eq!(response.status, CpSolverStatus::OPTIMAL);
    assert_eq!(response.solution, [3, 1]);
    assert!((response.objective_value - 9.0).abs() < 1e-9);
    assert!((response.best_objective_bound - 9.0).abs() < 1e-9);
}

#[test]
fn reports_infeasible() {
    let mut model = CpModelProto::default();
    let x = var(&mut model, 0, 3);
    linear(&mut model, &[(x, 1)], 5, 10);
    let response = cpsat::solve(&model, &SatParameters::default()).unwrap();
    assert_eq!(response.status, CpSolverStatus::INFEASIBLE);
}

#[test]
fn reports_model_invalid_without_error() {
    let mut model = CpModelProto::default();
    var(&mut model, 5, 1);
    let response = cpsat::solve(&model, &SatParameters::default()).unwrap();
    assert_eq!(response.status, CpSolverStatus::MODEL_INVALID);
}

#[test]
fn resolves_after_model_change() {
    let mut model = small_model();
    let first = cpsat::solve(&model, &single_worker()).unwrap();
    assert_eq!(first.solution, [3, 1]);

    // Tighten x <= 1 through its domain, the Proto equivalent of Python's
    // IntVar domain changes.
    model.variables[0].domain = vec![0, 1];
    let second = cpsat::solve(&model, &single_worker()).unwrap();
    assert_eq!(second.status, CpSolverStatus::OPTIMAL);
    assert_eq!(second.solution, [0, 2]);

    // Replace the objective: maximize x alone.
    maximize(&mut model, &[(0, 1)]);
    let third = cpsat::solve(&model, &single_worker()).unwrap();
    assert!((third.objective_value - 1.0).abs() < 1e-9);

    // Remove the second constraint, so y is bounded only by x + y <= 4.
    model.constraints.truncate(1);
    maximize(&mut model, &[(1, 1)]);
    let fourth = cpsat::solve(&model, &single_worker()).unwrap();
    assert_eq!(fourth.solution[1], 4);
}

#[test]
fn clone_and_serialize_round_trip() {
    let model = small_model();
    let copy = model.clone();
    let decoded = CpModelProto::decode_from_slice(&model.encode_to_vec()).unwrap();
    assert_eq!(copy, decoded);
    let response = cpsat::solve(&decoded, &SatParameters::default()).unwrap();
    assert_eq!(response.status, CpSolverStatus::OPTIMAL);
}

#[test]
fn keeps_a_feasible_hint() {
    let mut model = small_model();
    model.solution_hint = MessageField::some(PartialVariableAssignment {
        vars: vec![0, 1],
        values: vec![3, 1],
        ..Default::default()
    });
    let params = SatParameters {
        stop_after_first_solution: Some(true),
        num_workers: Some(1),
        ..Default::default()
    };
    let response = cpsat::solve(&model, &params).unwrap();
    assert_eq!(response.solution, [3, 1]);
}

#[test]
fn returns_sufficient_assumptions() {
    // a, b, c are literals; a + b + c <= 1 conflicts with assuming a and b.
    let mut model = CpModelProto::default();
    let a = var(&mut model, 0, 1);
    let b = var(&mut model, 0, 1);
    let c = var(&mut model, 0, 1);
    linear(&mut model, &[(a, 1), (b, 1), (c, 1)], 0, 1);
    model.assumptions = vec![a, b, c];
    let response = cpsat::solve(&model, &single_worker()).unwrap();
    assert_eq!(response.status, CpSolverStatus::INFEASIBLE);
    // Python 9.15 returns the same core for this model.
    assert_eq!(response.sufficient_assumptions_for_infeasibility, [a, b]);
}

#[test]
fn honors_time_limit() {
    let params = SatParameters {
        max_time_in_seconds: Some(0.5),
        ..Default::default()
    };
    let response = cpsat::solve(&common::hard_model(), &params).unwrap();
    assert!(response.wall_time < 5.0, "wall time {}", response.wall_time);
    assert_ne!(response.status, CpSolverStatus::OPTIMAL);
}

#[test]
fn fills_additional_solutions_when_asked() {
    // x in [0, 2] with no constraints has three solutions.
    let mut model = CpModelProto::default();
    var(&mut model, 0, 2);
    let params = SatParameters {
        enumerate_all_solutions: Some(true),
        fill_additional_solutions_in_response: Some(true),
        solution_pool_size: Some(10),
        num_workers: Some(1),
        ..Default::default()
    };
    let response = cpsat::solve(&model, &params).unwrap();
    assert_eq!(response.status, CpSolverStatus::OPTIMAL);
    let mut values: Vec<i64> = response
        .additional_solutions
        .iter()
        .map(|solution| solution.values[0])
        .collect();
    values.sort_unstable();
    assert_eq!(values, [0, 1, 2]);
}
