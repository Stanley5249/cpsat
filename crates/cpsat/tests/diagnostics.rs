//! Native diagnostics, mirroring Python's `CpSatHelper`.

#[expect(dead_code, reason = "each test binary uses a subset of the helpers")]
mod common;

use common::{small_model, var};
use cpsat::proto::{CpModelProto, SatParameters};

#[test]
fn validates_a_valid_model() {
    cpsat::validate(&small_model()).unwrap();
}

#[test]
fn reports_why_a_model_is_invalid() {
    let mut model = CpModelProto::default();
    var(&mut model, 5, 1);
    let error = cpsat::validate(&model).unwrap_err().to_string();
    // Python 9.15 reports the same message, typo included.
    assert_eq!(
        error,
        "invalid model: var #0 has and invalid domain() format: domain: 5 domain: 1"
    );
}

#[test]
fn describes_model_and_response() {
    let model = small_model();
    let stats = cpsat::model_stats(&model).unwrap();
    assert!(stats.contains("#Variables: 2"), "{stats}");

    let response = cpsat::solve(&model, &SatParameters::default()).unwrap();
    let stats = cpsat::response_stats(&response).unwrap();
    assert!(stats.contains("status: OPTIMAL"), "{stats}");
}
