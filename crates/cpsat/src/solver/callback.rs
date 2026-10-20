//! Solver callbacks and the trampolines that the bridge calls.
//!
//! A [`CpSolver`](crate::prelude::CpSolver) keeps its callbacks in a nested
//! tuple, such as `(((), SolutionCallback<F>), LogCallback<G>)`, so it stays
//! generic over each closure. A boxed closure would keep its borrows alive until
//! the solver drops.

use core::ffi::c_void;
use std::any::Any;
use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};

use buffa::MessageView;

use crate::error::Result;
use crate::proto::view::CpSolverResponseView;
use crate::solver::ffi::Session;

/// A panic payload from a callback.
type Panic = Box<dyn Any + Send>;

/// One callback and the first panic it raised, which the solve resumes after
/// it returns, as Python's binding reraises the first callback exception.
#[derive(Debug)]
struct Slot<F> {
    callback: F,
    panic: Option<Panic>,
}

impl<F> Slot<F> {
    fn new(callback: F) -> Self {
        Self {
            callback,
            panic: None,
        }
    }

    /// Returns nonzero to stop the search.
    fn call(&mut self, run: impl FnOnce(&mut F) -> ControlFlow<()>) -> i32 {
        if self.panic.is_some() {
            return 1;
        }
        match catch_unwind(AssertUnwindSafe(|| run(&mut self.callback))) {
            Ok(ControlFlow::Continue(())) => 0,
            Ok(ControlFlow::Break(())) => 1,
            Err(payload) => {
                self.panic = Some(payload);
                1
            }
        }
    }
}

/// A solution callback in a [`CpSolver`](crate::prelude::CpSolver), added by
/// [`CpSolver::add_solution_callback`](crate::prelude::CpSolver::add_solution_callback).
#[derive(Debug)]
pub struct SolutionCallback<F>(Slot<F>);

/// A best-bound callback in a [`CpSolver`](crate::prelude::CpSolver), added by
/// [`CpSolver::add_best_bound_callback`](crate::prelude::CpSolver::add_best_bound_callback).
#[derive(Debug)]
pub struct BestBoundCallback<F>(Slot<F>);

/// A log callback in a [`CpSolver`](crate::prelude::CpSolver), added by
/// [`CpSolver::add_log_callback`](crate::prelude::CpSolver::add_log_callback).
#[derive(Debug)]
pub struct LogCallback<F>(Slot<F>);

impl<F> SolutionCallback<F> {
    pub(crate) fn new(callback: F) -> Self {
        Self(Slot::new(callback))
    }
}

impl<F> BestBoundCallback<F> {
    pub(crate) fn new(callback: F) -> Self {
        Self(Slot::new(callback))
    }
}

impl<F> LogCallback<F> {
    pub(crate) fn new(callback: F) -> Self {
        Self(Slot::new(callback))
    }
}

mod sealed {
    use super::{Panic, Result, Session};

    pub trait Sealed {
        /// Adds every callback to `session`, in order.
        ///
        /// # Safety
        ///
        /// `self` must neither move nor drop until the solve returns.
        unsafe fn register(&mut self, session: &Session) -> Result<()>;

        /// The first panic, in registration order.
        fn take_panic(&mut self) -> Option<Panic>;
    }

    /// One callback of a known kind.
    pub trait Callback {
        /// # Safety
        ///
        /// As for [`Sealed::register`].
        unsafe fn register(&mut self, session: &Session) -> Result<()>;

        fn take_panic(&mut self) -> Option<Panic>;
    }
}

/// The callbacks of a [`CpSolver`](crate::prelude::CpSolver), which its
/// `add_*` methods build. The trait is sealed.
pub trait Callbacks: sealed::Sealed {}

impl sealed::Sealed for () {
    unsafe fn register(&mut self, _: &Session) -> Result<()> {
        Ok(())
    }

    fn take_panic(&mut self) -> Option<Panic> {
        None
    }
}

impl Callbacks for () {}

impl<C: Callbacks, K: sealed::Callback> sealed::Sealed for (C, K) {
    unsafe fn register(&mut self, session: &Session) -> Result<()> {
        // SAFETY: the caller keeps both parts in place.
        unsafe {
            self.0.register(session)?;
            self.1.register(session)
        }
    }

    fn take_panic(&mut self) -> Option<Panic> {
        let first = self.0.take_panic();
        first.or_else(|| self.1.take_panic())
    }
}

impl<C: Callbacks, K: sealed::Callback> Callbacks for (C, K) {}

impl<F> sealed::Callback for SolutionCallback<F>
where
    F: FnMut(&CpSolverResponseView<'_>) -> ControlFlow<()> + Send,
{
    unsafe fn register(&mut self, session: &Session) -> Result<()> {
        // SAFETY: the trampoline matches the context type, and the caller
        // keeps the context in place.
        unsafe { session.add_solution_callback(Some(solution::<F>), (&raw mut self.0).cast()) }
    }

    fn take_panic(&mut self) -> Option<Panic> {
        self.0.panic.take()
    }
}

impl<F> sealed::Callback for BestBoundCallback<F>
where
    F: FnMut(f64) -> ControlFlow<()> + Send,
{
    unsafe fn register(&mut self, session: &Session) -> Result<()> {
        // SAFETY: as for the solution callback.
        unsafe { session.add_best_bound_callback(Some(best_bound::<F>), (&raw mut self.0).cast()) }
    }

    fn take_panic(&mut self) -> Option<Panic> {
        self.0.panic.take()
    }
}

impl<F> sealed::Callback for LogCallback<F>
where
    F: FnMut(&str) -> ControlFlow<()> + Send,
{
    unsafe fn register(&mut self, session: &Session) -> Result<()> {
        // SAFETY: as for the solution callback.
        unsafe { session.add_log_callback(Some(log::<F>), (&raw mut self.0).cast()) }
    }

    fn take_panic(&mut self) -> Option<Panic> {
        self.0.panic.take()
    }
}

/// Recovers the slot that a trampoline's context points to.
///
/// # Safety
///
/// `context` must point to a live `Slot<F>`, and the bridge's callback mutex
/// makes this the only live reference to it.
unsafe fn slot<'a, F>(context: *mut c_void) -> &'a mut Slot<F> {
    // SAFETY: the caller upholds the contract above.
    unsafe { &mut *context.cast::<Slot<F>>() }
}

unsafe extern "C" fn solution<F>(
    context: *mut c_void,
    response: *const u8,
    response_len: usize,
) -> i32
where
    F: FnMut(&CpSolverResponseView<'_>) -> ControlFlow<()>,
{
    // SAFETY: the bridge passes the context it was given and a response valid
    // for the duration of the call.
    let (slot, response) = unsafe {
        (
            slot::<F>(context),
            crate::solver::ffi::slice(response, response_len),
        )
    };
    slot.call(|callback| {
        let view = CpSolverResponseView::decode_view(response)
            .expect("the bridge sends a serialized CpSolverResponse");
        callback(&view)
    })
}

unsafe extern "C" fn best_bound<F>(context: *mut c_void, bound: f64) -> i32
where
    F: FnMut(f64) -> ControlFlow<()>,
{
    // SAFETY: the bridge passes the context it was given.
    unsafe { slot::<F>(context) }.call(|callback| callback(bound))
}

unsafe extern "C" fn log<F>(context: *mut c_void, message: *const u8, message_len: usize) -> i32
where
    F: FnMut(&str) -> ControlFlow<()>,
{
    // SAFETY: the bridge passes the context it was given and a message valid
    // for the duration of the call.
    let (slot, message) = unsafe {
        (
            slot::<F>(context),
            crate::solver::ffi::slice(message, message_len),
        )
    };
    slot.call(|callback| callback(&String::from_utf8_lossy(message)))
}
