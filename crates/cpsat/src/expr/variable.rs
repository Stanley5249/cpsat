//! Typed handles to the variables and intervals of a [`Model`](crate::prelude::Model).
//!
//! A handle is an index into the model's Protobuf message, so it is `Copy` and
//! does not borrow the model. The handle types keep integer variables,
//! Boolean literals, and intervals apart, but a handle does not know which
//! model created it, like the indices of `petgraph` and `slotmap`.

use std::ops::Not;

/// An integer variable, an index into `CpModelProto::variables`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntVar(pub(crate) i32);

impl IntVar {
    /// The index into `CpModelProto::variables`.
    #[must_use]
    pub const fn index(self) -> i32 {
        self.0
    }
}

/// A Boolean literal: a 0-1 variable or its negation, which `!` returns.
///
/// The schema writes the negation of variable `i` as `-i - 1`, which is what
/// [`index`](Self::index) returns for a negated literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoolVar(pub(crate) i32);

impl BoolVar {
    /// The literal as the schema writes it, negative when negated.
    #[must_use]
    pub const fn index(self) -> i32 {
        self.0
    }

    /// The underlying 0-1 variable, without the negation.
    #[must_use]
    pub const fn variable(self) -> IntVar {
        if self.is_negated() {
            IntVar(!self.0)
        } else {
            IntVar(self.0)
        }
    }

    #[must_use]
    pub const fn is_negated(self) -> bool {
        self.0 < 0
    }
}

impl Not for BoolVar {
    type Output = Self;

    /// Bitwise not equals `-index - 1` and never overflows.
    fn not(self) -> Self {
        Self(!self.0)
    }
}

/// An interval, an index into `CpModelProto::constraints`, because the schema
/// stores an interval as a constraint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntervalVar(pub(crate) i32);

impl IntervalVar {
    /// The index into `CpModelProto::constraints`.
    #[must_use]
    pub const fn index(self) -> i32 {
        self.0
    }
}
