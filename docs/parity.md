# Python CP-SAT parity

Each row maps a capability of the Python binding in OR-Tools v9.15
(`ortools/sat/python/cp_model.py` over `cp_model_helper.cc`) to this crate. Test
names are in `crates/cpsat/tests/e2e`. The asserted values were cross-checked
against `ortools==9.15.6755`, and `samples.rs` ports the official samples.

## Solver

| Python                                                           | Rust                                                          | Native                             | Test                                                                                                 |
| ---------------------------------------------------------------- | ------------------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `CpSolver.solve(model)`                                          | `Model::solve`, `CpModelProto::solve`, `Solver::solve`        | `SolveWrapper::Solve`              | `samples.rs::cp_sat_example`, `events.rs::solver_solves_again`                                       |
| `CpSolver.parameters`                                            | `SatParameters` passed to `Solver::new`, `Solver::params`     | `SolveWrapper::SetParameters`      | `events.rs::forwards_logs_and_parameters`                                                            |
| `max_time_in_seconds`                                            | `SatParameters` field                                         | none                               | `solve.rs::honors_time_limit`                                                                        |
| `CpSolverSolutionCallback.on_solution_callback`                  | `Event::Solution`                                             | `SolutionCallback`                 | `samples.rs::solve_and_print_intermediate_solutions`                                                 |
| `CpSolverSolutionCallback.value`, `boolean_value`, `float_value` | `CpSolverResponseView::value`, `boolean_value`, `float_value` | none                               | `events.rs::solution_view_reads_values_and_converts_to_owned`                                        |
| `CpSolver.best_bound_callback`                                   | `Event::BestBound`                                            | `AddBestBoundCallback`             | `events.rs::reports_best_bounds`                                                                     |
| `CpSolver.log_callback`                                          | `Event::Log`                                                  | `AddLogCallback`                   | `events.rs::forwards_logs_and_parameters`                                                            |
| `CpSolverSolutionCallback.stop_search`                           | return `ControlFlow::Break`                                   | `StopSearch`                       | `samples.rs::stop_after_n_solutions`                                                                 |
| `CpSolver.stop_search` from another thread                       | `StopHandle::stop` with `Solver::with_stop_handle`            | `StopSearch`                       | `events.rs::stops_from_another_thread`                                                               |
| Exception in a callback stops the search and is reraised         | panic stops the search and resumes after the solve            | none                               | `events.rs::callback_panic_stops_and_resumes_after_solve`                                            |
| `enumerate_all_solutions`                                        | `SatParameters` field with `EventSet::SOLUTION`               | none                               | `samples.rs::search_for_all_solutions`                                                               |
| `fill_additional_solutions_in_response`                          | `CpSolverResponse::additional_solutions`                      | none                               | `solve.rs::fills_additional_solutions_when_asked`                                                    |
| `CpSolver.value`, `boolean_value`, `float_value`                 | `CpSolverResponse::value`, `boolean_value`, `float_value`     | none                               | `solve.rs::reads_negated_literals_and_expressions`, `solve.rs::solves_a_float_objective_like_python` |
| `CpSolver.status_name`                                           | `Display for CpSolverStatus`                                  | none                               | `solve.rs::displays_status_like_python`                                                              |
| `CpSolver.response_stats`                                        | `CpSolverResponse::response_stats`                            | `CpSatHelper::SolverResponseStats` | `diagnostics.rs::describes_model_and_response`                                                       |
| `CpSolver.sufficient_assumptions_for_infeasibility`              | `CpSolverResponse` field                                      | none                               | `samples.rs::assumptions`                                                                            |

## Model

`equivalence.rs` checks that each family writes the same `CpModelProto` as
Python, from the fixtures that `just fixtures` regenerates, and `constraints.rs`
solves each global and scheduling family.

| Python                                                                        | Rust                                                                                   | Test                                                                                                  |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `CpModel()`, `CpModel(model_proto)`                                           | `Model::new`, `Model::from(proto)`                                                     | `solve.rs::clone_and_serialize_round_trip`                                                            |
| `new_int_var`, `new_int_var_from_domain`                                      | `Model::int_var` with a range or a `Domain`                                            | `samples.rs`, `domain.rs`                                                                             |
| `new_bool_var`                                                                | `Model::bool_var`                                                                      | `samples.rs::boolean_product`                                                                         |
| `new_constant`, `True`, `False`                                               | `Model::constant`, `bool_constant`                                                     | `model.rs::constants_and_true_literals_solve_to_their_values`                                         |
| `new_interval_var`, `new_fixed_size_interval_var`                             | `Model::interval_var`, `fixed_size_interval_var`                                       | `samples.rs::no_overlap`, `equivalence.rs::scheduling`                                                |
| `new_optional_interval_var`, `new_optional_fixed_size_interval_var`           | `Model::optional_interval_var`, `optional_fixed_size_interval_var`                     | `samples.rs::ranking_circuit`, `equivalence.rs::scheduling`                                           |
| `IntervalVar.start_expr`, `size_expr`, `end_expr`, `presence_literals`        | `Model::interval_start`, `interval_size`, `interval_end`, `interval_presence_literals` | `model.rs::reads_interval_parts_like_python`                                                          |
| `new_*_series`                                                                | iterators and `collect`                                                                | `samples.rs::nqueens`                                                                                 |
| `LinearExpr` arithmetic, `sum`, `weighted_sum`                                | `+`, `-`, `*`, unary `-`, `+=`, and `Sum` on `LinearExpr`                              | `equivalence.rs::linear`, `model.rs`                                                                  |
| `~b`, `Not()`                                                                 | `!b`                                                                                   | `model.rs::literals_negate_and_name_their_variable`                                                   |
| A 0-1 `IntVar` where a literal goes                                           | `Model::to_literal`                                                                    | `model.rs::a_zero_one_variable_serves_as_a_literal`                                                   |
| `add(x == y)` and the other comparisons                                       | `eq`, `ne`, `le`, `lt`, `ge`, `gt`                                                     | `equivalence.rs::linear`                                                                              |
| `add_linear_constraint`, `add_linear_expression_in_domain`                    | `in_domain` with a range or a `Domain`                                                 | `equivalence.rs::linear`, `samples.rs::step_function`                                                 |
| `add(True)`, `add(False)`                                                     | `Constraint::bool_and([])`, `Constraint::bool_or([])`                                  | none                                                                                                  |
| `only_enforce_if`, `with_name`                                                | `Constraint::only_if`, `with_name`                                                     | `model.rs::constraint_builder_sets_enforcement_and_name`                                              |
| `add_bool_or`, `add_at_least_one`, `add_bool_and`                             | `Constraint::bool_or`, `bool_and`                                                      | `equivalence.rs::boolean`                                                                             |
| `add_at_most_one`, `add_exactly_one`, `add_bool_xor`, `add_implication`       | `Constraint::at_most_one`, `exactly_one`, `bool_xor`, `implication`                    | `equivalence.rs::boolean`                                                                             |
| `add_all_different`, `add_element`, `add_inverse`                             | `Constraint::all_different`, `element`, `inverse`                                      | `equivalence.rs::global_constraints`, `samples.rs::nqueens`, `constraints.rs`                         |
| `add_allowed_assignments`, `add_forbidden_assignments`                        | `Constraint::allowed_assignments`, `forbidden_assignments`                             | `equivalence.rs::global_constraints`, `constraints.rs`                                                |
| `add_automaton`                                                               | `Constraint::automaton`                                                                | `equivalence.rs::global_constraints`, `constraints.rs`                                                |
| `add_circuit`, `add_multiple_circuit`                                         | `Constraint::circuit`, `multiple_circuit`                                              | `samples.rs::ranking_circuit`, `constraints.rs`                                                       |
| `add_min_equality`, `add_max_equality`, `add_abs_equality`                    | `Constraint::min_equality`, `max_equality`, `abs_equality`                             | `equivalence.rs::arithmetic`, `samples.rs::no_overlap`                                                |
| `add_division_equality`, `add_modulo_equality`, `add_multiplication_equality` | `Constraint::division_equality`, `modulo_equality`, `multiplication_equality`          | `equivalence.rs::arithmetic`, `samples.rs::non_linear`                                                |
| `add_no_overlap`, `add_no_overlap_2d`                                         | `Constraint::no_overlap`, `no_overlap_2d` with `(x, y)` pairs                          | `samples.rs::minimal_jobshop`, `equivalence.rs::scheduling`, `constraints.rs`                         |
| `add_cumulative`                                                              | `Constraint::cumulative`                                                               | `equivalence.rs::scheduling`, `constraints.rs`                                                        |
| `add_reservoir_constraint`, `add_reservoir_constraint_with_active`            | `Constraint::reservoir`, `reservoir_with_active`                                       | `equivalence.rs::scheduling`, `constraints.rs`                                                        |
| `add_map_domain`                                                              | a loop of `eq` and `ne` with `only_if`                                                 | none                                                                                                  |
| `minimize`, `maximize`, `has_objective`, `clear_objective`                    | `Model::minimize`, `maximize`, `has_objective`, `clear_objective`                      | `model.rs::replaces_and_clears_the_objective`                                                         |
| `minimize`, `maximize` with float coefficients                                | `FloatExpr`, `Model::minimize_float`, `maximize_float`                                 | `equivalence.rs::minimize_float`, `maximize_float`                                                    |
| `add_decision_strategy`                                                       | `Model::add_decision_strategy`                                                         | `samples.rs::channeling`                                                                              |
| `add_hint`, `clear_hints`                                                     | `Model::add_hint`, `add_bool_hint`, `clear_hints`                                      | `samples.rs::solution_hinting`, `solve.rs::first_solution_follows_the_hint`, `equivalence.rs::search` |
| `add_assumption`, `add_assumptions`, `clear_assumptions`                      | `Model::add_assumption`, `add_assumptions`, `clear_assumptions`                        | `samples.rs::assumptions`, `model.rs::adds_and_clears_assumptions`                                    |
| `get_int_var_from_proto_index` and the bool and interval versions             | `Model::int_var_from_index`, `bool_var_from_index`, `interval_var_from_index`          | `model.rs::looks_up_handles_by_index`                                                                 |
| `IntVar.name`, `Constraint.proto`, `CpModel.name`                             | `model[handle]`, `proto_mut().name`                                                    | `model.rs::literals_negate_and_name_their_variable`                                                   |
| `remove_all_names`                                                            | `Model::remove_all_names`                                                              | `model.rs::constraint_builder_sets_enforcement_and_name`                                              |
| Change domains, objectives, constraints, then solve again                     | `model[handle]`, `proto_mut`                                                           | `solve.rs::resolves_after_model_change`                                                               |
| `clone`, `proto`, serialization                                               | `Clone`, `Model::proto`, `Message::encode_to_vec`                                      | `solve.rs::clone_and_serialize_round_trip`                                                            |
| `validate`, `model_stats`                                                     | `Model::validate`, `Model::model_stats`, also on `CpModelProto`                        | `diagnostics.rs`                                                                                      |

## Formats and parameters

| Python                                         | Rust                                                                                                                                                                           | Native                          | Test                                                                                                          |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `export_to_file`, `text_format`                | `buffa::text` with the `text` feature, on by default, or `Message::encode_to_vec`, then `std::fs::write`. The caller picks text or binary, where Python picks by the file name | none                            | `formats.rs::reads_python_text_format`, `text_format_round_trips`, `solve.rs::clone_and_serialize_round_trip` |
| `json_format`                                  | `serde_json` with the `json` feature                                                                                                                                           | none                            | `formats.rs::reads_python_json`                                                                               |
| reading `solver.parameters` defaults           | `cpsat::default_parameters`                                                                                                                                                    | reflection over `SatParameters` | `diagnostics.rs::reads_parameter_defaults_from_or_tools`                                                      |
| `Domain`, `greater_or_equal`, `lower_or_equal` | `cpsat::Domain`, `From<RangeFrom>`, `From<RangeToInclusive>`                                                                                                                   | none                            | `domain.rs`                                                                                                   |

## Limitations

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
