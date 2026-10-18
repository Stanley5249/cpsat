//! Model and solver-control capabilities, mirroring Python's `CpModel` and
//! `CpSolver`.

use buffa::Message;
use cpsat::prelude::*;
use cpsat::proto::CpModelProto;

/// max 2x + 3y subject to x + y <= 4, x + 3y <= 6, x, y in [0, 10].
/// The optimum is x = 3, y = 1 with objective 9.
fn small_model() -> (Model, IntVar, IntVar) {
    let mut model = Model::new();
    let x = model.int_var(0..=10);
    let y = model.int_var(0..=10);
    model.add((x + y).le(4));
    model.add((x + 3 * y).le(6));
    model.maximize(2 * x + 3 * y);
    (model, x, y)
}

fn single_worker() -> SatParameters {
    SatParameters::default().with_num_workers(1)
}

#[test]
fn solves_to_optimality() {
    let (model, x, y) = small_model();
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert_eq!((response.value(x), response.value(y)), (Some(3), Some(1)));
    assert_eq!(response.value(2 * x + 3 * y - 1), Some(8));
    assert!((response.objective_value - 9.0).abs() < 1e-9);
    assert!((response.best_objective_bound - 9.0).abs() < 1e-9);
}

#[test]
fn reads_negated_literals_and_expressions() {
    let mut model = Model::new();
    let x = model.constant(4);
    let b = model.bool_var();
    model.add(b.eq(1));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.value(-x), Some(-4));
    assert_eq!(response.value(7), Some(7));
    assert_eq!(response.boolean_value(b), Some(true));
    assert_eq!(response.boolean_value(!b), Some(false));
    assert_eq!(response.value(!b), Some(0));

    // A handle from a larger model is out of range here.
    let mut larger = Model::new();
    let far = [(); 3].map(|()| larger.int_var(0..=1))[2];
    assert_eq!(response.value(far), None);
}

#[test]
fn displays_status_like_python() {
    assert_eq!(CpSolverStatus::OPTIMAL.to_string(), "OPTIMAL");
    assert!(CpSolverStatus::FEASIBLE.has_solution());
    assert!(!CpSolverStatus::INFEASIBLE.has_solution());
}

#[test]
fn reports_infeasible() {
    let mut model = Model::new();
    let x = model.int_var(0..=3);
    model.add(x.in_domain(5..=10));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::INFEASIBLE);
    assert_eq!(response.value(x), None);
}

#[test]
fn reports_model_invalid_as_status() {
    let mut model = Model::new();
    model.int_var(Domain::new(5, 1));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::MODEL_INVALID);
}

#[test]
fn resolves_after_model_change() {
    let (mut model, x, y) = small_model();
    assert_eq!(model.solve(&single_worker()).unwrap().solution, [3, 1]);

    // Tighten x <= 1 through its domain.
    model[x].domain = vec![0, 1];
    assert_eq!(model.solve(&single_worker()).unwrap().solution, [0, 2]);

    // Replace the objective, then remove the second constraint, so y is
    // bounded only by x + y <= 4.
    model.maximize(y);
    model.proto_mut().constraints.truncate(1);
    let response = model.solve(&single_worker()).unwrap();
    assert_eq!(response.value(y), Some(4));
}

#[test]
fn clone_and_serialize_round_trip() {
    let (model, ..) = small_model();
    let decoded = CpModelProto::decode_from_slice(&model.proto().encode_to_vec()).unwrap();
    assert_eq!(model.clone(), Model::from(decoded.clone()));
    let response = decoded.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
}

#[test]
fn first_solution_follows_the_hint() {
    // x + y == 10 has eleven solutions; without the hint the first one found
    // is not (7, 3).
    let mut model = Model::new();
    let x = model.int_var(0..=10);
    let y = model.int_var(0..=10);
    model.add((x + y).eq(10));
    // Presolve solves this model outright and skips the hint, in Python too,
    // so it is off here.
    let parameters = single_worker()
        .with_stop_after_first_solution(true)
        .with_cp_model_presolve(false);
    assert_ne!(model.solve(&parameters).unwrap().solution, [7, 3]);

    model.add_hint(x, 7);
    model.add_hint(y, 3);
    assert_eq!(model.solve(&parameters).unwrap().solution, [7, 3]);

    model.clear_hints();
    assert!(model.proto().solution_hint.is_unset());
}

#[test]
fn honors_time_limit() {
    let parameters = SatParameters::default().with_max_time_in_seconds(0.5);
    let response = crate::common::hard_model().solve(&parameters).unwrap();
    assert_ne!(response.status(), CpSolverStatus::OPTIMAL);
    assert!(
        response.wall_time < 30.0,
        "wall time {}",
        response.wall_time
    );
}

#[test]
fn fills_additional_solutions_when_asked() {
    let mut model = Model::new();
    model.int_var(0..=2);
    let parameters = single_worker()
        .with_enumerate_all_solutions(true)
        .with_fill_additional_solutions_in_response(true)
        .with_solution_pool_size(10);
    let response = model.solve(&parameters).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    let mut values: Vec<i64> = response
        .additional_solutions
        .iter()
        .map(|solution| solution.values[0])
        .collect();
    values.sort_unstable();
    assert_eq!(values, [0, 1, 2]);
}

#[test]
fn solves_a_float_objective_like_python() {
    let mut model = Model::new();
    let x = model.int_var(0..=10);
    let y = model.int_var(0..=10);
    model.add((x + y).le(7));
    model.minimize_float(0.5 * x - 1.25 * y);
    let response = model.solve(&SatParameters::default()).unwrap();
    // Python's OR-Tools 9.15 reports -8.75 at x = 0, y = 7.
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert!((response.objective_value + 8.75).abs() < 1e-9);
    assert_eq!(response.float_value(0.5 * x + y), Some(7.0));
}
