//! End-to-end tests through the OR-Tools solver. They form one test binary, so
//! OR-Tools links once and every test runs in one parallel pass.

mod callbacks;
mod common;
mod constraints;
mod diagnostics;
mod domain;
mod equivalence;
#[cfg(any(feature = "text", feature = "json"))]
mod formats;
mod model;
mod protobuf;
mod samples;
mod solve;
