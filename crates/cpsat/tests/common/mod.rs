//! Model builders shared by the integration tests.

use buffa::MessageField;
use cpsat::proto::constraint_proto::Constraint;
use cpsat::proto::{
    ConstraintProto, CpModelProto, CpObjectiveProto, IntegerVariableProto, LinearConstraintProto,
    SatParameters,
};

pub fn var(model: &mut CpModelProto, lower: i64, upper: i64) -> i32 {
    model.variables.push(IntegerVariableProto {
        domain: vec![lower, upper],
        ..Default::default()
    });
    i32::try_from(model.variables.len() - 1).expect("tests use few variables")
}

/// Adds `lower <= sum(coeffs * vars) <= upper`.
pub fn linear(model: &mut CpModelProto, terms: &[(i32, i64)], lower: i64, upper: i64) {
    model.constraints.push(ConstraintProto {
        constraint: Some(Constraint::Linear(Box::new(LinearConstraintProto {
            vars: terms.iter().map(|&(var, _)| var).collect(),
            coeffs: terms.iter().map(|&(_, coeff)| coeff).collect(),
            domain: vec![lower, upper],
            ..Default::default()
        }))),
        ..Default::default()
    });
}

/// Maximizes `sum(coeffs * vars)` the way Python does: negate the terms and
/// set a scaling factor of -1.
pub fn maximize(model: &mut CpModelProto, terms: &[(i32, i64)]) {
    model.objective = MessageField::some(CpObjectiveProto {
        vars: terms.iter().map(|&(var, _)| var).collect(),
        coeffs: terms.iter().map(|&(_, coeff)| -coeff).collect(),
        scaling_factor: -1.0,
        ..Default::default()
    });
}

/// max 2x + 3y subject to x + y <= 4, x + 3y <= 6, x, y in [0, 10].
/// The optimum is x = 3, y = 1 with objective 9.
pub fn small_model() -> CpModelProto {
    let mut model = CpModelProto::default();
    let x = var(&mut model, 0, 10);
    let y = var(&mut model, 0, 10);
    linear(&mut model, &[(x, 1), (y, 1)], i64::MIN, 4);
    linear(&mut model, &[(x, 1), (y, 3)], i64::MIN, 6);
    maximize(&mut model, &[(x, 2), (y, 3)]);
    model
}

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
    let mut model = CpModelProto::default();
    let vars: Vec<i32> = (0..120).map(|_| var(&mut model, 0, 20)).collect();
    for _ in 0..40 {
        let terms: Vec<(i32, i64)> = vars.iter().map(|&v| (v, next(50))).collect();
        linear(&mut model, &terms, 0, 5_000);
    }
    let objective: Vec<(i32, i64)> = vars.iter().map(|&v| (v, next(100))).collect();
    maximize(&mut model, &objective);
    model
}

pub fn single_worker() -> SatParameters {
    SatParameters {
        num_workers: Some(1),
        ..Default::default()
    }
}
