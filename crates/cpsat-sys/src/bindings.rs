pub const CPSAT_OK: i32 = 0;
pub const CPSAT_ERROR: i32 = 1;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CpsatBuffer {
    pub data: *mut u8,
    pub len: usize,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of CpsatBuffer"][::core::mem::size_of::<CpsatBuffer>() - 16usize];
    ["Alignment of CpsatBuffer"][::core::mem::align_of::<CpsatBuffer>() - 8usize];
    ["Offset of field: CpsatBuffer::data"][::core::mem::offset_of!(CpsatBuffer, data) - 0usize];
    ["Offset of field: CpsatBuffer::len"][::core::mem::offset_of!(CpsatBuffer, len) - 8usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
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
    pub response: *const ::core::ffi::c_void,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of CpsatResponseRef"][::core::mem::size_of::<CpsatResponseRef>() - 112usize];
    ["Alignment of CpsatResponseRef"][::core::mem::align_of::<CpsatResponseRef>() - 8usize];
    ["Offset of field: CpsatResponseRef::status"]
        [::core::mem::offset_of!(CpsatResponseRef, status) - 0usize];
    ["Offset of field: CpsatResponseRef::solution"]
        [::core::mem::offset_of!(CpsatResponseRef, solution) - 8usize];
    ["Offset of field: CpsatResponseRef::solution_len"]
        [::core::mem::offset_of!(CpsatResponseRef, solution_len) - 16usize];
    ["Offset of field: CpsatResponseRef::objective_value"]
        [::core::mem::offset_of!(CpsatResponseRef, objective_value) - 24usize];
    ["Offset of field: CpsatResponseRef::best_objective_bound"]
        [::core::mem::offset_of!(CpsatResponseRef, best_objective_bound) - 32usize];
    ["Offset of field: CpsatResponseRef::num_booleans"]
        [::core::mem::offset_of!(CpsatResponseRef, num_booleans) - 40usize];
    ["Offset of field: CpsatResponseRef::num_conflicts"]
        [::core::mem::offset_of!(CpsatResponseRef, num_conflicts) - 48usize];
    ["Offset of field: CpsatResponseRef::num_branches"]
        [::core::mem::offset_of!(CpsatResponseRef, num_branches) - 56usize];
    ["Offset of field: CpsatResponseRef::num_binary_propagations"]
        [::core::mem::offset_of!(CpsatResponseRef, num_binary_propagations) - 64usize];
    ["Offset of field: CpsatResponseRef::num_integer_propagations"]
        [::core::mem::offset_of!(CpsatResponseRef, num_integer_propagations) - 72usize];
    ["Offset of field: CpsatResponseRef::wall_time"]
        [::core::mem::offset_of!(CpsatResponseRef, wall_time) - 80usize];
    ["Offset of field: CpsatResponseRef::user_time"]
        [::core::mem::offset_of!(CpsatResponseRef, user_time) - 88usize];
    ["Offset of field: CpsatResponseRef::deterministic_time"]
        [::core::mem::offset_of!(CpsatResponseRef, deterministic_time) - 96usize];
    ["Offset of field: CpsatResponseRef::response"]
        [::core::mem::offset_of!(CpsatResponseRef, response) - 104usize];
};
pub type CpsatSolutionCallback = ::core::option::Option<
    unsafe extern "C" fn(
        context: *mut ::core::ffi::c_void,
        response: *const CpsatResponseRef,
    ) -> i32,
>;
pub type CpsatBestBoundCallback = ::core::option::Option<
    unsafe extern "C" fn(context: *mut ::core::ffi::c_void, bound: f64) -> i32,
>;
pub type CpsatLogCallback = ::core::option::Option<
    unsafe extern "C" fn(
        context: *mut ::core::ffi::c_void,
        message: *const u8,
        message_len: usize,
    ) -> i32,
>;
#[repr(C)]
#[derive(Debug)]
pub struct CpsatSession {
    _unused: [u8; 0],
}
unsafe extern "C" {
    pub fn cpsat_session_new() -> *mut CpsatSession;
}
unsafe extern "C" {
    pub fn cpsat_session_free(session: *mut CpsatSession);
}
unsafe extern "C" {
    pub fn cpsat_session_stop(session: *mut CpsatSession);
}
unsafe extern "C" {
    pub fn cpsat_session_set_parameters(
        session: *mut CpsatSession,
        parameters: *const u8,
        parameters_len: usize,
        error: *mut CpsatBuffer,
    ) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_session_add_solution_callback(
        session: *mut CpsatSession,
        callback: CpsatSolutionCallback,
        context: *mut ::core::ffi::c_void,
        error: *mut CpsatBuffer,
    ) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_session_add_best_bound_callback(
        session: *mut CpsatSession,
        callback: CpsatBestBoundCallback,
        context: *mut ::core::ffi::c_void,
        error: *mut CpsatBuffer,
    ) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_session_add_log_callback(
        session: *mut CpsatSession,
        callback: CpsatLogCallback,
        context: *mut ::core::ffi::c_void,
        error: *mut CpsatBuffer,
    ) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_session_solve(
        session: *mut CpsatSession,
        model: *const u8,
        model_len: usize,
        response: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_response_ref_serialize(
        response_ref: *const CpsatResponseRef,
        response: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_model_validate(
        model: *const u8,
        model_len: usize,
        message: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_model_stats(
        model: *const u8,
        model_len: usize,
        stats: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_response_stats(
        response: *const u8,
        response_len: usize,
        stats: *mut CpsatBuffer,
        error: *mut CpsatBuffer,
    ) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_parameters_default(parameters: *mut CpsatBuffer, error: *mut CpsatBuffer) -> i32;
}
unsafe extern "C" {
    pub fn cpsat_buffer_free(buffer: *mut CpsatBuffer);
}
