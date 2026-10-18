# Proto2 defaults in Buffa

`sat_parameters.proto` is proto2, and most of its fields declare a default:

```protobuf
optional double max_time_in_seconds = 36 [default = inf];
optional bool cp_model_presolve = 86 [default = true];
```

Buffa generates each `optional` field as `Option<T>` and, by design, no accessor
that returns the `[default = X]` value. Its README says that custom defaults
apply only to `required` fields through `impl Default`, and that callers read
optional fields with pattern matching or `.unwrap_or(X)`. So an unset
`SatParameters` field reads as `None`, where Python's
`solver.parameters.max_time_in_seconds` reads `inf`.

`None` changes nothing for a solve, because OR-Tools applies its defaults to
every unset field. It matters only to code that reads a parameter, and that code
has two options:

- `cpsat::default_parameters()` returns `SatParameters` with every singular
  field set to its OR-Tools default, read through C++ reflection, so the values
  follow the pinned OR-Tools version instead of a copy.
- `.unwrap_or(X)` on one field, which copies `X` from the `.proto` and drifts
  when the schema changes.
