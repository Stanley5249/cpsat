//! Owners of bridge resources, the safe side of the `cpsat-sys` FFI.

use std::borrow::Cow;

use cpsat_sys::{CPSAT_OK, CpsatBuffer, CpsatSession};

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
        if self.0.data.is_null() {
            return &[];
        }
        // SAFETY: the bridge filled data with len bytes it owns until drop.
        unsafe { core::slice::from_raw_parts(self.0.data, self.0.len) }
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
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: the session came from cpsat_session_new, and every
        // StopHandle detaches before the session drops.
        unsafe { cpsat_sys::cpsat_session_free(self.0) };
    }
}
