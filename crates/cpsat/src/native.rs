//! Ownership wrappers for bridge resources.

use anyhow::{Result, bail, ensure};
use cpsat_sys::{CPSAT_OK, CpsatBuffer, CpsatSession};

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

    pub fn text(&self) -> String {
        String::from_utf8_lossy(self.bytes()).into_owned()
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
    if error.bytes().is_empty() {
        bail!("OR-Tools failed without a message");
    }
    bail!("OR-Tools failed: {}", error.text());
}

/// Owns one native session, which serves one solve.
pub struct Session(*mut CpsatSession);

impl Session {
    pub fn new() -> Result<Self> {
        // SAFETY: no preconditions.
        let session = unsafe { cpsat_sys::cpsat_session_new() };
        ensure!(!session.is_null(), "failed to allocate a CP-SAT session");
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
