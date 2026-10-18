# Python CP-SAT parity

Each row maps a capability of the Python binding in OR-Tools v9.15
(`ortools/sat/python/cp_model.py` over `cp_model_helper.cc`) to this crate. The
values asserted in the tests were cross-checked against `ortools==9.15.6755` on
Python.

## Solver

| Python                                                          | Rust                                               | Native                             | Test                                              |
| --------------------------------------------------------------- | -------------------------------------------------- | ---------------------------------- | ------------------------------------------------- |
| `CpSolver.solve(model)`                                         | `cpsat::solve`                                     | `SolveWrapper::Solve`              | `solve.rs::solves_to_optimality`                  |
| `CpSolver.parameters`                                           | `SatParameters` passed as one message              | `SolveWrapper::SetParameters`      | `events.rs::forwards_logs_and_parameters`         |
| `num_workers`, presolve, search, time and gap limits            | `SatParameters` fields                             | none                               | `solve.rs::honors_time_limit`                     |
| `CpSolverSolutionCallback.on_solution_callback`                 | `Event::Solution` in `solve_with`                  | `SolutionCallback`                 | `events.rs::enumerates_all_solutions`             |
| `CpSolverSolutionCallback.value`, `objective_value`, statistics | fields of `CpSolverResponseView`                   | none                               | `events.rs::solution_view_converts_to_owned`      |
| `CpSolver.best_bound_callback`                                  | `Event::BestBound`                                 | `AddBestBoundCallback`             | `events.rs::reports_best_bounds`                  |
| `CpSolver.log_callback`                                         | `Event::Log`                                       | `AddLogCallback`                   | `events.rs::forwards_logs_and_parameters`         |
| `CpSolverSolutionCallback.stop_search`                          | return `ControlFlow::Break`                        | `StopSearch`                       | `events.rs::callback_stops_search`                |
| `CpSolver.stop_search` from another thread                      | `StopHandle::stop`                                 | `StopSearch`                       | `events.rs::stops_from_another_thread`            |
| Exception in a callback stops the search and is reraised        | panic stops the search and resumes after the solve | none                               | `events.rs::callback_panic_resumes_after_solve`   |
| `enumerate_all_solutions`                                       | `SatParameters` field with `Events::SOLUTION`      | none                               | `events.rs::enumerates_all_solutions`             |
| `fill_additional_solutions_in_response`                         | `CpSolverResponse::additional_solutions`           | none                               | `solve.rs::fills_additional_solutions_when_asked` |
| `CpSolver.value`, `boolean_value`, `status_name`, `wall_time`   | `CpSolverResponse` fields                          | none                               | `solve.rs`                                        |
| `CpSolver.response_stats`                                       | `cpsat::response_stats`                            | `CpSatHelper::SolverResponseStats` | `diagnostics.rs::describes_model_and_response`    |
| `CpSolver.sufficient_assumptions_for_infeasibility`             | `CpSolverResponse` field                           | none                               | `solve.rs::returns_sufficient_assumptions`        |

## Model

| Python                                                    | Rust                                                                                | Native                       | Test                                             |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------ |
| `new_int_var`, `add`, constraint helpers                  | push to `CpModelProto` fields                                                       | none                         | `tests/common/mod.rs`                            |
| `minimize`, `maximize`                                    | `CpObjectiveProto`, negated with `scaling_factor = -1` for maximize, as Python does | none                         | `solve.rs::solves_to_optimality`                 |
| Change domains, objectives, constraints, then solve again | edit the message                                                                    | none                         | `solve.rs::resolves_after_model_change`          |
| `clone`, `proto`, serialization                           | `Clone`, `Message::encode_to_vec`, `decode_from_slice`                              | none                         | `solve.rs::clone_and_serialize_round_trip`       |
| `add_hint`, `clear_hints`                                 | `CpModelProto::solution_hint`                                                       | none                         | `solve.rs::keeps_a_feasible_hint`                |
| `add_assumptions`, `clear_assumptions`                    | `CpModelProto::assumptions`                                                         | none                         | `solve.rs::returns_sufficient_assumptions`       |
| `validate`                                                | `cpsat::validate`                                                                   | `CpSatHelper::ValidateModel` | `diagnostics.rs::reports_why_a_model_is_invalid` |
| `model_stats`                                             | `cpsat::model_stats`                                                                | `CpSatHelper::ModelStats`    | `diagnostics.rs::describes_model_and_response`   |

## Limitations

- Buffa does not generate getters for proto2 `[default = X]` values, so an unset
  `SatParameters` field reads as `None` rather than its OR-Tools default. The
  solver still applies the default.
- Text-format I/O is out of scope: Python's `export_to_file` with a `.txt` name
  and string parameters. Binary export is `encode_to_vec` plus `std::fs::write`.
- `Domain` arithmetic is out of scope. Domains are flat `[min, max, ...]` lists
  in the schema.
- An OR-Tools `CHECK` failure aborts the process in both Python and Rust.
