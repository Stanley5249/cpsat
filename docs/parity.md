# Python CP-SAT parity

The reference is the public CP-SAT API in OR-Tools v9.15
[`cp_model.py`](https://github.com/google/or-tools/blob/v9.15/ortools/sat/python/cp_model.py),
its imported modeling types in
[`cp_model_helper.cc`](https://github.com/google/or-tools/blob/v9.15/ortools/sat/python/cp_model_helper.cc),
and `Domain` from
[`sorted_interval_list.cc`](https://github.com/google/or-tools/blob/v9.15/ortools/util/python/sorted_interval_list.cc).
Entries distinguish direct APIs, equivalents built with Rust iterators or
Protobuf fields, and Python-specific conveniences. Deprecated aliases share
their canonical method's entry rather than requiring a second Rust API. Private
underscore-prefixed methods, decorator/name-conversion utilities, Python typing
aliases, and extension-only implementation classes are outside this surface; the
imported `FlatIntExpr`, `FlatFloatExpr`, and `BoundedLinearExpression` are
included explicitly. Generated Protobuf fields are covered as schema types,
rather than listing every generated accessor.

Test names are relative to `crates/cpsat/tests/e2e`. A test citation identifies
only the behavior that test asserts; an available equivalent with no direct test
is marked as such. `equivalence.rs` compares committed Python-generated model
fixtures, while solve tests check results. Neither implies exhaustive argument
or error-path parity. `just gen-fixtures` pins the Python `ortools` package, and
`samples.rs` ports selected official samples. Rust naming follows the Methods
and Names rules in `docs/design.md`.

## Solver

- `CpSolver.solve(model)`
  - Rust: `CpModel::solve`, `CpModelProto::solve`, `CpSolver::solve`
  - OR-Tools: `SolveWrapper::Solve`
  - Test: `samples.rs::cp_sat_example`, `events.rs::solver_solves_again`
- `CpSolver.parameters`
  - Rust: `SatParameters` passed to `CpSolver::new`, `CpSolver::parameters`
  - OR-Tools: `SolveWrapper::SetParameters`
  - Test: `events.rs::forwards_logs_and_parameters`
- `max_time_in_seconds`
  - Rust: `SatParameters` field
  - OR-Tools: none
  - Test: `solve.rs::honors_time_limit`
- `CpSolverSolutionCallback.on_solution_callback`
  - Rust: `Event::Solution`
  - OR-Tools: `SolutionCallback`
  - Test: `samples.rs::solve_and_print_intermediate_solutions`
- `CpSolverSolutionCallback.value`, `boolean_value`, `float_value`
  - Rust: `CpSolverResponseView::value`, which reads a literal as `bool` and a
    `FloatExpr` as `f64`
  - OR-Tools: none
  - Test: `events.rs::solution_view_reads_values_and_converts_to_owned`,
    `solution_view_accessors_match_the_owned_response`
- `CpSolver.best_bound_callback`
  - Rust: `Event::BestBound`
  - OR-Tools: `AddBestBoundCallback`
  - Test: `events.rs::reports_best_bounds`
- `CpSolver.log_callback`
  - Rust: `Event::Log`
  - OR-Tools: `AddLogCallback`
  - Test: `events.rs::forwards_logs_and_parameters`
- `CpSolverSolutionCallback.stop_search`
  - Rust: return `ControlFlow::Break`
  - OR-Tools: `StopSearch`
  - Test: `samples.rs::stop_after_n_solutions`
- `CpSolver.stop_search` from another thread
  - Rust: `StopHandle::stop` with `CpSolver::with_stop_handle`
  - OR-Tools: `StopSearch`
  - Test: `events.rs::stops_from_another_thread`
- Exception in a callback stops the search and is reraised
  - Rust: panic stops the search and resumes after the solve
  - OR-Tools: none
  - Test: `events.rs::callback_panic_stops_and_resumes_after_solve`
- `enumerate_all_solutions`
  - Rust: `SatParameters` field with `EventSet::SOLUTION`
  - OR-Tools: none
  - Test: `samples.rs::search_for_all_solutions`
- `fill_additional_solutions_in_response`
  - Rust: `CpSolverResponse::additional_solutions`
  - OR-Tools: none
  - Test: `solve.rs::fills_additional_solutions_when_asked`
- `CpSolver.value`, `boolean_value`, `float_value`
  - Rust: `CpSolverResponse::value`, which reads a literal as `bool` and a
    `FloatExpr` as `f64`
  - OR-Tools: none
  - Test: `solve.rs::reads_negated_literals_and_expressions`,
    `solve.rs::solves_a_float_objective_like_python`
- `CpSolver.status_name`
  - Rust: `Display for CpSolverStatus`
  - OR-Tools: none
  - Test: `solve.rs::displays_status_like_python`
- `CpSolver.response_stats`
  - Rust: `CpSolverResponse::stats`
  - OR-Tools: `CpSatHelper::SolverResponseStats`
  - Test: `diagnostics.rs::describes_model_and_response`
- `CpSolver.sufficient_assumptions_for_infeasibility`
  - Rust: `CpSolverResponse` field
  - OR-Tools: none
  - Test: `samples.rs::assumptions`
- `CpSolver.objective_value`, `best_objective_bound`
  - Rust: Same-named `CpSolverResponse` fields
  - OR-Tools: none
  - Test: `solve.rs::solves_to_optimality`
- `CpSolver.response_proto`
  - Rust: The returned `CpSolverResponse`; Rust keeps each response
    independently of `CpSolver`
  - OR-Tools: none
  - Test: Used throughout the solve tests; independent response ownership has no
    direct test
- `CpSolver.num_booleans`, `num_conflicts`, `num_branches`,
  `num_binary_propagations`, `num_integer_propagations`
  - Rust: Same-named response fields
  - OR-Tools: none
  - Test: `solve.rs::reports_search_statistics_and_log` checks the fields are
    readable and in range, not equal to Python's run
- `CpSolver.deterministic_time`, `wall_time`, `user_time`
  - Rust: Same-named response fields
  - OR-Tools: none
  - Test: `solve.rs::honors_time_limit`, `reports_search_statistics_and_log`
    check ranges, not equality with Python's run
- `CpSolver.solve_log`, `solution_info`, `solve_info`
  - Rust: Response `solve_log` with `log_to_response`, and the pinned schema's
    `solution_info`; Rust exposes no separate `solve_info` property
  - OR-Tools: none
  - Test: `solve.rs::reports_search_statistics_and_log` covers `solve_log`;
    `solution_info` has no direct test
- `CpSolver.values`, `boolean_values`, `float_values`
  - Rust: Iterate the corresponding response accessor and collect `Option`
    values; Python returns a pandas `Series` preserving the input index
  - OR-Tools: none
  - Test: `solve.rs::evaluates_collections_in_order`
- `CpSolverSolutionCallback.has_response`, `response_proto`
  - Rust: `Event::Solution` always carries a response view;
    `MessageView::to_owned_message` retains it after the callback
  - OR-Tools: none
  - Test: `events.rs::solution_view_reads_values_and_converts_to_owned`
- Callback `objective_value`, `best_objective_bound`, `num_booleans`,
  `num_conflicts`, `num_branches`, `num_integer_propagations`,
  `num_binary_propagations`, `deterministic_time`, `wall_time`, `user_time`
  - Rust: Same-named solution-view fields
  - OR-Tools: none
  - Test: `samples.rs::solve_and_print_intermediate_solutions` reads objective
    value; the other fields have no direct parity assertions,
    `events.rs::solution_view_carries_search_statistics`
- `SearchForAllSolutions`
  - Rust: Set `enumerate_all_solutions` and subscribe to solution events;
    Python's deprecated wrapper also rejects objectives and restores the
    previous parameter
  - OR-Tools: none
  - Test: `samples.rs::search_for_all_solutions` covers enumeration, not wrapper
    validation or restoration
- `ObjectiveSolutionPrinter`, `VarArrayAndObjectiveSolutionPrinter`,
  `VarArraySolutionPrinter`, `solution_count`
  - Rust: Print or count in a solution-event closure; Python's printer classes
    and output format are presentation conveniences
  - OR-Tools: none
  - Test: `samples.rs::stop_after_n_solutions` counts events; exact Python
    printer output has no direct test

## Model

`equivalence.rs` checks that each family writes the same `CpModelProto` as
Python, from the fixtures that `just gen-fixtures` regenerates, and
`constraints.rs` solves each global and scheduling family.

- `CpModel()`, `CpModel(model_proto)`
  - Rust: `CpModel::new`, `CpModel::from(proto)`
  - Test: `solve.rs::clone_and_serialize_round_trip`
- `new_int_var`, `new_int_var_from_domain`
  - Rust: `CpModel::new_var` with a bounded range or a `Domain`
  - Test: `samples.rs`, `domain.rs`
- `new_bool_var`
  - Rust: `CpModel::new_var(Bool)`
  - Test: `samples.rs::boolean_product`
- `new_constant`, `True`, `False`
  - Rust: `CpModel::new_var` with a fixed range such as `5..=5`, or with `true`
    or `false` for a fixed literal; each call appends a variable, while Python
    and C++ share one variable per value
  - Test: `model.rs::constants_and_true_literals_solve_to_their_values`
- `new_interval_var`, `new_fixed_size_interval_var`
  - Rust: `CpModel::new_var` with `Interval::new` or `Interval::fixed_size`
  - Test: `samples.rs::no_overlap`, `equivalence.rs::scheduling`
- `new_optional_interval_var`, `new_optional_fixed_size_interval_var`
  - Rust: `Interval::only_enforce_if` on either
  - Test: `samples.rs::ranking_circuit`, `equivalence.rs::scheduling`
- `IntervalVar.start_expr`, `size_expr`, `end_expr`, `presence_literals`
  - Rust: `CpModel::interval_start`, `interval_size`, `interval_end`,
    `interval_presence_literals`
  - Test: `model.rs::reads_interval_parts_like_python`
- `new_int_var_series`, `new_bool_var_series`, `new_interval_var_series`,
  `new_fixed_size_interval_var_series`, `new_optional_interval_var_series`,
  `new_optional_fixed_size_interval_var_series`
  - Rust: Iterators and `collect`; callers choose collection keys and validate
    alignment, instead of pandas `Series` indexing and name checks
  - Test: `samples.rs::nqueens` demonstrates a variable collection; pandas
    validation has no direct equivalent
- `LinearExpr` arithmetic, `sum`, `weighted_sum`
  - Rust: `+`, `-`, `*`, unary `-`, assignment operators, and `Sum` on
    `LinearExpr`; weighted sums use `zip`, multiplication, then `sum`, with
    caller-checked lengths
  - Test: `equivalence.rs::linear`,
    `model.rs::expressions_merge_terms_like_python`,
    `solve.rs::evaluates_collections_in_order`; `zip` truncates mismatched
    lengths, so the caller checks them
- Floating-point `LinearExpr` arithmetic
  - Rust: `FloatExpr`, mixed variable/scalar arithmetic, assignment operators,
    and `Sum`
  - Test: `model.rs::float_operators_mix_constants_and_expressions`,
    `solve.rs::solves_a_float_objective_like_python`; float `Sum` has no direct
    test
- `LinearExpr.constant`, `term`, `affine`
  - Rust: `LinearExpr::constant`, `FloatExpr::constant`, `coeff * expr`,
    `coeff * expr + offset`
  - Test: `model.rs::expressions_merge_terms_like_python`,
    `float_operators_mix_constants_and_expressions`
- `LinearExpr.is_integer`
  - Rust: The static distinction between `LinearExpr` and `FloatExpr`; a
    `FloatExpr` remains floating-point even when its coefficients are whole
    numbers
  - Test: Compile-time type distinction; no direct runtime parity test
- `FlatIntExpr`, `FlatFloatExpr` and their `vars`, `coeffs`, `offset`,
  `FlatIntExpr.ok`
  - Rust: Convert `LinearExpr` into `LinearExpressionProto`; set a float
    objective and inspect `floating_point_objective` for normalized float terms.
    Rust's expression types replace the runtime integer-validity flag
  - Test: `model.rs::expressions_merge_terms_like_python`,
    `equivalence.rs::minimize_float`; standalone float-expression inspection has
    no direct Rust API
- `BoundedLinearExpression` constructors, `bounds`, `vars`, `coeffs`, `offset`,
  `ok`
  - Rust: Comparisons or `(lhs - rhs).in_domain(domain)` build a `Constraint`;
    inspect `Constraint::proto` or `into_proto` for its linear fields, with the
    offset shifted into the domain
  - Test: `equivalence.rs::linear`,
    `model.rs::comparisons_at_the_i64_limits_match_python`
- `~b`, `Not()`
  - Rust: `!b`
  - Test: `model.rs::literals_negate_and_name_their_variable`
- Python truth conversion of literals/expressions and unsupported arithmetic
  (`/`, `%`, powers, shifts, bitwise operators, `abs`)
  - Rust: Rust keeps handles/expressions distinct from `bool`; constraints use
    named comparisons, and arithmetic equalities use `Constraint` constructors.
    Handle `==` compares identity, while `.eq(...)` builds a constraint
  - Test: `model.rs::literals_negate_and_name_their_variable` covers literal
    identity; compile-error behavior has no dedicated test
- Variable, literal, interval, and constraint `index`
  - Rust: `IntVar::index`, `BoolVar::index`, `IntervalVar::index`,
    `ConstraintIndex::index`
  - Test: `model.rs::literals_negate_and_name_their_variable`,
    `looks_up_handles_by_index`
- `CpModel.negated(index)`
  - Rust: `!index` for an `i32` schema index, or `(!literal).index()` on a typed
    literal
  - Test: `model.rs::literals_negate_and_name_their_variable` tests typed
    negation
- `IntVar.is_boolean`, literal `negated`, `NotBooleanVariable`
  - Rust: `CpModel::to_bool_var(var).is_some()`, `!literal`,
    `BoolVar::is_negated`, `variable`; one Rust literal type represents both
    signs
  - Test: `model.rs::a_zero_one_variable_serves_as_a_literal`,
    `only_domains_within_zero_and_one_are_literals`,
    `literals_negate_and_name_their_variable`
- `IntVar.domain`, `with_domain`, `with_name`, constructor names
  - Rust: Name with `with_name` before `new_var`, read `model[var].domain` and
    `.name`, and change both through `CpModel::var_mut`; convert domains with
    `Domain::from_flat_intervals` and `flattened_intervals`
  - Test: `solve.rs::resolves_after_model_change`,
    `model.rs::literals_negate_and_name_their_variable`
- A 0-1 `IntVar` where a literal goes
  - Rust: `CpModel::to_bool_var`
  - Test: `model.rs::a_zero_one_variable_serves_as_a_literal`
- `add(x == y)` and the other comparisons
  - Rust: `eq`, `ne`, `le`, `lt`, `ge`, `gt`
  - Test: `equivalence.rs::linear`
- `add_linear_constraint`, `add_linear_expression_in_domain`
  - Rust: `in_domain` with a range or a `Domain`
  - Test: `equivalence.rs::linear`, `samples.rs::step_function`
- `add(True)`, `add(False)`
  - Rust: `Constraint::bool_and([])`, `Constraint::bool_or([])` are semantic
    equivalents; Python's true-constraint encoding uses a constant literal
  - Test: `constraints.rs::empty_bool_and_holds_and_empty_bool_or_fails`,
    `enforced_empty_bool_or_forces_its_literal_false`
- `only_enforce_if`, `with_name`
  - Rust: `Constraint::only_enforce_if`, `with_name`
  - Test: `model.rs::constraint_builder_sets_enforcement_and_name`
- `add_bool_or`, `add_at_least_one`, `add_bool_and`
  - Rust: `Constraint::bool_or`, `bool_and`
  - Test: `equivalence.rs::boolean`
- `add_at_most_one`, `add_exactly_one`, `add_bool_xor`, `add_implication`
  - Rust: `Constraint::at_most_one`, `exactly_one`, `bool_xor`, `implication`
  - Test: `equivalence.rs::boolean`,
    `constraints.rs::bool_xor_needs_an_odd_count`
- `add_all_different`, `add_element`, `add_inverse`
  - Rust: `Constraint::all_different`, `element`, `inverse`
  - Test: `equivalence.rs::global_constraints`, `samples.rs::nqueens`,
    `constraints.rs`
- `add_allowed_assignments`, `add_forbidden_assignments`
  - Rust: `Constraint::allowed_assignments`, `forbidden_assignments`
  - Test: `equivalence.rs::global_constraints`, `constraints.rs`,
    `constraints.rs::table_rejects_a_ragged_row`
- `add_automaton`
  - Rust: `Constraint::automaton`
  - Test: `equivalence.rs::global_constraints`, `constraints.rs`
- `add_circuit`, `add_multiple_circuit`
  - Rust: `Constraint::circuit`, `multiple_circuit`
  - Test: `samples.rs::ranking_circuit`, `constraints.rs`
- `add_min_equality`, `add_max_equality`, `add_abs_equality`
  - Rust: `Constraint::min_equality`, `max_equality`, `abs_equality`
  - Test: `equivalence.rs::arithmetic`, `samples.rs::no_overlap`,
    `constraints.rs::min_equality_takes_the_smallest`,
    `abs_equality_drops_the_sign`
- `add_division_equality`, `add_modulo_equality`, `add_multiplication_equality`
  - Rust: `Constraint::division_equality`, `modulo_equality`,
    `multiplication_equality`
  - Test: `equivalence.rs::arithmetic`, `samples.rs::non_linear`,
    `constraints.rs::modulo_equality_keeps_the_sign_of_the_dividend`
- `add_no_overlap`, `add_no_overlap_2d`
  - Rust: `Constraint::no_overlap`, `no_overlap_2d` with `(x, y)` pairs
  - Test: `samples.rs::minimal_jobshop`, `equivalence.rs::scheduling`,
    `constraints.rs`
- `add_cumulative`
  - Rust: `Constraint::cumulative`
  - Test: `equivalence.rs::scheduling`, `constraints.rs`
- `add_reservoir_constraint`, `add_reservoir_constraint_with_active`
  - Rust: `Constraint::reservoir`, `reservoir_with_active`
  - Test: `equivalence.rs::scheduling`, `constraints.rs`
- `add_map_domain`
  - Rust: For each literal at index `i`, add
    `var.eq(offset + i).only_enforce_if(literal)` and
    `var.ne(offset + i).only_enforce_if(!literal)`; a manual encoding rather
    than a direct method
  - Test: `equivalence.rs::map_domain`,
    `constraints.rs::map_domain_sets_only_the_literal_of_the_value`
- `minimize`, `maximize`, `has_objective`, `clear_objective`
  - Rust: `CpModel::minimize`, `maximize`, `has_objective`, `clear_objective`
  - Test: `model.rs::replaces_and_clears_the_objective`
- `minimize`, `maximize` with float coefficients
  - Rust: `CpModel::minimize`, `maximize` with a `FloatExpr` or `f64`; the type
    picks the float objective where Python checks the coefficients
  - Test: `equivalence.rs::minimize_float`, `equivalence.rs::maximize_float`,
    `model.rs::integer_and_float_objectives_replace_each_other`
- `add_decision_strategy`
  - Rust: `CpModel::add_decision_strategy`
  - Test: `samples.rs::channeling`
- `add_hint`, `clear_hints`
  - Rust: `CpModel::add_hint` with an `i64` or a `bool`, `clear_hints`
  - Test: `samples.rs::solution_hinting`,
    `solve.rs::first_solution_follows_the_hint`, `equivalence.rs::search`
- `add_assumption`, `add_assumptions`, `clear_assumptions`
  - Rust: `CpModel::add_assumptions`, `clear_assumptions`; one literal is `[b]`
  - Test: `samples.rs::assumptions`, `model.rs::adds_and_clears_assumptions`
- `get_int_var_from_proto_index`, `get_bool_var_from_proto_index`,
  `get_interval_var_from_proto_index`
  - Rust: `CpModel::int_var_from_index`, `interval_var_from_index`;
    `to_bool_var` on the variable for a literal
  - Test: `model.rs::looks_up_handles_by_index`
- `IntVar.name`, `IntervalVar.name`, `Constraint.name`, `CpModel.name`
  - Rust: `with_name` before insertion, `model[handle].name` to read,
    `CpModel::var_mut` to rename a variable, and `model.proto_mut()` for the
    rest
  - Test: `model.rs::literals_negate_and_name_their_variable`,
    `constraint_builder_sets_enforcement_and_name`
- `remove_all_names`
  - Rust: `CpModel::remove_all_names`
  - Test: `model.rs::constraint_builder_sets_enforcement_and_name`
- Change domains, objectives, constraints, then solve again
  - Rust: `CpModel::var_mut`, the objective setters, and `proto_mut`
  - Test: `solve.rs::resolves_after_model_change`
- `clone`, `proto`, serialization
  - Rust: `Clone`, `CpModel::proto`, `Message::encode_to_vec`
  - Test: `solve.rs::clone_and_serialize_round_trip`
- Variable, interval, and constraint `model_proto`; `CpModel.model_proto`
  - Rust: `CpModel::proto` or `proto_mut`; handles carry an index rather than a
    Python shared model reference
  - Test: `solve.rs::resolves_after_model_change`,
    `clone_and_serialize_round_trip`
- `Constraint.proto`, `index`, construction from an existing index
  - Rust: Before insertion, `Constraint::proto` or `into_proto`; after
    insertion, `model[ConstraintIndex]` to read and `proto_mut` to change; an
    existing constraint is accessible through `proto().constraints[index]`
  - Test: `model.rs::constraint_builder_sets_enforcement_and_name`,
    `looks_up_handles_by_index`
- `IntervalVar.proto`, construction from an existing index;
  `IntVar(model_proto, index)`
  - Rust: `model[interval]` to read; `CpModel::interval_var_from_index` and
    `int_var_from_index` return checked handles
  - Test: `model.rs::looks_up_handles_by_index`
- `get_or_make_index_from_constant`, `get_or_make_variable_index`,
  `get_or_make_boolean_index`, `rebuild_constant_map` inherited from
  `CpBaseModel`
  - Rust: `new_var(value..=value).index()`, typed handle indices,
    `new_var(bool)` / `to_bool_var`; no constant cache to rebuild
  - Test: `model.rs::a_bool_creates_a_fixed_literal`,
    `equivalence.rs::constants`
- `CpModel.is_boolean_value`, `object_is_a_true_literal`,
  `object_is_a_false_literal`
  - Rust: Typed `BoolVar` conversion and explicit inspection of
    `model[literal].domain` plus `is_negated`; Python's dynamic object
    predicates have no direct Rust helper
  - Test: `model.rs::only_domains_within_zero_and_one_are_literals` covers
    conversion; fixed-literal predicates have no direct test
- `validate`, `model_stats`
  - Rust: `CpModel::validate`, `CpModel::stats`, also on `CpModelProto`
  - Test: `diagnostics.rs`

## Formats and parameters

- `export_to_file`, `text_format`
  - Rust: `buffa::text` with the `text` feature, on by default, or
    `Message::encode_to_vec`, then `std::fs::write`. The caller picks text or
    binary, where Python picks by the file name
  - OR-Tools: none
  - Test: `formats.rs::reads_python_text_format`, `text_format_round_trips`,
    `solve.rs::clone_and_serialize_round_trip`
- `json_format`
  - Rust: `serde_json` with the `json` feature
  - OR-Tools: none
  - Test: `formats.rs::reads_python_json`
- reading `solver.parameters` defaults
  - Rust: `cpsat::prelude::default_parameters`
  - OR-Tools: reflection over `SatParameters`
  - Test: `diagnostics.rs::reads_parameter_defaults_from_or_tools`
- `Domain(lb, ub)`, `all_values`, `greater_or_equal`, `lower_or_equal`
  - Rust: `Domain::new`, `all_values`, `From<RangeFrom>`,
    `From<RangeToInclusive>`; `Domain::default()` is empty
  - OR-Tools: none
  - Test: `domain.rs::complements_at_the_i64_limits`, `normalizes_intervals`,
    `builds_half_lines_from_ranges`
- `Domain.from_values`, `from_intervals`, `from_flat_intervals`,
  `flattened_intervals`
  - Rust: Same-named `Domain` methods; `FromIterator<i64>` also builds one from
    values
  - OR-Tools: none
  - Test: `domain.rs::normalizes_intervals`, `flat_form_round_trips`
- `Domain.is_empty`, `min`, `max`, `size`, `contains`
  - Rust: Same-named `Domain` methods; empty bounds return `None`, and size
    saturates at `u64::MAX` rather than Python's `i64::MAX`
  - OR-Tools: none
  - Test: `domain.rs::reports_bounds_and_size`, `unions_and_intersects`
- `Domain.complement`, `union_with`, `intersection_with`
  - Rust: Same-named `Domain` methods
  - OR-Tools: none
  - Test: `domain.rs::complements_at_the_i64_limits`, `unions_and_intersects`
- `Domain.is_included_in`, `overlaps_with`
  - Rust: Same-named `Domain` methods
  - OR-Tools: none
  - Test: `domain.rs::compares_sets`
- `Domain.negation`, `addition_with`
  - Rust: Same-named `Domain` methods
  - OR-Tools: none
  - Test: `domain.rs::negates_and_adds_like_python`
- `Domain.__copy__`, `__deepcopy__`
  - Rust: `Clone`; Rust also provides normalized-set `PartialEq` and `Eq`
  - OR-Tools: none
  - Test: `domain.rs::flat_form_round_trips` uses cloning and equality
- `CpModelProto`, `CpSolverResponse`, `CpSolverStatus`, `SatParameters`
  - Rust: Generated types in `cpsat::proto`; the response's `status()` method
    returns a known `CpSolverStatus`
  - OR-Tools: none
  - Test: `protobuf.rs`, `formats.rs`, `solve.rs` cover encoding and
    representative field access
- `INT_MIN`, `INT_MAX`, `INT32_MIN`, `INT32_MAX`, solver status and
  decision-strategy constants
  - Rust: `i64::MIN/MAX`, `i32::MIN/MAX`, `CpSolverStatus` and
    `decision_strategy_proto` enums
  - OR-Tools: none
  - Test: `model.rs::comparisons_at_the_i64_limits_match_python`,
    `protobuf.rs::status_values_match_schema`, `equivalence.rs::search`
- Model, variable, interval, expression, constraint, and `Domain` string/repr,
  copy, and pickle protocols
  - Rust: Rust `Debug`, `Clone` or `Copy`, plus model Protobuf serialization;
    human-readable Python formatting and pickle bytes are Python-specific
  - OR-Tools: none
  - Test: `solve.rs::clone_and_serialize_round_trip` covers model clone and
    serialization; exact Python display/pickle output has no equivalent

## Rust additions

These items have no Python counterpart. Most replace Python's runtime dispatch
or mutable solver state with types.

- `CpSolver<'a, F>`: Borrows parameters, holds one event callback and a stop
  handle, and returns each response
- `Event`, `EventSet`, `NoEvents`: One tagged callback with a subscription set
  replaces Python's three callback kinds
- `StopHandle`: A shareable, sticky cancellation token
- `Evaluate`, `Objective`, `Hint`: Sealed traits that pick the value type at
  compile time, such as `bool` for a literal
- `Constraint` and `ConstraintIndex`: A constraint value before insertion, and
  the index `CpModel::add` returns
- `Error`, `Result`: Typed failures where Python raises or returns a string
- `CpSolverResponseView`: Buffa's zero-copy view of a solution inside an event
- `Index` on `CpModel`: `model[x]`, `model[interval]`, and `model[constraint]`
  read the stored message
- `CpModel::proto`, `proto_mut`, `into_proto`, `From`, `AsRef`: Move between
  `CpModel` and `CpModelProto`, and solve, validate, or describe a bare message
- `BoolVar::variable`, `is_negated`, and `index` on handles: Read a handle's
  encoding without a model
- `+=`, `-=`, `*=`, and `Sum` on `LinearExpr` and `FloatExpr`: Build expressions
  in loops and iterator chains
- `From` ranges, `FromIterator`, `PartialEq`, `intervals` on `Domain`: Build a
  domain from `0..=10` or values, and compare normalized sets
- `CpSolverStatus::has_solution`: True for `OPTIMAL` and `FEASIBLE`

## Limitations

- Python's `validate` returns an empty string or an error description. Rust's
  `validate` returns `Result<(), Error>`, with `Error::InvalidModel` carrying
  the description. Python's `export_to_file` returns a success Boolean; Rust
  callers handle the result of their chosen file-writing operation.
- Python's `solve` returns a status and stores its response on the solver. Rust
  returns an owned response, and response access is explicit. Variable accessors
  return `Option` for missing values, while Python can raise on unavailable
  response access. Constants can still be evaluated without a solution.
- Python constructors accept names, and constraint methods mutate an already
  inserted constraint. Rust names variables and intervals with `with_name`
  before `new_var`, configures a `Constraint` before insertion, and changes it
  later through `proto_mut`.
- Python accepts dynamic literal inputs and sequence/varargs overloads. Rust
  takes typed `BoolVar` literals and iterator inputs. Use `new_var(true)` for a
  fixed literal and `to_bool_var` for a 0-1 integer variable. Where Python mixes
  `True` into a list of presence literals, an `Option<BoolVar>` says the same
  without a variable.
- Integer expression overflow panics in Rust, while Python's native expression
  arithmetic can wrap. `Domain::size` and empty-bound behavior differ as noted
  in the Domain entries. Compare supported values and explicit edge cases
  instead of assuming all error paths match.
- Rust registers solution callbacks with `CpSolver::on_event` before solving and
  borrows the view only during the event. Python passes a reusable callback
  object to `solve` and exposes response-presence checks on that object.
- An unset `SatParameters` field reads as `None` instead of its proto2 default.
  `docs/buffa-proto2-defaults.md` explains why and how to read the defaults.
- An OR-Tools `CHECK` failure aborts the process, as in Python.
- Python rejects some arguments while it builds a constraint. Rust types reject
  some of them, such as a product of two variables, and the OR-Tools validator
  reports the rest, such as an empty arc list, at `validate` or as
  `MODEL_INVALID`.
- A stopped `StopHandle` stays stopped, so it also stops later solves, where
  Python's `stop_search` stops only the running solve. A new handle per solve
  gives Python's behavior.
- A handle does not know its model. A handle from another model reads the wrong
  variable, or `None` from a response and a panic from `model[handle]` when it
  is out of range.
- A list that mixes variables and expressions needs `.into()` on its elements,
  such as `[x.into(), y + 1]`, because a Rust array has one element type.
