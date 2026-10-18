//! Native diagnostics, mirroring Python's `CpSatHelper`.

use cpsat::Error;
use cpsat::proto::{CpModelProto, SatParameters};

#[test]
fn validates_a_valid_model() {
    let mut model = CpModelProto::default();
    model.add_int_var(0, 1);
    cpsat::validate_model(&model).unwrap();
}

#[test]
fn reports_why_a_model_is_invalid() {
    let mut model = CpModelProto::default();
    model.add_int_var(5, 1);
    let Err(Error::InvalidModel(message)) = cpsat::validate_model(&model) else {
        panic!("an empty domain must be invalid");
    };
    // Python 9.15 reports the same message for this model.
    assert!(message.starts_with("var #0 has"), "{message}");
}

#[test]
fn describes_model_and_response() {
    let mut model = CpModelProto::default();
    let x = model.add_int_var(0, 10);
    let y = model.add_int_var(0, 10);
    model.maximize(&[(x, 1), (y, 1)]);
    let stats = cpsat::model_stats(&model).unwrap();
    assert!(stats.contains("#Variables: 2"), "{stats}");

    let response = cpsat::solve(&model, &SatParameters::default()).unwrap();
    let stats = cpsat::response_stats(&response).unwrap();
    assert!(stats.contains("status: OPTIMAL"), "{stats}");
}
