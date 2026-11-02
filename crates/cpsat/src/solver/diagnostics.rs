//! Bridge helpers: the model and response diagnostics that Python's
//! `CpSatHelper` exposes, and the schema defaults of `SatParameters`.

use buffa::Message;

use crate::error::{Error, Result};
use crate::proto::{CpModelProto, CpSolverResponse, SatParameters};
use crate::solver::ffi;

impl CpModelProto {
    /// Checks the model with OR-Tools' own validation, which a solve also runs
    /// before it reports `MODEL_INVALID`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidModel`] with OR-Tools' message when the model
    /// is invalid.
    #[doc(alias("ValidateCpModel"))]
    pub fn validate(&self) -> Result<()> {
        let message = ffi::validate(&self.encode_to_vec())?;
        if message.is_empty() {
            Ok(())
        } else {
            Err(Error::InvalidModel(message))
        }
    }

    /// OR-Tools' human-readable statistics for the model.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge call fails.
    #[doc(alias("model_stats", "CpModelStats"))]
    pub fn stats(&self) -> Result<String> {
        ffi::model_stats(&self.encode_to_vec())
    }
}

impl CpSolverResponse {
    /// OR-Tools' human-readable statistics for the response.
    ///
    /// # Errors
    ///
    /// Returns an error if the bridge call fails.
    #[doc(alias("response_stats", "CpSolverResponseStats"))]
    pub fn stats(&self) -> Result<String> {
        ffi::response_stats(&self.encode_to_vec())
    }
}

/// Returns `SatParameters` with every singular field set to its OR-Tools
/// default, as Python's `solver.parameters` reads them.
///
/// Buffa reads an unset proto2 field as `None` instead of its `[default = X]`,
/// so this asks OR-Tools for the defaults. A solve applies the defaults
/// either way.
///
/// # Panics
///
/// If the bridge call fails or its output does not decode, which only a
/// broken build of the bridge can cause.
#[must_use]
pub fn default_parameters() -> SatParameters {
    let parameters =
        ffi::default_parameters().expect("the bridge reads the SatParameters defaults");
    SatParameters::decode_from_slice(parameters.bytes())
        .expect("the bridge encodes valid SatParameters")
}
