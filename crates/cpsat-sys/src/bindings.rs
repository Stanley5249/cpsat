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
pub type CpsatSolutionCallback = ::core::option::Option<
    unsafe extern "C" fn(
        context: *mut ::core::ffi::c_void,
        response: *const u8,
        response_len: usize,
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
