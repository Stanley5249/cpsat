//! Variable creation through [`CpModel::new_var`], like Python's `new_int_var`
//! and `new_bool_var`.

use std::ops::{Range, RangeInclusive};

use super::{CpModel, sealed};
use crate::expr::{BoolVar, Domain, IntVar};

/// What [`CpModel::new_var`] creates, and the handle it returns: an
/// [`IntVar`] for a [`Domain`] or a bounded range, a [`BoolVar`] for
/// [`Bool`], and an [`IntervalVar`](crate::prelude::IntervalVar) for an
/// [`Interval`](crate::prelude::Interval).
///
/// `RangeFrom` and `RangeToInclusive` are missing on purpose, because a
/// variable whose domain reaches `i64::MIN` or `i64::MAX` is invalid.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot create a variable",
    label = "expected a bounded `i64` domain, `Bool`, or an `Interval`",
    note = "a variable needs finite `i64` bounds, such as `0..=10`"
)]
pub trait NewVar: sealed::Sealed {
    /// The handle that [`CpModel::new_var`] returns.
    type Output;

    /// Appends the variable and returns its handle.
    #[doc(hidden)]
    fn insert(self, model: &mut CpModel) -> Self::Output;
}

/// A 0-1 variable for [`CpModel::new_var`].
///
/// ```
/// use cpsat::prelude::*;
///
/// // From `bool_or_sample_sat.py`.
/// let mut model = CpModel::new();
/// let x = model.new_var(Bool.with_name("x"));
/// let y = model.new_var(Bool.with_name("y"));
/// model.add(Constraint::bool_or([x, !y]));
/// ```
#[doc(alias("new_bool_var", "NewBoolVar"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bool;

/// A variable and its name, which [`WithName::with_name`] pairs.
#[derive(Clone, Debug, PartialEq)]
pub struct Named<T> {
    var: T,
    name: String,
}

/// Names a variable before [`CpModel::new_var`] inserts it, like the `name`
/// argument of Python's `new_int_var` and `new_bool_var`.
pub trait WithName: sealed::Sealed + Sized {
    /// Attaches `name`, which the model stores with the variable.
    #[doc(alias("WithName"))]
    #[must_use]
    fn with_name(self, name: impl Into<String>) -> Named<Self> {
        Named {
            var: self,
            name: name.into(),
        }
    }
}

impl<T> sealed::Sealed for Named<T> {}

macro_rules! domain_var {
    ($($ty:ty),*) => {$(
        impl sealed::Sealed for $ty {}
        impl WithName for $ty {}

        impl NewVar for $ty {
            type Output = IntVar;

            fn insert(self, model: &mut CpModel) -> IntVar {
                IntVar(model.push_variable(Domain::from(self).flattened_intervals(), String::new()))
            }
        }

        impl NewVar for Named<$ty> {
            type Output = IntVar;

            fn insert(self, model: &mut CpModel) -> IntVar {
                IntVar(model.push_variable(Domain::from(self.var).flattened_intervals(), self.name))
            }
        }
    )*};
}

domain_var!(Domain, Range<i64>, RangeInclusive<i64>);

impl sealed::Sealed for Bool {}
impl WithName for Bool {}

impl NewVar for Bool {
    type Output = BoolVar;

    fn insert(self, model: &mut CpModel) -> BoolVar {
        BoolVar(model.push_variable(vec![0, 1], String::new()))
    }
}

impl NewVar for Named<Bool> {
    type Output = BoolVar;

    fn insert(self, model: &mut CpModel) -> BoolVar {
        BoolVar(model.push_variable(vec![0, 1], self.name))
    }
}
