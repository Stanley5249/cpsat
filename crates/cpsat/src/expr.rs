//! Integer linear expressions, like Python's `LinearExpr`.

use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::constraint::{self, Constraint};
use crate::domain::Domain;
use crate::proto::LinearExpressionProto;
use crate::variable::{BoolVar, IntVar};

/// `sum(coeff * var) + offset` with `i64` coefficients.
///
/// Arithmetic operators build one from variables, literals, and constants,
/// and the comparison methods turn it into a linear [`Constraint`]. A negated
/// literal `!b` counts as `1 - b`.
///
/// The arithmetic panics when a coefficient or the offset overflows `i64`,
/// where Python wraps silently. `eq`, `lt`, and `le` panic on a constant
/// `i64::MIN`, and `eq`, `ge`, and `gt` on a constant `i64::MAX`, because
/// domains read these limits as infinity. Python raises on these limits too.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let x = model.int_var(0..=10);
/// let y = model.int_var(0..=10);
/// let total: LinearExpr = [x, y].into_iter().sum();
/// model.add((total + 2 * y).le(10));
/// ```
#[derive(Clone, Debug, Default)]
pub struct LinearExpr(LinearExpressionProto);

const OVERFLOW: &str = "linear expression overflows i64";

impl LinearExpr {
    #[must_use]
    pub fn constant(value: i64) -> Self {
        Self(LinearExpressionProto {
            offset: value,
            ..Default::default()
        })
    }

    fn term(var: i32, coeff: i64) -> Self {
        Self(LinearExpressionProto {
            vars: vec![var],
            coeffs: vec![coeff],
            ..Default::default()
        })
    }

    /// Adds `factor * other` to this expression.
    fn add_scaled(&mut self, other: Self, factor: i64) {
        let LinearExpressionProto {
            vars,
            coeffs,
            offset,
            ..
        } = other.0;
        self.0.vars.extend(vars);
        self.0.coeffs.extend(
            coeffs
                .into_iter()
                .map(|coeff| coeff.checked_mul(factor).expect(OVERFLOW)),
        );
        self.0.offset = offset
            .checked_mul(factor)
            .and_then(|scaled| self.0.offset.checked_add(scaled))
            .expect(OVERFLOW);
    }

    /// The terms, sorted by variable, merged, and without zero coefficients,
    /// as Python writes them into a constraint, and the offset.
    pub(crate) fn into_terms(self) -> (Vec<i32>, Vec<i64>, i64) {
        let LinearExpressionProto {
            vars,
            coeffs,
            offset,
            ..
        } = self.0;
        let mut terms: Vec<(i32, i64)> = vars.into_iter().zip(coeffs).collect();
        terms.sort_unstable_by_key(|&(var, _)| var);
        let mut merged: Vec<(i32, i64)> = Vec::with_capacity(terms.len());
        for (var, coeff) in terms {
            match merged.last_mut() {
                Some(last) if last.0 == var => {
                    last.1 = last.1.checked_add(coeff).expect(OVERFLOW);
                }
                _ => merged.push((var, coeff)),
            }
        }
        merged.retain(|&(_, coeff)| coeff != 0);
        let (vars, coeffs) = merged.into_iter().unzip();
        (vars, coeffs, offset)
    }

    /// The value under `solution`, indexed by variable, or `None` if a
    /// variable is out of range.
    ///
    /// # Panics
    ///
    /// If the value overflows `i64`.
    pub(crate) fn evaluate(&self, solution: &[i64]) -> Option<i64> {
        self.0
            .vars
            .iter()
            .zip(&self.0.coeffs)
            .try_fold(self.0.offset, |sum, (&var, &coeff)| {
                let value = solution.get(usize::try_from(var).ok()?)?;
                Some(
                    coeff
                        .checked_mul(*value)
                        .and_then(|term| sum.checked_add(term))
                        .expect(OVERFLOW),
                )
            })
    }

    /// `self` compared with `rhs` like Python: a constant `rhs` builds the
    /// domain directly, and otherwise `self - rhs` lies in `relative`.
    fn compare(
        self,
        rhs: Self,
        relative: &Domain,
        constant: impl FnOnce(i64) -> Domain,
    ) -> Constraint {
        if rhs.0.vars.is_empty() {
            constraint::linear(self, &constant(rhs.0.offset))
        } else {
            constraint::linear(self - rhs, relative)
        }
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

impl From<LinearExpr> for LinearExpressionProto {
    /// The canonical form that Python writes, with terms sorted and merged.
    fn from(expr: LinearExpr) -> Self {
        let (vars, coeffs, offset) = expr.into_terms();
        Self {
            vars,
            coeffs,
            offset,
            ..Default::default()
        }
    }
}

impl From<LinearExpressionProto> for LinearExpr {
    fn from(proto: LinearExpressionProto) -> Self {
        Self(proto)
    }
}

impl From<&Self> for LinearExpr {
    /// Clones the expression, so `response.value(&expr)` keeps `expr`.
    fn from(expr: &Self) -> Self {
        expr.clone()
    }
}

impl From<i64> for LinearExpr {
    fn from(value: i64) -> Self {
        Self::constant(value)
    }
}

impl From<IntVar> for LinearExpr {
    fn from(var: IntVar) -> Self {
        Self::term(var.0, 1)
    }
}

impl From<BoolVar> for LinearExpr {
    fn from(literal: BoolVar) -> Self {
        if literal.is_negated() {
            let mut expr = Self::term(literal.variable().0, -1);
            expr.0.offset = 1;
            expr
        } else {
            Self::term(literal.0, 1)
        }
    }
}

impl<R: Into<Self>> AddAssign<R> for LinearExpr {
    fn add_assign(&mut self, rhs: R) {
        self.add_scaled(rhs.into(), 1);
    }
}

impl<R: Into<Self>> SubAssign<R> for LinearExpr {
    fn sub_assign(&mut self, rhs: R) {
        self.add_scaled(rhs.into(), -1);
    }
}

impl MulAssign<i64> for LinearExpr {
    fn mul_assign(&mut self, factor: i64) {
        let expr = std::mem::take(self);
        self.add_scaled(expr, factor);
    }
}

impl<T: Into<Self>> Sum<T> for LinearExpr {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::default(), |mut sum, item| {
            sum += item;
            sum
        })
    }
}

/// The operators and comparisons of the types that convert into
/// [`LinearExpr`].
macro_rules! linear {
    ($($ty:ty),*) => {$(
        impl<R: Into<LinearExpr>> Add<R> for $ty {
            type Output = LinearExpr;

            fn add(self, rhs: R) -> LinearExpr {
                let mut expr = LinearExpr::from(self);
                expr += rhs;
                expr
            }
        }

        impl<R: Into<LinearExpr>> Sub<R> for $ty {
            type Output = LinearExpr;

            fn sub(self, rhs: R) -> LinearExpr {
                let mut expr = LinearExpr::from(self);
                expr -= rhs;
                expr
            }
        }

        impl Mul<i64> for $ty {
            type Output = LinearExpr;

            fn mul(self, factor: i64) -> LinearExpr {
                let mut expr = LinearExpr::from(self);
                expr *= factor;
                expr
            }
        }

        impl Neg for $ty {
            type Output = LinearExpr;

            fn neg(self) -> LinearExpr {
                self * -1
            }
        }

        impl Add<$ty> for i64 {
            type Output = LinearExpr;

            fn add(self, rhs: $ty) -> LinearExpr {
                rhs + self
            }
        }

        impl Sub<$ty> for i64 {
            type Output = LinearExpr;

            fn sub(self, rhs: $ty) -> LinearExpr {
                -rhs + self
            }
        }

        impl Mul<$ty> for i64 {
            type Output = LinearExpr;

            fn mul(self, rhs: $ty) -> LinearExpr {
                rhs * self
            }
        }

        impl $ty {
            /// `self == rhs`.
            #[must_use]
            pub fn eq(self, rhs: impl Into<LinearExpr>) -> Constraint {
                LinearExpr::from(self).compare(rhs.into(), &Domain::new(0, 0), |c| {
                    Domain::new(finite(c, i64::MIN), finite(c, i64::MAX))
                })
            }

            /// `self != rhs`.
            #[must_use]
            pub fn ne(self, rhs: impl Into<LinearExpr>) -> Constraint {
                LinearExpr::from(self).compare(rhs.into(), &Domain::new(0, 0).complement(), |c| {
                    Domain::new(c, c).complement()
                })
            }

            /// `self <= rhs`.
            #[must_use]
            pub fn le(self, rhs: impl Into<LinearExpr>) -> Constraint {
                LinearExpr::from(self).compare(rhs.into(), &Domain::new(i64::MIN, 0), |c| {
                    Domain::new(i64::MIN, finite(c, i64::MIN))
                })
            }

            /// `self < rhs`, which is `self <= rhs - 1` over integers.
            #[must_use]
            pub fn lt(self, rhs: impl Into<LinearExpr>) -> Constraint {
                LinearExpr::from(self).compare(rhs.into(), &Domain::new(i64::MIN, -1), |c| {
                    Domain::new(i64::MIN, finite(c, i64::MIN) - 1)
                })
            }

            /// `self >= rhs`.
            #[must_use]
            pub fn ge(self, rhs: impl Into<LinearExpr>) -> Constraint {
                LinearExpr::from(self).compare(rhs.into(), &Domain::new(0, i64::MAX), |c| {
                    Domain::new(finite(c, i64::MAX), i64::MAX)
                })
            }

            /// `self > rhs`, which is `self >= rhs + 1` over integers.
            #[must_use]
            pub fn gt(self, rhs: impl Into<LinearExpr>) -> Constraint {
                LinearExpr::from(self).compare(rhs.into(), &Domain::new(1, i64::MAX), |c| {
                    Domain::new(finite(c, i64::MAX) + 1, i64::MAX)
                })
            }

            /// `self` lies in `domain`, like Python's
            /// `add_linear_expression_in_domain`.
            #[must_use]
            pub fn in_domain(self, domain: impl Into<Domain>) -> Constraint {
                constraint::linear(self.into(), &domain.into())
            }
        }
    )*};
}

linear!(IntVar, BoolVar, LinearExpr);
