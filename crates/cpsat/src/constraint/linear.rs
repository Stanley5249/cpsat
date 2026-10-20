//! Linear constraints: the comparison methods on handles and expressions, and
//! `in_domain`, like Python's comparison operators and
//! `add_linear_expression_in_domain`.

use super::Constraint;
use crate::expr::{BoolVar, Domain, IntVar, LinearExpr};
use crate::proto::LinearConstraintProto;

/// `expr` in `domain`, with the offset moved into the domain as Python does.
fn linear(expr: LinearExpr, domain: Domain) -> Constraint {
    let (vars, coeffs, offset) = expr.into_terms();
    let domain = if offset == 0 {
        domain.into_flattened_intervals()
    } else {
        // The domain bounds i64::MIN and i64::MAX stand for infinity, so they
        // stay, and finite bounds saturate into them like Python's `Domain`
        // arithmetic. A saturated bound admits no extra value, because
        // OR-Tools rejects a variable whose domain reaches near either limit.
        let shift = |bound: i64| match bound {
            i64::MIN | i64::MAX => bound,
            _ => i64::try_from(
                (i128::from(bound) - i128::from(offset)).clamp(i64::MIN.into(), i64::MAX.into()),
            )
            .expect("the bound is clamped into i64"),
        };
        // Saturation can make intervals touch, so normalize them again.
        Domain::from_intervals(
            domain
                .intervals()
                .iter()
                .map(|&[lower, upper]| [shift(lower), shift(upper)]),
        )
        .into_flattened_intervals()
    };
    Constraint::of(LinearConstraintProto {
        vars,
        coeffs,
        domain,
        ..Default::default()
    })
}

/// `lhs` compared with `rhs` like Python: a constant `rhs` builds the
/// domain directly, and otherwise `lhs - rhs` lies in `relative()`.
fn compare(
    lhs: LinearExpr,
    rhs: LinearExpr,
    relative: impl FnOnce() -> Domain,
    constant: impl FnOnce(i64) -> Domain,
) -> Constraint {
    match rhs.constant_value() {
        Some(value) => linear(lhs, constant(value)),
        None => linear(lhs - rhs, relative()),
    }
}

/// `value` as a finite domain bound.
///
/// # Panics
///
/// If `value` equals `infinity`, which the bound would read as infinite, like
/// Python's `ArithmeticError`.
fn finite(value: i64, infinity: i64) -> i64 {
    assert!(
        value != infinity,
        "comparing with {value} is not supported, because domains read it as infinity",
    );
    value
}

macro_rules! comparisons {
    ($($ty:ty),*) => {$(
        impl $ty {
            /// `self == rhs`.
            ///
            /// # Panics
            ///
            /// On a constant `rhs` at a domain limit or an `i64` overflow, as
            /// [`LinearExpr`] describes.
            #[doc(alias("AddEquality"))]
            #[must_use]
            pub fn eq(self, rhs: impl Into<LinearExpr>) -> Constraint {
                compare(self.into(), rhs.into(), || Domain::new(0, 0), |c| {
                    Domain::new(finite(c, i64::MIN), finite(c, i64::MAX))
                })
            }

            /// `self != rhs`.
            ///
            /// # Panics
            ///
            /// On an `i64` overflow, as [`LinearExpr`] describes.
            #[doc(alias("AddNotEqual"))]
            #[must_use]
            pub fn ne(self, rhs: impl Into<LinearExpr>) -> Constraint {
                compare(self.into(), rhs.into(), || Domain::new(0, 0).complement(), |c| {
                    Domain::new(c, c).complement()
                })
            }

            /// `self <= rhs`.
            ///
            /// # Panics
            ///
            /// On a constant `rhs` at a domain limit or an `i64` overflow, as
            /// [`LinearExpr`] describes.
            #[doc(alias("AddLessOrEqual"))]
            #[must_use]
            pub fn le(self, rhs: impl Into<LinearExpr>) -> Constraint {
                compare(self.into(), rhs.into(), || Domain::new(i64::MIN, 0), |c| {
                    Domain::new(i64::MIN, finite(c, i64::MIN))
                })
            }

            /// `self < rhs`, which is `self <= rhs - 1` over integers.
            ///
            /// # Panics
            ///
            /// On a constant `rhs` at a domain limit or an `i64` overflow, as
            /// [`LinearExpr`] describes.
            #[doc(alias("AddLessThan"))]
            #[must_use]
            pub fn lt(self, rhs: impl Into<LinearExpr>) -> Constraint {
                compare(self.into(), rhs.into(), || Domain::new(i64::MIN, -1), |c| {
                    Domain::new(i64::MIN, finite(c, i64::MIN) - 1)
                })
            }

            /// `self >= rhs`.
            ///
            /// # Panics
            ///
            /// On a constant `rhs` at a domain limit or an `i64` overflow, as
            /// [`LinearExpr`] describes.
            #[doc(alias("AddGreaterOrEqual"))]
            #[must_use]
            pub fn ge(self, rhs: impl Into<LinearExpr>) -> Constraint {
                compare(self.into(), rhs.into(), || Domain::new(0, i64::MAX), |c| {
                    Domain::new(finite(c, i64::MAX), i64::MAX)
                })
            }

            /// `self > rhs`, which is `self >= rhs + 1` over integers.
            ///
            /// # Panics
            ///
            /// On a constant `rhs` at a domain limit or an `i64` overflow, as
            /// [`LinearExpr`] describes.
            #[doc(alias("AddGreaterThan"))]
            #[must_use]
            pub fn gt(self, rhs: impl Into<LinearExpr>) -> Constraint {
                compare(self.into(), rhs.into(), || Domain::new(1, i64::MAX), |c| {
                    Domain::new(finite(c, i64::MAX) + 1, i64::MAX)
                })
            }

            /// `self` lies in `domain`.
            ///
            /// # Panics
            ///
            /// On an `i64` overflow, as [`LinearExpr`] describes.
            #[doc(alias(
                "add_linear_constraint",
                "add_linear_expression_in_domain",
                "AddLinearConstraint"
            ))]
            #[must_use]
            pub fn in_domain(self, domain: impl Into<Domain>) -> Constraint {
                linear(self.into(), domain.into())
            }
        }
    )*};
}

comparisons!(IntVar, BoolVar, LinearExpr);
