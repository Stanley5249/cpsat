//! Raw FFI declarations for the CP-SAT bridge.
//!
//! `bridge/bridge.h` is the contract: it documents each function, the
//! preconditions every call must meet, and buffer ownership. Use the `cpsat`
//! crate for a safe interface.

#![expect(
    missing_docs,
    reason = "`bridge/bridge.h` documents each item, and a copy here would drift"
)]

use core::ffi::c_void;

pub const CPSAT_OK: i32 = 0;
pub const CPSAT_ERROR: i32 = 1;

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

/// See `CpsatResponseRef` in `bridge/bridge.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CpsatResponseRef {
    pub status: i32,
    pub solution: *const i64,
    pub solution_len: usize,
    pub objective_value: f64,
    pub best_objective_bound: f64,
    pub num_booleans: i64,
    pub num_conflicts: i64,
    pub num_branches: i64,
    pub num_binary_propagations: i64,
    pub num_integer_propagations: i64,
    pub wall_time: f64,
    pub user_time: f64,
    pub deterministic_time: f64,
    pub response: *const c_void,
}

pub type CpsatSolutionCallback =
    unsafe extern "C" fn(context: *mut c_void, response: *const CpsatResponseRef) -> i32;
pub type CpsatBestBoundCallback = unsafe extern "C" fn(context: *mut c_void, bound: f64) -> i32;
pub type CpsatLogCallback =
    unsafe extern "C" fn(context: *mut c_void, message: *const u8, message_len: usize) -> i32;

/// Opaque bridge session for one solve.
#[repr(C)]
pub struct CpsatSession {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub fn cpsat_session_new() -> *mut CpsatSession;
    pub fn cpsat_session_free(session: *mut CpsatSession);
    pub fn cpsat_session_stop(session: *mut CpsatSession);
    pub fn cpsat_session_set_parameters(
        session: *mut CpsatSession,
        parameters: *const u8,
        parameters_len: usize,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_session_add_solution_callback(
        session: *mut CpsatSession,
        callback: CpsatSolutionCallback,
        context: *mut c_void,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_session_add_best_bound_callback(
        session: *mut CpsatSession,
        callback: CpsatBestBoundCallback,
        context: *mut c_void,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_session_add_log_callback(
        session: *mut CpsatSession,
        callback: CpsatLogCallback,
        context: *mut c_void,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_session_solve(
        session: *mut CpsatSession,
        model: *const u8,
        model_len: usize,
        response: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_response_ref_serialize(
        response_ref: *const CpsatResponseRef,
        response: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
    pub fn cpsat_model_validate(
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
    pub fn cpsat_parameters_default(parameters: *mut CpsatBuffer, error: *mut CpsatBuffer) -> i32;
    pub fn cpsat_buffer_free(buffer: *mut CpsatBuffer);
}
