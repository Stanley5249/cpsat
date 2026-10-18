//! Models shared by the integration tests.

use cpsat::prelude::*;
use cpsat::proto::CpModelProto;

/// A pseudo-random multi-dimensional knapsack that CP-SAT cannot prove
/// optimal within seconds, for testing stops.
pub fn hard_model() -> CpModelProto {
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move |range: i64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        i64::try_from(state >> 33).expect("33-bit shift fits in i64") % range + 1
    };
    let mut model = Model::new();
    let vars: Vec<IntVar> = (0..120).map(|_| model.int_var(0..=20)).collect();
    for _ in 0..40 {
        let load: LinearExpr = vars.iter().map(|&var| var * next(50)).sum();
        model.add(load.in_domain(0..=5_000));
    }
    model.maximize(vars.iter().map(|&var| var * next(100)).sum::<LinearExpr>());
    model.into_proto()
}

/// Parameters that would let `hard_model` run for a minute without a stop.
pub fn long_params() -> SatParameters {
    SatParameters::default()
        .with_max_time_in_seconds(60.0)
        .with_num_workers(2)
}

/// Solves `model` with default parameters and asserts that it is optimal.
pub fn solve_optimal(model: &Model) -> CpSolverResponse {
    let response = model.solve(&SatParameters::default()).unwrap();
    assert_eq!(response.status(), CpSolverStatus::OPTIMAL);
    response
}
