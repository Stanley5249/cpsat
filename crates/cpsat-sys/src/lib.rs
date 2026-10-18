//! Raw FFI declarations for the CP-SAT bridge.
//!
//! `native/bridge.h` is the contract: it documents each function, the
//! preconditions every call must meet, and buffer ownership. Use the `cpsat`
//! crate for a safe interface.

use core::ffi::c_void;

pub const CPSAT_OK: i32 = 0;
pub const CPSAT_ERROR: i32 = 1;

pub const CPSAT_EVENT_SOLUTION: u32 = 1;
pub const CPSAT_EVENT_BEST_BOUND: u32 = 2;
pub const CPSAT_EVENT_LOG: u32 = 4;

/// Bridge-owned bytes, released with [`cpsat_buffer_free`].
#[repr(C)]
#[derive(Debug)]
pub struct CpsatBuffer {
    pub data: *mut u8,
    pub len: usize,
}

impl CpsatBuffer {
    pub const EMPTY: Self = Self {
        data: core::ptr::null_mut(),
        len: 0,
    };
}

/// One solver event; see `CpsatEvent` in `native/bridge.h`.
#[repr(C)]
#[derive(Debug)]
pub struct CpsatEvent {
    pub kind: u32,
    pub data: *const u8,
    pub len: usize,
    pub bound: f64,
}

/// Event callback; see `CpsatEventFn` in `native/bridge.h`.
pub type CpsatEventFn = unsafe extern "C" fn(context: *mut c_void, event: *const CpsatEvent) -> i32;

/// Opaque native session for one solve.
#[repr(C)]
pub struct CpsatSession {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub fn cpsat_session_new() -> *mut CpsatSession;
    pub fn cpsat_session_free(session: *mut CpsatSession);
    pub fn cpsat_session_stop(session: *mut CpsatSession);
    pub fn cpsat_session_solve(
        session: *mut CpsatSession,
        model: *const u8,
        model_len: usize,
        params: *const u8,
        params_len: usize,
        events: u32,
        on_event: Option<CpsatEventFn>,
        context: *mut c_void,
        response: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_validate_model(
        model: *const u8,
        model_len: usize,
        message: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_model_stats(
        model: *const u8,
        model_len: usize,
        stats: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_response_stats(
        response: *const u8,
        response_len: usize,
        stats: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_buffer_free(buffer: *mut CpsatBuffer);
}
