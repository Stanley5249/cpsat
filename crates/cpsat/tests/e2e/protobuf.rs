//! Buffa encoding of the OR-Tools schemas.

use buffa::Message;
use cpsat::proto::{CpSolverStatus, SatParameters, sat_parameters};

#[test]
fn proto2_presence_survives_round_trip() {
    let params = SatParameters {
        num_workers: Some(8),
        cp_model_presolve: Some(false),
        ..Default::default()
    };
    let decoded = SatParameters::decode_from_slice(&params.encode_to_vec()).unwrap();
    assert_eq!(decoded.num_workers, Some(8));
    // An explicit false is present, unlike an unset field.
    assert_eq!(decoded.cp_model_presolve, Some(false));
    assert_eq!(decoded.max_time_in_seconds, None);
}

#[test]
fn unset_parameters_encode_to_nothing() {
    assert_eq!(SatParameters::default().encode_to_vec(), Vec::<u8>::new());
}

#[test]
fn proto2_enum_values_match_schema() {
    let params = SatParameters {
        search_branching: Some(sat_parameters::SearchBranching::FIXED_SEARCH),
        ..Default::default()
    };
    // Field 82 `search_branching`, varint FIXED_SEARCH = 1.
    assert_eq!(params.encode_to_vec(), [0x90, 0x05, 0x01]);
}

#[test]
fn preserves_unknown_fields() {
    // Field 100000, varint 7, from a newer schema.
    let bytes = [0x80, 0xea, 0x30, 0x07];
    let params = SatParameters::decode_from_slice(&bytes).unwrap();
    assert_eq!(params.encode_to_vec(), bytes);
}

#[test]
fn status_values_match_schema() {
    assert_eq!(CpSolverStatus::OPTIMAL as i32, 4);
    assert_eq!(CpSolverStatus::MODEL_INVALID as i32, 1);
}
