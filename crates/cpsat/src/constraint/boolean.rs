//! Boolean constraints, like Python's `add_bool_or` and its relatives.

use super::{Constraint, Kind, literals};
use crate::expr::BoolVar;
use crate::proto::BoolArgumentProto;

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
    /// At least one literal is true. With no literals it is always false, like
    /// Python's `add(False)`.
    #[doc(alias("add_bool_or", "add_at_least_one", "AddBoolOr", "AddAtLeastOne"))]
    #[must_use]
    pub fn bool_or(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::BoolOr, literals)
    }

    /// Every literal is true. With no literals it is always true, like Python's
    /// `add(True)`.
    #[doc(alias("add_bool_and", "AddBoolAnd"))]
    #[must_use]
    pub fn bool_and(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::BoolAnd, literals)
    }

    /// At most one literal is true.
    #[doc(alias("add_at_most_one", "AddAtMostOne"))]
    #[must_use]
    pub fn at_most_one(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::AtMostOne, literals)
    }

    /// Exactly one literal is true.
    #[doc(alias("add_exactly_one", "AddExactlyOne"))]
    #[must_use]
    pub fn exactly_one(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::ExactlyOne, literals)
    }

    /// An odd number of literals is true.
    #[doc(alias("add_bool_xor", "AddBoolXor"))]
    #[must_use]
    pub fn bool_xor(literals: impl IntoIterator<Item = BoolVar>) -> Self {
        bool_argument(Kind::BoolXor, literals)
    }

    /// `a` implies `b`, which writes `b` enforced by `a`.
    #[doc(alias("add_implication", "AddImplication"))]
    #[must_use]
    pub fn implication(a: BoolVar, b: BoolVar) -> Self {
        Self::bool_and([b]).only_enforce_if(a)
    }
}
