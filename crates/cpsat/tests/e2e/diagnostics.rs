//! Native diagnostics, mirroring Python's `CpSatHelper`.

use cpsat::Error;
use cpsat::prelude::*;

#[test]
fn validates_a_valid_model() {
    let mut model = Model::new();
    model.bool_var();
    validate_model(model.proto()).unwrap();
}

#[test]
fn reports_why_a_model_is_invalid() {
    let mut model = Model::new();
    model.int_var(Domain::new(5, 1));
    let Err(Error::InvalidModel(message)) = validate_model(model.proto()) else {
        panic!("an empty domain must be invalid");
    };
    // Python 9.15 reports the same message for this model.
    assert!(message.starts_with("var #0 has"), "{message}");
}

#[test]
fn describes_model_and_response() {
    let mut model = Model::new();
    let x = model.int_var(0..=10);
    let y = model.int_var(0..=10);
    model.maximize(x + y);
    let stats = cpsat::model_stats(model.proto()).unwrap();
    assert!(stats.contains("#Variables: 2"), "{stats}");

    let response = solve(model.proto(), &SatParameters::default()).unwrap();
    let stats = cpsat::response_stats(&response).unwrap();
    assert!(stats.contains("status: OPTIMAL"), "{stats}");
}

#[test]
fn reads_parameter_defaults_from_or_tools() {
    let params = cpsat::default_parameters().unwrap();
    // Values from the [default = X] annotations in sat_parameters.proto.
    assert_eq!(params.num_workers, Some(0));
    assert_eq!(params.cp_model_presolve, Some(true));
    assert_eq!(params.log_search_progress, Some(false));
    assert_eq!(params.max_time_in_seconds, Some(f64::INFINITY));
    // Solving with every default spelled out matches solving with none.
    let mut model = Model::new();
    let x = model.int_var(0..=3);
    model.maximize(x);
    let response = solve(model.proto(), &params).unwrap();
    assert_eq!(response.value(x), Some(3));
}
