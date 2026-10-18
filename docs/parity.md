# Python CP-SAT parity

Each row maps a capability of the Python binding in OR-Tools v9.15
(`ortools/sat/python/cp_model.py` over `cp_model_helper.cc`) to this crate. Test
names are in `crates/cpsat/tests`. The asserted values were cross-checked
against `ortools==9.15.6755`, and `samples.rs` ports the official samples.

## Solver

| Python                                                   | Rust                                               | Native                             | Test                                                          |
| -------------------------------------------------------- | -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------- |
| `CpSolver.solve(model)`                                  | `cpsat::solve`, `Solver::solve`                    | `SolveWrapper::Solve`              | `samples.rs::cp_sat_example`                                  |
| `CpSolver.parameters`                                    | `SatParameters`, `Solver::params_mut`              | `SolveWrapper::SetParameters`      | `events.rs::forwards_logs_and_parameters`                     |
| `max_time_in_seconds`                                    | `SatParameters` field                              | none                               | `solve.rs::honors_time_limit`                                 |
| `CpSolverSolutionCallback.on_solution_callback`          | `Event::Solution`                                  | `SolutionCallback`                 | `samples.rs::solve_and_print_intermediate_solutions`          |
| `CpSolverSolutionCallback.value`, `boolean_value`        | `CpSolverResponseView::value`, `boolean_value`     | none                               | `events.rs::solution_view_reads_values_and_converts_to_owned` |
| `CpSolver.best_bound_callback`                           | `Event::BestBound`                                 | `AddBestBoundCallback`             | `events.rs::reports_best_bounds`                              |
| `CpSolver.log_callback`                                  | `Event::Log`                                       | `AddLogCallback`                   | `events.rs::forwards_logs_and_parameters`                     |
| `CpSolverSolutionCallback.stop_search`                   | return `ControlFlow::Break`                        | `StopSearch`                       | `samples.rs::stop_after_n_solutions`                          |
| `CpSolver.stop_search` from another thread               | `StopHandle::stop`                                 | `StopSearch`                       | `events.rs::stops_from_another_thread`                        |
| Exception in a callback stops the search and is reraised | panic stops the search and resumes after the solve | none                               | `events.rs::callback_panic_stops_and_resumes_after_solve`     |
| `enumerate_all_solutions`                                | `SatParameters` field with `EventSet::SOLUTION`    | none                               | `samples.rs::search_for_all_solutions`                        |
| `fill_additional_solutions_in_response`                  | `CpSolverResponse::additional_solutions`           | none                               | `solve.rs::fills_additional_solutions_when_asked`             |
| `CpSolver.value`, `boolean_value`                        | `CpSolverResponse::value`, `boolean_value`         | none                               | `solve.rs::reads_negated_literals_and_expressions`            |
| `CpSolver.status_name`                                   | `Display for CpSolverStatus`                       | none                               | `solve.rs::displays_status_like_python`                       |
| `CpSolver.response_stats`                                | `cpsat::response_stats`                            | `CpSatHelper::SolverResponseStats` | `diagnostics.rs::describes_model_and_response`                |
| `CpSolver.sufficient_assumptions_for_infeasibility`      | `CpSolverResponse` field                           | none                               | `samples.rs::assumptions`                                     |

## Model

`equivalence.rs` checks that each family writes the same `CpModelProto` as
Python, from the fixtures that `just fixtures` regenerates.

| Python                                                                        | Rust                                                                          | Test                                                          |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `CpModel()`, `CpModel(model_proto)`                                           | `Model::new`, `Model::from(proto)`                                            | `solve.rs::clone_and_serialize_round_trip`                    |
| `new_int_var`, `new_int_var_from_domain`                                      | `Model::int_var` with a range or a `Domain`                                   | `samples.rs`, `domain.rs`                                     |
| `new_bool_var`                                                                | `Model::bool_var`                                                             | `samples.rs::boolean_product`                                 |
| `new_constant`, `True`, `False`                                               | `Model::constant`, `bool_constant`                                            | `model.rs::constants_and_true_literals_solve_to_their_values` |
| `new_interval_var`, `new_fixed_size_interval_var`                             | `Model::interval_var`, `fixed_size_interval_var`                              | `samples.rs::no_overlap`, `equivalence.rs::scheduling`        |
| `new_optional_interval_var`, `new_optional_fixed_size_interval_var`           | `Model::optional_interval_var`, `optional_fixed_size_interval_var`            | `samples.rs::ranking_circuit`, `equivalence.rs::scheduling`   |
| `new_*_series`                                                                | iterators and `collect`                                                       | `samples.rs::nqueens`                                         |
| `LinearExpr` arithmetic, `sum`, `weighted_sum`                                | `+`, `-`, `*`, unary `-`, `+=`, and `Sum` on `LinearExpr`                     | `equivalence.rs::linear`, `model.rs`                          |
| `~b`, `Not()`                                                                 | `!b`                                                                          | `model.rs::literals_negate_and_name_their_variable`           |
| `add(x == y)` and the other comparisons                                       | `eq`, `ne`, `le`, `lt`, `ge`, `gt`                                            | `equivalence.rs::linear`                                      |
| `add_linear_constraint`, `add_linear_expression_in_domain`                    | `in_domain` with a range or a `Domain`                                        | `equivalence.rs::linear`, `samples.rs::step_function`         |
| `add(True)`, `add(False)`                                                     | `Constraint::bool_and([])`, `Constraint::bool_or([])`                         | none                                                          |
| `only_enforce_if`, `with_name`                                                | `Constraint::only_if`, `with_name`                                            | `model.rs::constraint_builder_sets_enforcement_and_name`      |
| `add_bool_or`, `add_at_least_one`, `add_bool_and`                             | `Constraint::bool_or`, `bool_and`                                             | `equivalence.rs::boolean`                                     |
| `add_at_most_one`, `add_exactly_one`, `add_bool_xor`, `add_implication`       | `Constraint::at_most_one`, `exactly_one`, `bool_xor`, `implication`           | `equivalence.rs::boolean`                                     |
| `add_all_different`, `add_element`, `add_inverse`                             | `Constraint::all_different`, `element`, `inverse`                             | `equivalence.rs::global_constraints`, `samples.rs::nqueens`   |
| `add_allowed_assignments`, `add_forbidden_assignments`                        | `Table::allowed`, `Table::forbidden`                                          | `equivalence.rs::global_constraints`                          |
| `add_automaton`                                                               | `Automaton`                                                                   | `equivalence.rs::global_constraints`                          |
| `add_circuit`, `add_multiple_circuit`                                         | `Circuit`, `MultipleCircuit`                                                  | `samples.rs::ranking_circuit`                                 |
| `add_min_equality`, `add_max_equality`, `add_abs_equality`                    | `Constraint::min_equality`, `max_equality`, `abs_equality`                    | `equivalence.rs::arithmetic`, `samples.rs::no_overlap`        |
| `add_division_equality`, `add_modulo_equality`, `add_multiplication_equality` | `Constraint::division_equality`, `modulo_equality`, `multiplication_equality` | `equivalence.rs::arithmetic`, `samples.rs::non_linear`        |
| `add_no_overlap`, `add_no_overlap_2d`                                         | `Constraint::no_overlap`, `no_overlap_2d` with `(x, y)` pairs                 | `samples.rs::minimal_jobshop`, `equivalence.rs::scheduling`   |
| `add_cumulative`                                                              | `Cumulative`                                                                  | `equivalence.rs::scheduling`                                  |
| `add_reservoir_constraint`, `add_reservoir_constraint_with_active`            | `Reservoir` with `event` or `active_event`                                    | `equivalence.rs::scheduling`                                  |
| `add_map_domain`                                                              | a loop of `eq` and `ne` with `only_if`                                        | none                                                          |
| `minimize`, `maximize`, `has_objective`, `clear_objective`                    | `Model::minimize`, `maximize`, `has_objective`, `clear_objective`             | `model.rs::replaces_and_clears_the_objective`                 |
| floating-point objective                                                      | `proto_mut().floating_point_objective`                                        | none                                                          |
| `add_decision_strategy`                                                       | `Model::add_decision_strategy`                                                | `samples.rs::channeling`                                      |
| `add_hint`, `clear_hints`                                                     | `Model::add_hint`, `add_bool_hint`, `clear_hints`                             | `samples.rs::solution_hinting`                                |
| `add_assumption`, `add_assumptions`, `clear_assumptions`                      | `Model::add_assumption`, `add_assumptions`, `clear_assumptions`               | `samples.rs::assumptions`                                     |
| `get_int_var_from_proto_index` and the bool and interval versions             | `Model::int_var_from_index`, `bool_var_from_index`, `interval_var_from_index` | `model.rs::looks_up_handles_by_index`                         |
| `IntVar.name`, `Constraint.proto`, `CpModel.name`                             | `model[handle]`, `proto_mut().name`                                           | `model.rs::literals_negate_and_name_their_variable`           |
| `remove_all_names`                                                            | `Model::remove_all_names`                                                     | `model.rs::constraint_builder_sets_enforcement_and_name`      |
| Change domains, objectives, constraints, then solve again                     | `model[handle]`, `proto_mut`                                                  | `solve.rs::resolves_after_model_change`                       |
| `clone`, `proto`, serialization                                               | `Clone`, `Model::proto`, `Message::encode_to_vec`                             | `solve.rs::clone_and_serialize_round_trip`                    |
| `validate`, `model_stats`                                                     | `cpsat::validate_model`, `model_stats` on `model.proto()`                     | `diagnostics.rs`                                              |

## Formats and parameters

| Python                                             | Rust                                         | Native                          | Test                                                     |
| -------------------------------------------------- | -------------------------------------------- | ------------------------------- | -------------------------------------------------------- |
| `export_to_file` with a `.txt` name, `text_format` | `buffa::text`, `text` feature, on by default | none                            | `formats.rs::reads_python_text_format`                   |
| `json_format`                                      | `serde_json` with the `json` feature         | none                            | `formats.rs::reads_python_json`                          |
| reading `solver.parameters` defaults               | `cpsat::default_parameters`                  | reflection over `SatParameters` | `diagnostics.rs::reads_parameter_defaults_from_or_tools` |
| `Domain`                                           | `cpsat::Domain`                              | none                            | `domain.rs`                                              |

## Limitations

- An unset `SatParameters` field reads as `None`, because Buffa generates no
  getters for proto2 `[default = X]` values. `cpsat::default_parameters` fills
  them, and `docs/buffa-proto2-defaults.md` drafts the upstream request.
- `Domain` covers set operations only, not Python's arithmetic such as
  `addition_with` or `multiplication_by`.
- An OR-Tools `CHECK` failure aborts the process in both Python and Rust.
  `validate_model` catches invalid models first, and a subprocess isolates
  untrusted ones.
- Python checks arguments while it builds a constraint, such as affine-only
  expressions or a non-empty arc list. Rust leaves these checks to the OR-Tools
  validator, so the error appears at `validate_model` or as `MODEL_INVALID`.
- `Model::constant` and `bool_constant` add a variable on every call, where
  Python reuses one variable per value.
- A handle does not know its model. A handle from another model reads the wrong
  variable, or `None` from a response and a panic from `model[handle]` when it
  is out of range.
- A list that mixes variables and expressions needs `.into()` on its elements,
  such as `[x.into(), y + 1]`, because a Rust array has one element type.
