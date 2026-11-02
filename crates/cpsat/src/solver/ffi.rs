//! Owners of bridge resources, the safe side of the `cpsat-sys` FFI.

use core::ffi::c_void;
use std::borrow::Cow;

use cpsat_sys::{
    CPSAT_OK, CpsatBestBoundCallback, CpsatBuffer, CpsatLogCallback, CpsatSession,
    CpsatSolutionCallback,
};

use crate::error::{Error, Result};

/// Owns a bridge-allocated buffer and frees it on drop.
pub struct Buffer(CpsatBuffer);

impl Buffer {
    pub fn new() -> Self {
        Self(CpsatBuffer::EMPTY)
    }

    pub fn as_mut_ptr(&mut self) -> *mut CpsatBuffer {
        &raw mut self.0
    }

    pub fn bytes(&self) -> &[u8] {
        // SAFETY: the bridge filled data with len bytes it owns until drop.
        unsafe { slice(self.0.data, self.0.len) }
    }

    pub fn text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.bytes())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: the buffer is empty or was filled by the bridge.
        unsafe { cpsat_sys::cpsat_buffer_free(&raw mut self.0) };
    }
}

/// Borrows a bridge array, which the bridge passes as null when empty.
///
/// # Safety
///
/// `data` must be null or point to `len` values valid for `'a`.
pub unsafe fn slice<'a, T>(data: *const T, len: usize) -> &'a [T] {
    if data.is_null() {
        return &[];
    }
    // SAFETY: the caller upholds the contract above.
    unsafe { core::slice::from_raw_parts(data, len) }
}

/// Runs a bridge function with an empty error buffer, and turns its return
/// code into a result with the bridge's message.
pub fn call(function: impl FnOnce(*mut CpsatBuffer) -> i32) -> Result<()> {
    let mut error = Buffer::new();
    if function(error.as_mut_ptr()) == CPSAT_OK {
        return Ok(());
    }
    let message = match error.text() {
        text if text.is_empty() => "no message".into(),
        text => text.into_owned(),
    };
    Err(Error::OrTools(message))
}

/// Runs a bridge helper that reads `bytes` and writes text.
fn call_helper(
    helper: unsafe extern "C" fn(*const u8, usize, *mut CpsatBuffer, *mut CpsatBuffer) -> i32,
    bytes: &[u8],
) -> Result<String> {
    let mut output = Buffer::new();
    // SAFETY: bytes is valid for its length, and both buffers are empty
    // bridge buffers.
    call(|error| unsafe { helper(bytes.as_ptr(), bytes.len(), output.as_mut_ptr(), error) })?;
    Ok(output.text().into_owned())
}

/// OR-Tools' validation message for an encoded `CpModelProto`, which is
/// empty for a valid model.
pub fn validate(model: &[u8]) -> Result<String> {
    call_helper(cpsat_sys::cpsat_model_validate, model)
}

/// OR-Tools' statistics for an encoded `CpModelProto`.
pub fn model_stats(model: &[u8]) -> Result<String> {
    call_helper(cpsat_sys::cpsat_model_stats, model)
}

/// OR-Tools' statistics for an encoded `CpSolverResponse`.
pub fn response_stats(response: &[u8]) -> Result<String> {
    call_helper(cpsat_sys::cpsat_response_stats, response)
}

/// The encoded `SatParameters` with every singular field set to its default.
pub fn default_parameters() -> Result<Buffer> {
    let mut parameters = Buffer::new();
    // SAFETY: both buffers are empty bridge buffers.
    call(|error| unsafe { cpsat_sys::cpsat_parameters_default(parameters.as_mut_ptr(), error) })?;
    Ok(parameters)
}

/// Owns one bridge session, which serves one solve.
pub struct Session(*mut CpsatSession);

impl Session {
    pub fn new() -> Result<Self> {
        // SAFETY: no preconditions.
        let session = unsafe { cpsat_sys::cpsat_session_new() };
        if session.is_null() {
            return Err(Error::OrTools("failed to allocate a session".into()));
        }
        Ok(Self(session))
    }

    pub fn as_ptr(&self) -> *mut CpsatSession {
        self.0
    }

    /// Stops the solve, from any thread, or the next solve if none runs.
    pub fn stop(&self) {
        // SAFETY: the session is alive, and the bridge's stop is thread-safe.
        unsafe { cpsat_sys::cpsat_session_stop(self.0) };
    }

    /// Sends the parameters for the solve.
    pub fn set_parameters(&self, parameters: &[u8]) -> Result<()> {
        // SAFETY: the session is alive, and the bytes outlive the call.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_set_parameters(
                self.0,
                parameters.as_ptr(),
                parameters.len(),
                error,
            )
        })
    }

    /// Adds a solution callback with its context.
    ///
    /// # Safety
    ///
    /// `context` must suit `callback` and stay valid until [`Session::solve`]
    /// returns.
    pub unsafe fn add_solution_callback(
        &self,
        callback: CpsatSolutionCallback,
        context: *mut c_void,
    ) -> Result<()> {
        // SAFETY: the session is alive, and the caller upholds the rest.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_add_solution_callback(self.0, callback, context, error)
        })
    }

    /// Adds a best-bound callback with its context.
    ///
    /// # Safety
    ///
    /// As for [`Session::add_solution_callback`].
    pub unsafe fn add_best_bound_callback(
        &self,
        callback: CpsatBestBoundCallback,
        context: *mut c_void,
    ) -> Result<()> {
        // SAFETY: the session is alive, and the caller upholds the rest.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_add_best_bound_callback(self.0, callback, context, error)
        })
    }

    /// Adds a log callback with its context.
    ///
    /// # Safety
    ///
    /// As for [`Session::add_solution_callback`].
    pub unsafe fn add_log_callback(
        &self,
        callback: CpsatLogCallback,
        context: *mut c_void,
    ) -> Result<()> {
        // SAFETY: the session is alive, and the caller upholds the rest.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_add_log_callback(self.0, callback, context, error)
        })
    }

    /// Solves `model` and returns the encoded response.
    pub fn solve(&self, model: &[u8]) -> Result<Buffer> {
        let mut response = Buffer::new();
        // SAFETY: the session is alive, the bytes outlive the call, and the
        // buffers are empty bridge buffers.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_solve(
                self.0,
                model.as_ptr(),
                model.len(),
                response.as_mut_ptr(),
                error,
            )
        })?;
        Ok(response)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: the session came from cpsat_session_new, and every
        // StopHandle detaches before the session drops.
        unsafe { cpsat_sys::cpsat_session_free(self.0) };
    }
}
