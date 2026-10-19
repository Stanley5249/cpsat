//! Constraints, built before [`Model::add`](crate::prelude::Model::add) inserts them.
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
// types, `only_if` and `with_name` on each of them, and wrappers around the
// shared `BoolArgumentProto` and `LinearArgumentProto`. Prefer it only when a
// family needs state that a tuple cannot express.

mod arithmetic;
mod boolean;
mod circuit;
mod global;
mod scheduling;

use crate::domain::Domain;
use crate::expr::LinearExpr;
use crate::proto::constraint_proto::Constraint as Kind;
use crate::proto::{ConstraintProto, LinearConstraintProto, LinearExpressionProto};
use crate::variable::{BoolVar, IntVar};

/// A constraint before insertion, like the `Constraint` that Python's `add_*`
/// methods return, except that it is configured before insertion.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let x = model.int_var(0..=10);
/// let enabled = model.bool_var();
/// model.add(x.ge(5).only_if(enabled).with_name("x is large"));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Constraint(ConstraintProto);

impl Constraint {
    /// Enforces this constraint only when `literal` is true, like Python's
    /// `only_enforce_if`. Several calls require all their literals. Every
    /// family accepts it, but OR-Tools notes that an enforced circuit,
    /// multiple circuit, no-overlap, or cumulative constraint may solve slower.
    #[must_use]
    pub fn only_if(mut self, literal: BoolVar) -> Self {
        self.0.enforcement_literal.push(literal.0);
        self
    }

    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.0.name = name.into();
        self
    }

    #[must_use]
    pub fn proto(&self) -> &ConstraintProto {
        &self.0
    }

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
pub struct ConstraintRef(pub(crate) i32);

impl ConstraintRef {
    /// The index into `CpModelProto::constraints`.
    #[must_use]
    pub const fn index(self) -> i32 {
        self.0
    }
}

/// `expr` in `domain`, with the offset moved into the domain as Python does.
pub(crate) fn linear(expr: LinearExpr, domain: &Domain) -> Constraint {
    let (vars, coeffs, offset) = expr.into_terms();
    // The domain bounds i64::MIN and i64::MAX stand for infinity, so they
    // stay, and finite bounds saturate into them like Python's `Domain`
    // arithmetic. A saturated bound admits no extra value, because OR-Tools
    // rejects a variable whose domain reaches near either limit.
    let shift = |bound: i64| match bound {
        i64::MIN | i64::MAX => bound,
        _ => i64::try_from(
            (i128::from(bound) - i128::from(offset)).clamp(i64::MIN.into(), i64::MAX.into()),
        )
        .expect("the bound is clamped into i64"),
    };
    // Saturation can make intervals touch, so normalize them again.
    let domain = Domain::from_intervals(
        domain
            .intervals()
            .iter()
            .map(|&(lower, upper)| (shift(lower), shift(upper))),
    );
    Constraint::of(LinearConstraintProto {
        vars,
        coeffs,
        domain: domain.flattened_intervals(),
        ..Default::default()
    })
}

fn exprs(exprs: impl IntoIterator<Item = impl Into<LinearExpr>>) -> Vec<LinearExpressionProto> {
    exprs.into_iter().map(|expr| expr.into().into()).collect()
}

fn literals(literals: impl IntoIterator<Item = BoolVar>) -> Vec<i32> {
    literals.into_iter().map(BoolVar::index).collect()
}

fn variables(vars: impl IntoIterator<Item = IntVar>) -> Vec<i32> {
    vars.into_iter().map(IntVar::index).collect()
}
