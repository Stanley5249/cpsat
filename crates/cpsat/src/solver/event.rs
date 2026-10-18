use std::ops::{BitOr, BitOrAssign};

use cpsat_sys::{CPSAT_EVENT_BEST_BOUND, CPSAT_EVENT_LOG, CPSAT_EVENT_SOLUTION};

use crate::proto::view::CpSolverResponseView;

/// The set of events a solve reports to its callback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EventSet(u32);

impl EventSet {
    pub const NONE: Self = Self(0);
    pub const SOLUTION: Self = Self(CPSAT_EVENT_SOLUTION);
    pub const BEST_BOUND: Self = Self(CPSAT_EVENT_BEST_BOUND);
    /// Log lines arrive only when `log_search_progress` is set. Set
    /// `log_to_stdout` to false to avoid printing them twice.
    pub const LOG: Self = Self(CPSAT_EVENT_LOG);
    pub const ALL: Self = Self(CPSAT_EVENT_SOLUTION | CPSAT_EVENT_BEST_BOUND | CPSAT_EVENT_LOG);

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }
}

impl BitOr for EventSet {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOrAssign for EventSet {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// One solver event. Borrowed data is valid only during the callback.
#[derive(Debug)]
#[non_exhaustive]
pub enum Event<'a> {
    /// A new solution, as a zero-copy view of the intermediate response. Call
    /// [`buffa::MessageView::to_owned_message`] to keep it.
    Solution(&'a CpSolverResponseView<'a>),
    /// A new best objective bound.
    BestBound(f64),
    /// One log message, decoded lossily as UTF-8.
    Log(&'a str),
}
