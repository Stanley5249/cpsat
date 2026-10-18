# Draft issue: proto2 default getters in Buffa

A draft for [anthropics/buffa](https://github.com/anthropics/buffa), to file
later. `cpsat::default_parameters` works around it until then.

## Title

Generate accessors that return the `[default = X]` value of proto2 optional
fields

## Body

Buffa 0.9.2 represents a proto2 `optional` scalar as `Option<T>`, so an unset
field reads as `None`. The guide says that `[default = X]` on `optional` fields
does not generate unwrap-to-default accessors, and that custom defaults apply
only to `required` fields.

Schemas such as OR-Tools' `sat_parameters.proto` rely on defaults heavily:

```protobuf
optional double max_time_in_seconds = 36 [default = inf];
optional bool cp_model_presolve = 86 [default = true];
```

A caller that wants the effective value has to copy each default from the
`.proto` by hand, and the copies drift when the schema changes.

Request: for each proto2 `optional` scalar or enum field with a default,
generate an accessor such as `fn max_time_in_seconds_or_default(&self) -> f64`,
matching what prost generates as `fn max_time_in_seconds(&self) -> f64`. A
`const` per default would also work.
