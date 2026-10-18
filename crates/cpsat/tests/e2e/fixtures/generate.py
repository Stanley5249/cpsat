"""Writes the models that `tests/e2e/equivalence.rs` rebuilds with the Rust API.

Run it from the repository root with `just fixtures`.
"""

import pathlib

from ortools.sat.python import cp_model

FIXTURES = pathlib.Path(__file__).parent


def variables(model: cp_model.CpModel):
    x = model.new_int_var(0, 10, "")
    y = model.new_int_var(0, 10, "")
    b = model.new_bool_var("")
    c = model.new_bool_var("")
    return x, y, b, c


def linear() -> cp_model.CpModel:
    model = cp_model.CpModel()
    x, y, b, c = variables(model)
    model.add(x + x + 2 * y - 3 <= 10)
    model.add(y + x <= 3)
    model.add(x == y)
    model.add(x != 3)
    model.add(x + 1 != y)
    model.add(x < y)
    model.add(x > 2)
    model.add(x - y >= -2).only_enforce_if(b, ~c)
    model.add(~b + c == 1)
    model.add(3 * (x + 2) - x <= 7)
    model.add(x - x <= 0)
    model.add(-x <= 3)
    model.add(b == 1)
    model.add(sum([x, y, b]) <= 5)
    model.add_linear_constraint(x + y, 1, 8)
    model.add_linear_expression_in_domain(x + y, cp_model.Domain.from_values([1, 3, 5]))
    return model


def boolean() -> cp_model.CpModel:
    model = cp_model.CpModel()
    _, _, b, c = variables(model)
    model.add_bool_or([b, ~c])
    model.add_bool_and([b, ~c]).only_enforce_if(~b)
    model.add_at_most_one([b, ~c])
    model.add_exactly_one([b, c])
    model.add_bool_xor([b, c])
    model.add_implication(b, c)
    return model


def global_constraints() -> cp_model.CpModel:
    model = cp_model.CpModel()
    x, y, b, c = variables(model)
    model.add_all_different([x, y + 1, 2 * b])
    model.add_element(x, [y, 3, b], 2)
    model.add_inverse([x, y], [y, x])
    model.add_allowed_assignments([x, y + 1], [(1, 2), (3, 4)])
    model.add_forbidden_assignments([x, y], [(1, 2)])
    model.add_automaton([x, y], 0, [1, 2], [(0, 1, 1), (1, 2, 2)])
    model.add_circuit([(0, 1, b), (1, 0, ~c)])
    model.add_multiple_circuit([(0, 1, b), (1, 0, c)])
    return model


def arithmetic() -> cp_model.CpModel:
    model = cp_model.CpModel()
    x, y, b, _ = variables(model)
    model.add_max_equality(x + 1, [2 * y, 3])
    model.add_min_equality(x, [y, 3])
    model.add_abs_equality(x, y - 4)
    model.add_division_equality(x, y, 3)
    model.add_modulo_equality(x, y, 3)
    model.add_multiplication_equality(x, [y, b])
    return model


def scheduling() -> cp_model.CpModel:
    model = cp_model.CpModel()
    x, y, b, c = variables(model)
    i = model.new_interval_var(x, 2, y, "")
    j = model.new_fixed_size_interval_var(x + 1, 3, "")
    k = model.new_optional_interval_var(x, y, x + 5, b, "")
    l = model.new_optional_fixed_size_interval_var(y, 4, ~c, "")
    model.add_no_overlap([i, j])
    model.add_no_overlap_2d([i, j], [k, l])
    model.add_cumulative([i, j], [2, x], 5)
    model.add_cumulative([i], [1], y + 1)
    model.add_reservoir_constraint([x, y + 1], [2, -2], 0, 5)
    model.add_reservoir_constraint_with_active([x, y], [1, -1], [b, ~c], -1, 3)
    return model


def search() -> cp_model.CpModel:
    model = cp_model.CpModel()
    x, y, b, c = variables(model)
    model.maximize(x + 2 * y + 1)
    model.add_hint(x, 3)
    model.add_hint(~b, True)
    model.add_assumption(~c)
    model.add_assumptions([b])
    model.add_decision_strategy(
        [x, ~b], cp_model.CHOOSE_FIRST, cp_model.SELECT_MIN_VALUE
    )
    return model


def minimize_constant() -> cp_model.CpModel:
    model = cp_model.CpModel()
    variables(model)
    model.minimize(5)
    return model


def minimize_float() -> cp_model.CpModel:
    model = cp_model.CpModel()
    x, y, b, _ = variables(model)
    model.minimize(0.5 * x - 1.25 * y + 0.1 + x + 0.5 * ~b)
    return model


def maximize_float() -> cp_model.CpModel:
    model = cp_model.CpModel()
    x, y, _, _ = variables(model)
    model.maximize(2.5 * x + y - 0.5)
    return model


def constants() -> cp_model.CpModel:
    model = cp_model.CpModel()
    x, _, b, _ = variables(model)
    model.add(x == model.new_constant(5))
    model.add(x <= model.new_constant(5) + model.new_constant(1))
    model.add_bool_or([True, b])
    model.add_bool_and([False]).only_enforce_if(b)
    return model


def main() -> None:
    for build in [
        linear,
        boolean,
        global_constraints,
        arithmetic,
        scheduling,
        search,
        minimize_constant,
        minimize_float,
        maximize_float,
        constants,
    ]:
        path = FIXTURES / f"{build.__name__}.txtpb"
        path.write_text(str(build().proto), encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
