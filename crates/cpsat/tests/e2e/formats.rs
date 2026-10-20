//! Text format and JSON, read from what Python OR-Tools 9.15 writes:
//! `CpModel.export_to_file("model.txt")` and `json_format.MessageToJson`.

use cpsat::prelude::*;
use cpsat::proto::{CpModelProto, CpSolverStatus, SatParameters};

/// x + y <= 4, maximize 2x + 3y over x, y in [0, 10]. The optimum is 12.
#[cfg(feature = "text")]
const MODEL_TEXT: &str = r#"variables { name: "x" domain: [0, 10] }
variables { name: "y" domain: [0, 10] }
constraints {
  linear {
    vars: [0, 1]
    coeffs: [1, 1]
    domain: [-9223372036854775808, 4]
  }
}
objective {
  vars: [0, 1]
  scaling_factor: -1
  coeffs: [-2, -3]
}
"#;

fn expected() -> CpModelProto {
    let mut model = CpModel::new();
    let x = model.new_var((0..=10).with_name("x"));
    let y = model.new_var((0..=10).with_name("y"));
    model.add((x + y).le(4));
    model.maximize(2 * x + 3 * y);
    model.into_proto()
}

#[cfg(feature = "text")]
#[test]
fn reads_python_text_format() {
    let model: CpModelProto = buffa::text::decode_from_str(MODEL_TEXT).unwrap();
    assert_eq!(model, expected());
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    assert!((response.objective_value - 12.0).abs() < 1e-9);
}

#[cfg(feature = "text")]
#[test]
fn text_format_round_trips() {
    let text = buffa::text::encode_to_string(&expected());
    let decoded: CpModelProto = buffa::text::decode_from_str(&text).unwrap();
    assert_eq!(decoded, expected());

    // Python's text form of SatParameters(num_workers=8, log_search_progress=True).
    let parameters: SatParameters =
        buffa::text::decode_from_str("log_search_progress: true\nnum_workers: 8\n").unwrap();
    assert_eq!(parameters.num_workers, Some(8));
    assert_eq!(parameters.log_search_progress, Some(true));
}

#[cfg(feature = "json")]
#[test]
fn reads_python_json() {
    const MODEL_JSON: &str = r#"{"variables": [{"name": "x", "domain": ["0", "10"]}, {"name": "y", "domain": ["0", "10"]}], "constraints": [{"linear": {"vars": [0, 1], "coeffs": ["1", "1"], "domain": ["-9223372036854775808", "4"]}}], "objective": {"vars": [0, 1], "scalingFactor": -1.0, "coeffs": ["-2", "-3"]}}"#;
    let model: CpModelProto = serde_json::from_str(MODEL_JSON).unwrap();
    assert_eq!(model, expected());
    let json = serde_json::to_string(&model).unwrap();
    assert_eq!(serde_json::from_str::<CpModelProto>(&json).unwrap(), model);
}
