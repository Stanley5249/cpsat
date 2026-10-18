//! Native model and response diagnostics, the same functions Python's
//! `CpSatHelper` exposes.

use anyhow::{Result, bail};
use buffa::Message;

use crate::native::Buffer;
use crate::proto::{CpModelProto, CpSolverResponse};

type Helper = unsafe extern "C" fn(
    *const u8,
    usize,
    *mut cpsat_sys::CpsatBuffer,
    *mut cpsat_sys::CpsatBuffer,
) -> i32;

fn call(helper: Helper, bytes: &[u8]) -> Result<String> {
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
    Ok(output.text())
}

/// Checks the model with OR-Tools' own validation, which `solve` also runs
/// before reporting `MODEL_INVALID`.
///
/// # Errors
///
/// Returns the validation message when the model is invalid.
pub fn validate(model: &CpModelProto) -> Result<()> {
    let message = call(cpsat_sys::cpsat_validate_model, &model.encode_to_vec())?;
    if !message.is_empty() {
        bail!("invalid model: {message}");
    }
    Ok(())
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
