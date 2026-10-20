//! Model and solver-control capabilities, mirroring Python's `CpModel` and
//! `CpSolver`.

use buffa::Message;
use cpsat::prelude::*;
use cpsat::proto::{CpModelProto, CpSolverStatus, SatParameters};

/// max 2x + 3y subject to x + y <= 4, x + 3y <= 6, x, y in [0, 10].
/// The optimum is x = 3, y = 1 with objective 9.
fn small_model() -> (Model, IntVar, IntVar) {
    let mut model = Model::new();
    let x = model.new_var(0..=10);
    let y = model.new_var(0..=10);
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
    let objective = 2 * x + 3 * y;
    assert_eq!(response.value(&objective), Some(9));
    assert!((response.objective_value - 9.0).abs() < 1e-9);
    assert!((response.best_objective_bound - 9.0).abs() < 1e-9);
}

#[test]
fn reads_negated_literals_and_expressions() {
    let mut model = Model::new();
    let x = model.constant(4);
    let b = model.new_var(Bool);
    model.add(b.eq(1));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.value(-x), Some(-4));
    assert_eq!(response.value(7), Some(7));
    assert_eq!(response.value(b), Some(true));
    assert_eq!(response.value(!b), Some(false));
    // In an expression, `!b` counts as `1 - b`.
    assert_eq!(response.value(LinearExpr::from(!b)), Some(0));

    // A handle from a larger model is out of range here.
    let mut larger = Model::new();
    let far = [(); 3].map(|()| larger.new_var(0..=1))[2];
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
    let x = model.new_var(0..=3);
    model.add(x.in_domain(5..=10));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::INFEASIBLE);
    assert_eq!(response.value(x), None);
}

#[test]
fn reports_model_invalid_as_status() {
    let mut model = Model::new();
    model.new_var(Domain::new(5, 1));
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::MODEL_INVALID);
}

#[test]
fn resolves_after_model_change() {
    let (mut model, x, y) = small_model();
    assert_eq!(model.solve(&single_worker()).unwrap().solution, [3, 1]);

    // Tighten x <= 1 through its domain.
    model.proto_mut().variables[usize::try_from(x.index()).unwrap()].domain = vec![0, 1];
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
    let x = model.new_var(0..=10);
    let y = model.new_var(0..=10);
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
    model.new_var(0..=2);
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
    let x = model.new_var(0..=10);
    let y = model.new_var(0..=10);
    model.add((x + y).le(7));
    let objective = 0.5 * x - 1.25 * y;
    model.minimize(&objective);
    let response = model.solve(&SatParameters::default()).unwrap();
    // Python's OR-Tools 9.15 reports -8.75 at x = 0, y = 7.
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert!((response.objective_value + 8.75).abs() < 1e-9);
    assert_eq!(response.value(0.5 * x + y), Some(7.0));
}

/// Search counters vary between runs, so they are checked for sanity, not
/// against Python's numbers.
#[test]
fn reports_search_statistics_and_log() {
    let (model, ..) = small_model();
    let parameters = single_worker()
        .with_log_search_progress(true)
        .with_log_to_stdout(false)
        .with_log_to_response(true);
    let response = model.solve(&parameters).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert!(response.num_booleans >= 0 && response.num_conflicts >= 0);
    assert!(response.num_branches >= 0 && response.num_binary_propagations >= 0);
    assert!(response.num_integer_propagations >= 0);
    for time in [
        response.wall_time,
        response.user_time,
        response.deterministic_time,
    ] {
        assert!(time.is_finite() && time >= 0.0, "{time}");
    }
    assert!(
        response.solve_log.contains("CpSolverResponse"),
        "{}",
        response.solve_log
    );
}

/// Python's `values` returns a pandas `Series`; Rust maps the accessor over
/// any collection and keeps its order, with `None` for a missing value.
#[test]
fn evaluates_collections_in_order() {
    let mut model = Model::new();
    let vars = [3, 1, 2].map(|value| model.new_var(value..=value));
    let literals = [model.new_var(Bool), model.new_var(Bool)];
    model.add(literals[0].eq(1));
    model.add(literals[1].eq(0));
    let response = model.solve(&SatParameters::default()).unwrap();

    let mut larger = Model::new();
    let far = [(); 6].map(|()| larger.new_var(0..=1))[5];
    let values: Vec<_> = vars
        .iter()
        .chain([&far])
        .map(|&var| response.value(var))
        .collect();
    assert_eq!(values, [Some(3), Some(1), Some(2), None]);
    let booleans = literals.map(|literal| response.value(literal));
    assert_eq!(booleans, [Some(true), Some(false)]);
    let floats = vars.map(|var| response.value(0.5 * var));
    assert_eq!(floats, [Some(1.5), Some(0.5), Some(1.0)]);

    // A weighted sum zips variables with weights. `zip` stops at the shorter
    // side, so the caller checks the lengths that Python checks.
    let weights = [2, 10, 100];
    let total: LinearExpr = vars
        .into_iter()
        .zip(weights)
        .map(|(var, weight)| var * weight)
        .sum();
    assert_eq!(response.value(total), Some(216));
}
