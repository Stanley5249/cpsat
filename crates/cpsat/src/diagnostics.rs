//! Native model and response diagnostics, the same functions Python's
//! `CpSatHelper` exposes.

use buffa::Message;
use cpsat_sys::CpsatBuffer;

use crate::error::{Error, Result};
use crate::native::Buffer;
use crate::proto::{CpModelProto, CpSolverResponse};

fn call(
    helper: unsafe extern "C" fn(*const u8, usize, *mut CpsatBuffer, *mut CpsatBuffer) -> i32,
    bytes: &[u8],
) -> Result<String> {
    let mut output = Buffer::new();
    let mut error = Buffer::new();
    // SAFETY: bytes is valid for its length, and both buffers are empty
    // bridge buffers.
    let code = unsafe {
        helper(
            bytes.as_ptr(),
            bytes.len(),
            output.as_mut_ptr(),
            error.as_mut_ptr(),
        )
    };
    crate::native::check(code, &error)?;
    Ok(output.text().into_owned())
}

/// Checks the model with OR-Tools' own validation, which a solve also runs
/// before it reports `MODEL_INVALID`.
///
/// # Errors
///
/// Returns [`Error::InvalidModel`] with OR-Tools' message when the model is
/// invalid.
pub fn validate_model(model: &CpModelProto) -> Result<()> {
    let message = call(cpsat_sys::cpsat_validate_model, &model.encode_to_vec())?;
    if message.is_empty() {
        Ok(())
    } else {
        Err(Error::InvalidModel(message))
    }
}

/// Returns OR-Tools' human-readable statistics for a model.
///
/// # Errors
///
/// Returns an error if the native call fails.
pub fn model_stats(model: &CpModelProto) -> Result<String> {
    call(cpsat_sys::cpsat_model_stats, &model.encode_to_vec())
}

/// Returns OR-Tools' human-readable statistics for a response.
///
/// # Errors
///
/// Returns an error if the native call fails.
pub fn response_stats(response: &CpSolverResponse) -> Result<String> {
    call(cpsat_sys::cpsat_response_stats, &response.encode_to_vec())
}
