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

/// Turns a bridge return code into a result, using the bridge's message.
pub fn check(code: i32, error: &Buffer) -> Result<()> {
    if code == CPSAT_OK {
        return Ok(());
    }
    let message = match error.text() {
        text if text.is_empty() => "no message".into(),
        text => text.into_owned(),
    };
    Err(Error::OrTools(message))
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

    pub fn set_parameters(&self, parameters: &[u8]) -> Result<()> {
        let mut error = Buffer::new();
        // SAFETY: the session is alive, and the bytes outlive the call.
        let code = unsafe {
            cpsat_sys::cpsat_session_set_parameters(
                self.0,
                parameters.as_ptr(),
                parameters.len(),
                error.as_mut_ptr(),
            )
        };
        check(code, &error)
    }

    /// # Safety
    ///
    /// `context` must suit `callback` and stay valid until [`Session::solve`]
    /// returns.
    pub unsafe fn add_solution_callback(
        &self,
        callback: CpsatSolutionCallback,
        context: *mut c_void,
    ) -> Result<()> {
        let mut error = Buffer::new();
        // SAFETY: the session is alive, and the caller upholds the rest.
        let code = unsafe {
            cpsat_sys::cpsat_session_add_solution_callback(
                self.0,
                callback,
                context,
                error.as_mut_ptr(),
            )
        };
        check(code, &error)
    }

    /// # Safety
    ///
    /// As for [`Session::add_solution_callback`].
    pub unsafe fn add_best_bound_callback(
        &self,
        callback: CpsatBestBoundCallback,
        context: *mut c_void,
    ) -> Result<()> {
        let mut error = Buffer::new();
        // SAFETY: the session is alive, and the caller upholds the rest.
        let code = unsafe {
            cpsat_sys::cpsat_session_add_best_bound_callback(
                self.0,
                callback,
                context,
                error.as_mut_ptr(),
            )
        };
        check(code, &error)
    }

    /// # Safety
    ///
    /// As for [`Session::add_solution_callback`].
    pub unsafe fn add_log_callback(
        &self,
        callback: CpsatLogCallback,
        context: *mut c_void,
    ) -> Result<()> {
        let mut error = Buffer::new();
        // SAFETY: the session is alive, and the caller upholds the rest.
        let code = unsafe {
            cpsat_sys::cpsat_session_add_log_callback(self.0, callback, context, error.as_mut_ptr())
        };
        check(code, &error)
    }

    /// Solves `model` and returns the encoded response.
    pub fn solve(&self, model: &[u8]) -> Result<Buffer> {
        let mut response = Buffer::new();
        let mut error = Buffer::new();
        // SAFETY: the session is alive, the bytes outlive the call, and the
        // buffers are empty bridge buffers.
        let code = unsafe {
            cpsat_sys::cpsat_session_solve(
                self.0,
                model.as_ptr(),
                model.len(),
                response.as_mut_ptr(),
                error.as_mut_ptr(),
            )
        };
        check(code, &error)?;
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
