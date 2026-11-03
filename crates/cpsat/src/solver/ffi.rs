//! The only caller of `cpsat-sys`, with owners of bridge resources and a
//! safe wrapper per bridge function.
//!
//! The file reads in four parts: the buffer and call primitives, the
//! stateless helpers, the session that serves one solve, and the callback
//! trampolines that the bridge calls during that solve.

use core::ffi::c_void;
use std::any::Any;
use std::borrow::Cow;
use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use buffa::MessageView;
use cpsat_sys::{CPSAT_OK, CpsatBuffer, CpsatSession};

use crate::error::{Error, Result};
use crate::proto::view::CpSolverResponseView;

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
unsafe fn slice<'a, T>(data: *const T, len: usize) -> &'a [T] {
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

    /// Solves `model` with `callbacks` and returns the encoded response. A
    /// session serves one solve, so this consumes it.
    ///
    /// The first callback panic stops the search, skips every later callback
    /// call, and resumes once the solve returns.
    pub fn solve(self, model: &[u8], callbacks: &mut Callbacks<'_>) -> Result<Buffer> {
        let panic = Mutex::new(None);
        let mut solution = contexts(&mut callbacks.solution, &panic);
        let mut best_bound = contexts(&mut callbacks.best_bound, &panic);
        let mut log = contexts(&mut callbacks.log, &panic);
        // SAFETY: each trampoline reads the context type it is paired with,
        // and the contexts stay in place until this function returns, after
        // the solve, the only bridge call that runs callbacks.
        unsafe {
            self.register(
                cpsat_sys::cpsat_session_add_solution_callback,
                Some(solution_trampoline),
                &mut solution,
            )?;
            self.register(
                cpsat_sys::cpsat_session_add_best_bound_callback,
                Some(best_bound_trampoline),
                &mut best_bound,
            )?;
            self.register(
                cpsat_sys::cpsat_session_add_log_callback,
                Some(log_trampoline),
                &mut log,
            )?;
        }
        let mut response = Buffer::new();
        // SAFETY: the session is alive, the bytes outlive the call, and the
        // buffers are empty bridge buffers.
        let solved = call(|error| unsafe {
            cpsat_sys::cpsat_session_solve(
                self.0.0,
                model.as_ptr(),
                model.len(),
                response.as_mut_ptr(),
                error,
            )
        });
        if let Some(payload) = panic.into_inner().unwrap_or_else(PoisonError::into_inner) {
            resume_unwind(payload);
        }
        solved.map(|()| response)
    }

    /// Adds one bridge callback per context, in order.
    ///
    /// # Safety
    ///
    /// `trampoline` must read its context as a `Context<C>`, and `contexts`
    /// must stay in place until the solve returns.
    unsafe fn register<C: ?Sized, T: Copy>(
        &self,
        add: unsafe extern "C" fn(*mut CpsatSession, T, *mut c_void, *mut CpsatBuffer) -> i32,
        trampoline: T,
        contexts: &mut [Context<'_, C>],
    ) -> Result<()> {
        for context in contexts {
            // SAFETY: the session is alive, and the caller upholds the rest.
            call(|error| unsafe { add(self.0.0, trampoline, (&raw mut *context).cast(), error) })?;
        }
        Ok(())
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

/// A solution callback, which reads a zero-copy view of each solution.
pub type SolutionCallback<'a> = dyn FnMut(&CpSolverResponseView<'_>) -> ControlFlow<()> + Send + 'a;

/// A best-bound callback, which reads each improved objective bound.
pub type BestBoundCallback<'a> = dyn FnMut(f64) -> ControlFlow<()> + Send + 'a;

/// A log callback, which reads each log message as text.
pub type LogCallback<'a> = dyn FnMut(&str) -> ControlFlow<()> + Send + 'a;

/// The borrowed callbacks of a solve, one list per bridge callback kind, each
/// in the order the bridge calls them. A plain reference has no drop glue, so
/// each borrow ends at its last use.
#[derive(Default)]
pub struct Callbacks<'a> {
    pub solution: Vec<&'a mut SolutionCallback<'a>>,
    pub best_bound: Vec<&'a mut BestBoundCallback<'a>>,
    pub log: Vec<&'a mut LogCallback<'a>>,
}

/// The first callback panic of a solve.
type Panic = Mutex<Option<Box<dyn Any + Send>>>;

/// What a trampoline's context points to: one callback and the panic record
/// that every callback of the solve shares.
struct Context<'a, C: ?Sized> {
    callback: &'a mut C,
    panic: &'a Panic,
}

/// One context per callback, all sharing `panic`.
fn contexts<'a, C: ?Sized>(callbacks: &'a mut [&mut C], panic: &'a Panic) -> Vec<Context<'a, C>> {
    callbacks
        .iter_mut()
        .map(|callback| Context {
            callback: &mut **callback,
            panic,
        })
        .collect()
}

impl<C: ?Sized> Context<'_, C> {
    /// Recovers the context that a trampoline receives.
    ///
    /// # Safety
    ///
    /// `context` must come from [`Session::register`] with the same `C`, and
    /// the bridge's callback mutex must make this the only live reference.
    unsafe fn from_raw<'a>(context: *mut c_void) -> &'a mut Self {
        // SAFETY: the caller upholds the contract above.
        unsafe { &mut *context.cast::<Self>() }
    }

    /// Runs `call` on the callback unless a callback of the solve already
    /// panicked, and returns nonzero to stop the search.
    fn run(&mut self, call: impl FnOnce(&mut C) -> ControlFlow<()>) -> i32 {
        // Nothing panics while holding the lock, so a poisoned record is intact.
        let lock = || self.panic.lock().unwrap_or_else(PoisonError::into_inner);
        if lock().is_some() {
            return 1;
        }
        match catch_unwind(AssertUnwindSafe(|| call(&mut *self.callback))) {
            Ok(ControlFlow::Continue(())) => 0,
            Ok(ControlFlow::Break(())) => 1,
            Err(payload) => {
                lock().get_or_insert(payload);
                1
            }
        }
    }
}

unsafe extern "C" fn solution_trampoline(
    context: *mut c_void,
    response: *const u8,
    response_len: usize,
) -> i32 {
    // SAFETY: the bridge passes the context it was given and a response valid
    // for the duration of the call.
    let (context, response) = unsafe {
        (
            Context::<SolutionCallback<'_>>::from_raw(context),
            slice(response, response_len),
        )
    };
    context.run(|callback| {
        let view = CpSolverResponseView::decode_view(response)
            .expect("the bridge sends a serialized CpSolverResponse");
        callback(&view)
    })
}

unsafe extern "C" fn best_bound_trampoline(context: *mut c_void, bound: f64) -> i32 {
    // SAFETY: the bridge passes the context it was given.
    let context = unsafe { Context::<BestBoundCallback<'_>>::from_raw(context) };
    context.run(|callback| callback(bound))
}

unsafe extern "C" fn log_trampoline(
    context: *mut c_void,
    message: *const u8,
    message_len: usize,
) -> i32 {
    // SAFETY: the bridge passes the context it was given and a message valid
    // for the duration of the call.
    let (context, message) = unsafe {
        (
            Context::<LogCallback<'_>>::from_raw(context),
            slice(message, message_len),
        )
    };
    context.run(|callback| callback(&String::from_utf8_lossy(message)))
}
