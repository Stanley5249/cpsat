//! Boolean constraints, like Python's `add_bool_or` and its relatives.

use super::{Constraint, Kind, literals};
use crate::proto::BoolArgumentProto;
use crate::variable::BoolVar;

fn bool_argument(
    kind: fn(Box<BoolArgumentProto>) -> Kind,
    literals: impl IntoIterator<Item = BoolVar>,
) -> Constraint {
    Constraint::of(kind(Box::new(BoolArgumentProto {
        literals: self::literals(literals),
        ..Default::default()
    })))
}

impl Constraint {
    /// At least one literal is true, like Python's `add_bool_or` and
    /// `add_at_least_one`. With no literals it is always false, like
    /// Python's `add(False)`.
    #[must_use]
    pub fn bool_or(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::BoolOr, literals)
    }

    /// Every literal is true. With no literals it is always true, like
    /// Python's `add(True)`.
    #[must_use]
    pub fn bool_and(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::BoolAnd, literals)
    }

    #[must_use]
    pub fn at_most_one(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::AtMostOne, literals)
    }

    #[must_use]
    pub fn exactly_one(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::ExactlyOne, literals)
    }

    /// An odd number of literals is true.
    #[must_use]
    pub fn bool_xor(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::BoolXor, literals)
    }

    /// `a` implies `b`, which Python writes as `b` enforced by `a`.
    #[must_use]
    pub fn implication(a: BoolVar, b: BoolVar) -> Self {
        Self::bool_and([b]).only_if(a)
    }
}
