//! Constraints, built before [`CpModel::add`](crate::prelude::CpModel::add) inserts them.
//!
//! Every constraint is a [`Constraint`], which owns its `ConstraintProto`, so
//! inserting it moves the message into the model. Each family is a
//! constructor named after Python's `add_*` method without the prefix, such
//! as [`Constraint::all_different`] for `add_all_different`. A family with many
//! entries, such as the arcs of [`Constraint::circuit`], takes an iterator of
//! tuples.

// Alternative if constructors stop scaling: one type per family, such as
// `AllDifferent::new(exprs)` or `Circuit::new().arc(0, 1, a)`, each wrapping
// its message and converting into `Constraint`. It would cost about 25 public
// types, `only_enforce_if` and `with_name` on each of them, and wrappers around the
// shared `BoolArgumentProto` and `LinearArgumentProto`. Prefer it only when a
// family needs state that a tuple cannot express.

mod arithmetic;
mod boolean;
mod circuit;
mod global;
mod linear;
mod scheduling;

use crate::expr::{BoolVar, IntVar, IntervalVar};
use crate::proto::ConstraintProto;
use crate::proto::constraint_proto::Constraint as Kind;

/// A constraint before insertion, like the `Constraint` that Python's `add_*`
/// methods return, except that it is configured before insertion.
///
/// ```
/// use cpsat::prelude::*;
///
/// // From `channeling_sample_sat.py`.
/// let mut model = CpModel::new();
/// let x = model.new_var(0..=10);
/// let y = model.new_var(0..=10);
/// let b = model.new_var(Bool);
/// model.add(x.ge(5).only_enforce_if(b));
/// model.add(x.lt(5).only_enforce_if(!b));
/// model.add(y.eq(10 - x).only_enforce_if(b).with_name("y follows x"));
/// model.add(y.eq(0).only_enforce_if(!b));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Constraint(ConstraintProto);

impl Constraint {
    /// Enforces this constraint only when `literal` is true. Several calls
    /// require all their literals. Every family accepts it, but OR-Tools notes
    /// that an enforced circuit, multiple circuit, no-overlap, 2D no-overlap,
    /// or cumulative constraint may solve slower.
    #[doc(alias("OnlyEnforceIf"))]
    #[must_use]
    pub fn only_enforce_if(mut self, literal: BoolVar) -> Self {
        self.0.enforcement_literal.push(literal.0);
        self
    }

    /// Names the constraint.
    #[doc(alias("WithName"))]
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.0.name = name.into();
        self
    }

    /// The constraint's message.
    #[must_use]
    pub fn proto(&self) -> &ConstraintProto {
        &self.0
    }

    /// Unwraps the constraint's message.
    #[must_use]
    pub fn into_proto(self) -> ConstraintProto {
        self.0
    }

    fn of(kind: impl Into<Kind>) -> Self {
        Self(ConstraintProto {
            constraint: Some(kind.into()),
            ..Default::default()
        })
    }
}

impl From<ConstraintProto> for Constraint {
    fn from(proto: ConstraintProto) -> Self {
        Self(proto)
    }
}

impl From<Constraint> for ConstraintProto {
    fn from(constraint: Constraint) -> Self {
        constraint.0
    }
}

/// A constraint in a model, an index into `CpModelProto::constraints`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstraintIndex(pub(crate) i32);

impl ConstraintIndex {
    /// The index into `CpModelProto::constraints`.
    #[must_use]
    pub const fn index(self) -> i32 {
        self.0
    }
}

impl From<IntervalVar> for ConstraintIndex {
    /// The interval's row, which the schema stores as a constraint, so
    /// `model[ConstraintIndex::from(interval)]` reads its name and
    /// enforcement literals.
    fn from(interval: IntervalVar) -> Self {
        Self(interval.index())
    }
}

fn literals(literals: impl IntoIterator<Item = BoolVar>) -> Vec<i32> {
    literals.into_iter().map(BoolVar::index).collect()
}

fn vars(vars: impl IntoIterator<Item = IntVar>) -> Vec<i32> {
    vars.into_iter().map(IntVar::index).collect()
}
