//! Bridge helpers: the model and response diagnostics that Python's
//! `CpSatHelper` exposes, and the schema defaults of `SatParameters`.

use buffa::Message;
use cpsat_sys::CpsatBuffer;

use crate::error::{Error, Result};
use crate::proto::{CpModelProto, CpSolverResponse, SatParameters};
use crate::solver::ffi::{self, Buffer};

fn call_helper(
    helper: unsafe extern "C" fn(*const u8, usize, *mut CpsatBuffer, *mut CpsatBuffer) -> i32,
    bytes: &[u8],
) -> Result<String> {
    let mut output = Buffer::new();
    // SAFETY: bytes is valid for its length, and both buffers are empty
    // bridge buffers.
    ffi::call(|error| unsafe { helper(bytes.as_ptr(), bytes.len(), output.as_mut_ptr(), error) })?;
    Ok(output.text().into_owned())
}

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
        let message = call_helper(cpsat_sys::cpsat_model_validate, &self.encode_to_vec())?;
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
        call_helper(cpsat_sys::cpsat_model_stats, &self.encode_to_vec())
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
        call_helper(cpsat_sys::cpsat_response_stats, &self.encode_to_vec())
    }
}

/// Returns `SatParameters` with every singular field set to its OR-Tools
/// default, as Python's `solver.parameters` reads them.
///
/// Buffa reads an unset proto2 field as `None` instead of its `[default = X]`,
/// so this asks OR-Tools for the defaults. A solve applies the defaults
/// either way.
///
/// # Errors
///
/// Returns an error if the bridge call fails or its output does not decode.
pub fn default_parameters() -> Result<SatParameters> {
    let mut parameters = Buffer::new();
    // SAFETY: both buffers are empty bridge buffers.
    ffi::call(|error| unsafe {
        cpsat_sys::cpsat_parameters_default(parameters.as_mut_ptr(), error)
    })?;
    Ok(SatParameters::decode_from_slice(parameters.bytes())?)
}
