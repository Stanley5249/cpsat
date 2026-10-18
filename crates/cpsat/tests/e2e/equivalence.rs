//! The Rust modeling API writes the same `CpModelProto` as Python OR-Tools
//! 9.15. Each test rebuilds a model from `fixtures/generate.py`, whose output
//! `just fixtures` writes to `fixtures/*.txtpb`.

#![cfg(feature = "text")]

use cpsat::prelude::*;
use cpsat::proto::CpModelProto;
use cpsat::proto::decision_strategy_proto::{DomainReductionStrategy, VariableSelectionStrategy};

fn python(name: &str) -> CpModelProto {
    let path = format!(
        "{}/tests/e2e/fixtures/{name}.txtpb",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    buffa::text::decode_from_str(&text).unwrap()
}

/// Compares one constraint at a time, so a failure names the first one that
/// differs.
fn assert_matches_python(model: &Model, name: &str) {
    let expected = python(name);
    let actual = model.proto();
    for (index, (actual, expected)) in actual
        .constraints
        .iter()
        .zip(&expected.constraints)
        .enumerate()
    {
        assert_eq!(actual, expected, "constraint #{index} of {name}");
    }
    assert_eq!(actual, &expected, "{name}");
}

fn variables(model: &mut Model) -> (IntVar, IntVar, BoolVar, BoolVar) {
    let x = model.int_var(0..=10);
    let y = model.int_var(0..=10);
    (x, y, model.bool_var(), model.bool_var())
}

#[test]
fn linear() {
    let mut model = Model::new();
    let (x, y, b, c) = variables(&mut model);
    model.add((x + x + 2 * y - 3).le(10));
    model.add((y + x).le(3));
    model.add(x.eq(y));
    model.add(x.ne(3));
    model.add((x + 1).ne(y));
    model.add(x.lt(y));
    model.add(x.gt(2));
    model.add((x - y).ge(-2).only_if(b).only_if(!c));
    model.add((!b + c).eq(1));
    model.add((3 * (x + 2) - x).le(7));
    model.add((x - x).le(0));
    model.add((-x).le(3));
    model.add(b.eq(1));
    model.add(
        [x.into(), y.into(), LinearExpr::from(b)]
            .into_iter()
            .sum::<LinearExpr>()
            .le(5),
    );
    model.add((x + y).in_domain(1..=8));
    model.add((x + y).in_domain(Domain::from_values([1, 3, 5])));
    assert_matches_python(&model, "linear");
}

#[test]
fn boolean() {
    let mut model = Model::new();
    let (_, _, b, c) = variables(&mut model);
    model.add(Constraint::bool_or([b, !c]));
    model.add(Constraint::bool_and([b, !c]).only_if(!b));
    model.add(Constraint::at_most_one([b, !c]));
    model.add(Constraint::exactly_one([b, c]));
    model.add(Constraint::bool_xor([b, c]));
    model.add(Constraint::implication(b, c));
    assert_matches_python(&model, "boolean");
}

#[test]
fn global_constraints() {
    let mut model = Model::new();
    let (x, y, b, c) = variables(&mut model);
    model.add(Constraint::all_different([x.into(), y + 1, 2 * b]));
    model.add(Constraint::element(
        x,
        [y.into(), LinearExpr::constant(3), b.into()],
        2,
    ));
    model.add(Constraint::inverse([x, y], [y, x]));
    model.add(Constraint::allowed_assignments(
        [x.into(), y + 1],
        [[1, 2], [3, 4]],
    ));
    model.add(Constraint::forbidden_assignments([x, y], [[1, 2]]));
    model.add(Constraint::automaton(
        [x, y],
        0,
        [1, 2],
        [(0, 1, 1), (1, 2, 2)],
    ));
    model.add(Constraint::circuit([(0, 1, b), (1, 0, !c)]));
    model.add(Constraint::multiple_circuit([(0, 1, b), (1, 0, c)]));
    assert_matches_python(&model, "global_constraints");
}

#[test]
fn arithmetic() {
    let mut model = Model::new();
    let (x, y, b, _) = variables(&mut model);
    model.add(Constraint::max_equality(
        x + 1,
        [2 * y, LinearExpr::constant(3)],
    ));
    model.add(Constraint::min_equality(
        x,
        [y.into(), LinearExpr::constant(3)],
    ));
    model.add(Constraint::abs_equality(x, y - 4));
    model.add(Constraint::division_equality(x, y, 3));
    model.add(Constraint::modulo_equality(x, y, 3));
    model.add(Constraint::multiplication_equality(
        x,
        [LinearExpr::from(y), b.into()],
    ));
    assert_matches_python(&model, "arithmetic");
}

#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "mirrors the names in fixtures/generate.py"
)]
fn scheduling() {
    let mut model = Model::new();
    let (x, y, b, c) = variables(&mut model);
    let i = model.interval_var(x, 2, y);
    let j = model.fixed_size_interval_var(x + 1, 3);
    let k = model.optional_interval_var(x, y, x + 5, b);
    let l = model.optional_fixed_size_interval_var(y, 4, !c);
    model.add(Constraint::no_overlap([i, j]));
    model.add(Constraint::no_overlap_2d([(i, k), (j, l)]));
    model.add(Constraint::cumulative(
        5,
        [(i, LinearExpr::constant(2)), (j, x.into())],
    ));
    model.add(Constraint::cumulative(y + 1, [(i, 1)]));
    model.add(Constraint::reservoir(0, 5, [(x.into(), 2), (y + 1, -2)]));
    model.add(Constraint::reservoir_with_active(
        -1,
        3,
        [(x, 1, b), (y, -1, !c)],
    ));
    assert_matches_python(&model, "scheduling");
}

#[test]
fn search() {
    let mut model = Model::new();
    let (x, y, b, c) = variables(&mut model);
    model.maximize(x + 2 * y + 1);
    model.add_hint(x, 3);
    model.add_bool_hint(!b, true);
    model.add_assumption(!c);
    model.add_assumptions([b]);
    model.add_decision_strategy(
        [x.into(), LinearExpr::from(!b)],
        VariableSelectionStrategy::CHOOSE_FIRST,
        DomainReductionStrategy::SELECT_MIN_VALUE,
    );
    assert_matches_python(&model, "search");
}

#[test]
fn minimize_constant() {
    let mut model = Model::new();
    variables(&mut model);
    model.minimize(5);
    assert_matches_python(&model, "minimize_constant");
}
