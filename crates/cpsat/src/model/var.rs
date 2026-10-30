//! Variable creation through [`CpModel::new_var`], like Python's `new_int_var`
//! and `new_bool_var`, and edits through [`CpModel::var_mut`].

use std::ops::{Range, RangeInclusive};

use super::lookup::slot;
use super::{CpModel, sealed};
use crate::expr::{BoolVar, Domain, IntVar};
use crate::proto::IntegerVariableProto;

/// What [`CpModel::new_var`] creates, and the handle it returns: an
/// [`IntVar`] for a [`Domain`] or a bounded range, a [`BoolVar`] for
/// [`Bool`], and a fixed [`BoolVar`] for a `bool`. An
/// [`Interval`](crate::prelude::Interval) goes through
/// [`CpModel::add`] instead, because the schema stores it as a constraint.
///
/// `RangeFrom` and `RangeToInclusive` are missing on purpose, because they
/// always reach `i64::MAX` or `i64::MIN`, which makes a variable invalid. A
/// closed range or a [`Domain`] that reaches either limit compiles, and
/// [`validate`](CpModel::validate) rejects it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot create a variable",
    label = "expected a bounded `i64` domain, `Bool`, or a `bool`",
    note = "a variable needs finite `i64` bounds, such as `0..=10`",
    note = "an `Interval` goes through `CpModel::add`"
)]
pub trait NewVar: sealed::LowerVar {
    /// The handle that [`CpModel::new_var`] returns.
    type Handle: sealed::FromIndex;
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

impl<T: NewVar + WithName> sealed::LowerVar for Named<T> {
    fn into_proto(self) -> IntegerVariableProto {
        IntegerVariableProto {
            name: self.name,
            ..self.var.into_proto()
        }
    }
}

impl<T: NewVar + WithName> NewVar for Named<T> {
    type Handle = T::Handle;
}

/// An unnamed variable with the flat `domain`.
fn variable(domain: Vec<i64>) -> IntegerVariableProto {
    IntegerVariableProto {
        domain,
        ..Default::default()
    }
}

macro_rules! domain_var {
    ($($ty:ty),*) => {$(
        impl sealed::Sealed for $ty {}
        impl WithName for $ty {}

        impl sealed::LowerVar for $ty {
            fn into_proto(self) -> IntegerVariableProto {
                variable(Domain::from(self).into_flattened_intervals())
            }
        }

        impl NewVar for $ty {
            type Handle = IntVar;
        }
    )*};
}

domain_var!(Domain, Range<i64>, RangeInclusive<i64>);

impl sealed::Sealed for Bool {}
impl WithName for Bool {}

impl sealed::LowerVar for Bool {
    fn into_proto(self) -> IntegerVariableProto {
        variable(vec![0, 1])
    }
}

impl NewVar for Bool {
    type Handle = BoolVar;
}

impl sealed::Sealed for bool {}
impl WithName for bool {}

impl sealed::LowerVar for bool {
    fn into_proto(self) -> IntegerVariableProto {
        variable(vec![self.into(); 2])
    }
}

impl NewVar for bool {
    type Handle = BoolVar;
}

impl CpModel {
    /// Edits `var`, for example to tighten its domain before solving again.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// let mut model = CpModel::new();
    /// let x = model.new_var(0..=10);
    /// model.var_mut(x).set_domain(0..=5).set_name("x");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the model lacks `var`, like indexing with it.
    #[must_use]
    pub fn var_mut(&mut self, var: IntVar) -> VarMut<'_> {
        VarMut(&mut self.0.variables[slot(var.0)])
    }
}

/// A variable that [`CpModel::var_mut`] edits. Each setter writes a
/// well-formed field, so the stored domain stays sorted and disjoint.
#[derive(Debug)]
pub struct VarMut<'a>(&'a mut IntegerVariableProto);

impl VarMut<'_> {
    /// Replaces the domain, like Python's `IntVar.with_domain`. A literal of
    /// the variable stays valid only while the domain stays within `[0, 1]`,
    /// which [`validate`](CpModel::validate) checks where the literal fills a
    /// literal field. A literal used only in arithmetic is not checked, so
    /// after `set_domain(2..=2)` a solution reads `b` as `true` but
    /// `LinearExpr::from(b)` as 2.
    #[doc(alias("with_domain"))]
    pub fn set_domain(&mut self, domain: impl Into<Domain>) -> &mut Self {
        self.0.domain = domain.into().into_flattened_intervals();
        self
    }

    /// Replaces the name.
    pub fn set_name(&mut self, name: impl Into<String>) -> &mut Self {
        self.0.name = name.into();
        self
    }
}
