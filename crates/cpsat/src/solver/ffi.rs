//! The only caller of `cpsat-sys`, with owners of bridge resources and a
//! safe wrapper per bridge function.
//!
//! The file reads in three parts: the buffer and call primitives, the
//! stateless helpers, and the session that serves one solve.

use core::ffi::c_void;
use std::borrow::Cow;
use std::sync::{Arc, Weak};

use cpsat_sys::{
    CPSAT_OK, CpsatBestBoundCallback, CpsatBuffer, CpsatLogCallback, CpsatSession,
    CpsatSolutionCallback,
};

use crate::error::{Error, Result};

/// Owns a bridge-allocated buffer and frees it on drop.
pub struct Buffer(CpsatBuffer);

impl Buffer {
    /// An empty buffer for the bridge to fill.
    pub fn new() -> Self {
        Self(CpsatBuffer::EMPTY)
    }

    /// The pointer that a bridge function writes through.
    pub fn as_mut_ptr(&mut self) -> *mut CpsatBuffer {
        &raw mut self.0
    }

    /// The bytes the bridge wrote.
    pub fn bytes(&self) -> &[u8] {
        // SAFETY: the bridge filled data with len bytes it owns until drop.
        unsafe { slice(self.0.data, self.0.len) }
    }

    /// The bytes as text, replacing invalid UTF-8.
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
pub struct Session(Arc<Raw>);

impl Session {
    /// Allocates a session.
    pub fn new() -> Result<Self> {
        // SAFETY: no preconditions.
        let session = unsafe { cpsat_sys::cpsat_session_new() };
        if session.is_null() {
            return Err(Error::OrTools("failed to allocate a session".into()));
        }
        Ok(Self(Arc::new(Raw(session))))
    }

    /// A handle that stops this session from any thread, and does nothing
    /// once the session is gone.
    pub fn stopper(&self) -> Stopper {
        Stopper(Arc::downgrade(&self.0))
    }

    /// Sends the parameters for the solve.
    pub fn set_parameters(&mut self, parameters: &[u8]) -> Result<()> {
        // SAFETY: the session is alive, and the bytes outlive the call.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_set_parameters(
                self.0.0,
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
        &mut self,
        callback: CpsatSolutionCallback,
        context: *mut c_void,
    ) -> Result<()> {
        // SAFETY: the session is alive, and the caller upholds the rest.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_add_solution_callback(self.0.0, callback, context, error)
        })
    }

    /// Adds a best-bound callback with its context.
    ///
    /// # Safety
    ///
    /// As for [`Session::add_solution_callback`].
    pub unsafe fn add_best_bound_callback(
        &mut self,
        callback: CpsatBestBoundCallback,
        context: *mut c_void,
    ) -> Result<()> {
        // SAFETY: the session is alive, and the caller upholds the rest.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_add_best_bound_callback(self.0.0, callback, context, error)
        })
    }

    /// Adds a log callback with its context.
    ///
    /// # Safety
    ///
    /// As for [`Session::add_solution_callback`].
    pub unsafe fn add_log_callback(
        &mut self,
        callback: CpsatLogCallback,
        context: *mut c_void,
    ) -> Result<()> {
        // SAFETY: the session is alive, and the caller upholds the rest.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_add_log_callback(self.0.0, callback, context, error)
        })
    }

    /// Solves `model` and returns the encoded response.
    pub fn solve(&mut self, model: &[u8]) -> Result<Buffer> {
        let mut response = Buffer::new();
        // SAFETY: the session is alive, the bytes outlive the call, and the
        // buffers are empty bridge buffers.
        call(|error| unsafe {
            cpsat_sys::cpsat_session_solve(
                self.0.0,
                model.as_ptr(),
                model.len(),
                response.as_mut_ptr(),
                error,
            )
        })?;
        Ok(response)
    }
}

/// Stops a [`Session`] from any thread while the session lives.
#[derive(Debug)]
pub struct Stopper(Weak<Raw>);

impl Stopper {
    /// Stops the session's solve, or the next solve if none runs, and does
    /// nothing once the session is gone.
    pub fn stop(&self) {
        if let Some(session) = self.0.upgrade() {
            // SAFETY: the strong reference keeps the session alive, and the
            // bridge's stop is thread-safe.
            unsafe { cpsat_sys::cpsat_session_stop(session.0) };
        }
    }

    /// Whether the session still lives.
    pub fn is_live(&self) -> bool {
        self.0.strong_count() > 0
    }
}

/// The bridge session that a [`Session`] and its [`Stopper`]s share. The last
/// strong reference frees it.
struct Raw(*mut CpsatSession);

// SAFETY: the bridge allows calls on a session from any thread. Shared
// references from other threads only stop the session, which the bridge
// allows during any other call, and `Arc` runs the free after every call.
unsafe impl Send for Raw {}
// SAFETY: as for `Send`.
unsafe impl Sync for Raw {}

impl Drop for Raw {
    fn drop(&mut self) {
        // SAFETY: the session came from cpsat_session_new, and the last
        // reference is dropping, so no other call can run.
        unsafe { cpsat_sys::cpsat_session_free(self.0) };
    }
}
