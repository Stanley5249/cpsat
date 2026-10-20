//! OR-Tools diagnostics through the bridge, mirroring Python's `CpSatHelper`.

use cpsat::Error;
use cpsat::prelude::*;
use cpsat::proto::{CpSolverStatus, SatParameters};

#[test]
fn validates_a_valid_model() {
    let mut model = CpModel::new();
    model.new_var(Bool);
    model.validate().unwrap();
}

#[test]
fn reports_why_a_model_is_invalid() {
    let mut model = CpModel::new();
    model.new_var(Domain::new(5, 1));
    let Err(Error::InvalidModel(message)) = model.validate() else {
        panic!("an empty domain must be invalid");
    };
    // Python 9.15 reports the same message for this model.
    assert!(message.starts_with("var #0 has"), "{message}");
}

#[test]
fn describes_model_and_response() {
    let mut model = CpModel::new();
    let x = model.new_var(0..=10);
    let y = model.new_var(0..=10);
    model.maximize(x + y);
    let stats = model.stats().unwrap();
    assert!(stats.contains("#Variables: 2"), "{stats}");

    let response = model.solve(&SatParameters::default()).unwrap();
    let stats = response.stats().unwrap();
    assert!(stats.contains("status: OPTIMAL"), "{stats}");
}

#[test]
fn reads_parameter_defaults_from_or_tools() {
    let parameters = default_parameters().unwrap();
    // Values from the [default = X] annotations in sat_parameters.proto.
    assert_eq!(parameters.num_workers, Some(0));
    assert_eq!(parameters.cp_model_presolve, Some(true));
    assert_eq!(parameters.log_search_progress, Some(false));
    assert_eq!(parameters.max_time_in_seconds, Some(f64::INFINITY));
    // Solving with every default spelled out matches solving with none.
    let mut model = CpModel::new();
    let x = model.new_var(0..=3);
    model.maximize(x);
    let response = model.solve(&parameters).unwrap();
    let solution = response.solution().unwrap();
    assert_eq!(solution.value(x), 3);
}

/// Python accepts these models at build time and leaves the check to the
/// solver, so Rust reports them through `validate` and `MODEL_INVALID` too.
#[test]
fn rejects_what_python_rejects_only_at_solve_time() {
    let mut zero_divisor = CpModel::new();
    let x = zero_divisor.new_var(0..=10);
    zero_divisor.add(Constraint::division_equality(x, x, 0));

    // A handle from a larger model names a variable this model lacks.
    let mut larger = CpModel::new();
    let [.., far]: [IntVar; 3] = std::array::from_fn(|_| larger.new_var(0..=1));
    let mut unknown_variable = CpModel::new();
    unknown_variable.new_var(0..=1);
    unknown_variable.add(far.le(1));

    for model in [zero_divisor, unknown_variable] {
        assert!(matches!(model.validate(), Err(Error::InvalidModel(_))));
        let response = model.solve(&SatParameters::default()).unwrap();
        assert_eq!(response.status(), CpSolverStatus::MODEL_INVALID);
    }
}
