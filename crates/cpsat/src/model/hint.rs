//! Solution hints, like Python's `add_hint`.

use buffa::MessageField;

use super::{CpModel, sealed};
use crate::expr::{BoolVar, IntVar};

/// What `add_hint` accepts, and the type of its value.
pub trait Hint: sealed::Sealed {
    /// The type of the hinted value: `i64` or `bool`.
    type Value;

    /// The variable index and the value the schema stores.
    #[doc(hidden)]
    fn hint(self, value: Self::Value) -> (i32, i64);
}

impl sealed::Sealed for IntVar {}
impl Hint for IntVar {
    type Value = i64;

    fn hint(self, value: i64) -> (i32, i64) {
        (self.index(), value)
    }
}

impl sealed::Sealed for BoolVar {}
impl Hint for BoolVar {
    type Value = bool;

    /// A negated literal hints its variable with the opposite value.
    fn hint(self, value: bool) -> (i32, i64) {
        (
            self.to_int_var().index(),
            (value != self.is_negated()).into(),
        )
    }
}

impl CpModel {
    /// Hints that `var` has `value`: an `i64` for an integer variable and a
    /// `bool` for a literal.
    #[doc(alias("AddHint"))]
    pub fn add_hint<H: Hint>(&mut self, var: H, value: H::Value) {
        let (var, value) = var.hint(value);
        let hint = self.0.solution_hint.get_or_insert_default();
        hint.vars.push(var);
        hint.values.push(value);
    }

    /// Removes every hint.
    #[doc(alias("ClearHints"))]
    pub fn clear_hints(&mut self) {
        self.0.solution_hint = MessageField::none();
    }
}
