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
| `CpSolver.value`, `boolean_value`                        | `CpSolverResponse::value`, `boolean_value`         | none                               | `solve.rs::reads_negated_references`                          |
| `CpSolver.status_name`                                   | `Display for CpSolverStatus`                       | none                               | `solve.rs::displays_status_like_python`                       |
| `CpSolver.response_stats`                                | `cpsat::response_stats`                            | `CpSatHelper::SolverResponseStats` | `diagnostics.rs::describes_model_and_response`                |
| `CpSolver.sufficient_assumptions_for_infeasibility`      | `CpSolverResponse` field                           | none                               | `samples.rs::assumptions`                                     |

## Model

| Python                                                    | Rust                                                                | Native                       | Test                                             |
| --------------------------------------------------------- | ------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------ |
| `new_int_var`, `new_bool_var`                             | `CpModelProto::add_int_var`, `add_bool_var`                         | none                         | `samples.rs`                                     |
| `add` for a linear constraint                             | `add_linear`, or `add_constraint` with `LinearConstraintProto::new` | none                         | `samples.rs::cp_sat_example`                     |
| `only_enforce_if`                                         | the returned constraint's `enforcement_literal`                     | none                         | `samples.rs::assumptions`                        |
| `Not()`                                                   | `cpsat::not`                                                        | none                         | `solve.rs::reads_negated_references`             |
| `minimize`, `maximize`                                    | `CpModelProto::minimize`, `maximize`                                | none                         | `samples.rs::cp_sat_example`                     |
| Change domains, objectives, constraints, then solve again | edit the message                                                    | none                         | `solve.rs::resolves_after_model_change`          |
| `clone`, `proto`, serialization                           | `Clone`, `Message::encode_to_vec`, `decode_from_slice`              | none                         | `solve.rs::clone_and_serialize_round_trip`       |
| `add_hint`, `clear_hints`                                 | `CpModelProto::add_hint`, `clear_hints`                             | none                         | `samples.rs::solution_hinting`                   |
| `add_assumptions`, `clear_assumptions`                    | `CpModelProto::assumptions`                                         | none                         | `samples.rs::assumptions`                        |
| `validate`                                                | `cpsat::validate_model`                                             | `CpSatHelper::ValidateModel` | `diagnostics.rs::reports_why_a_model_is_invalid` |
| `model_stats`                                             | `cpsat::model_stats`                                                | `CpSatHelper::ModelStats`    | `diagnostics.rs::describes_model_and_response`   |

Other constraints, such as `add_all_different` or `add_no_overlap`, are fields
of `ConstraintProto` and need no helper.

## Formats and parameters

| Python                                             | Rust                                            | Native                          | Test                                                     |
| -------------------------------------------------- | ----------------------------------------------- | ------------------------------- | -------------------------------------------------------- |
| `export_to_file` with a `.txt` name, `text_format` | `buffa::text`, `text` feature, on by default    | none                            | `formats.rs::reads_python_text_format`                   |
| `json_format`                                      | `serde_json` with the `json` feature            | none                            | `formats.rs::reads_python_json`                          |
| reading `solver.parameters` defaults               | `cpsat::default_parameters`                     | reflection over `SatParameters` | `diagnostics.rs::reads_parameter_defaults_from_or_tools` |
| `Domain`, `new_int_var_from_domain`                | `cpsat::Domain`, `CpModelProto::add_int_var_in` | none                            | `domain.rs`                                              |

## Limitations

- An unset `SatParameters` field reads as `None`, because Buffa generates no
  getters for proto2 `[default = X]` values. `cpsat::default_parameters` fills
  them, and `docs/buffa-proto2-defaults.md` drafts the upstream request.
- `Domain` covers set operations only, not Python's arithmetic such as
  `addition_with` or `multiplication_by`.
- An OR-Tools `CHECK` failure aborts the process in both Python and Rust.
  `validate_model` catches invalid models first, and a subprocess isolates
  untrusted ones.
