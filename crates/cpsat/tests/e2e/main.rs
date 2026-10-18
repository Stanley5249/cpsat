//! End-to-end tests through the native solver. They form one test binary, so
//! OR-Tools links once and every test runs in one parallel pass.

mod common;
mod constraints;
mod diagnostics;
mod domain;
mod equivalence;
mod events;
#[cfg(any(feature = "text", feature = "json"))]
mod formats;
mod model;
mod protobuf;
mod samples;
mod solve;
