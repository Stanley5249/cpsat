//! Floating-point linear expressions for objectives, like the float sums that
//! Python builds when an expression meets a `float`.

use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::expr::{BoolVar, IntVar, LinearExpr};

/// `sum(coeff * var) + offset` with `f64` coefficients.
///
/// Multiplying a variable or a [`LinearExpr`] by an `f64` builds one, as
/// `0.5 * x` does in Python. CP-SAT accepts floating-point coefficients only
/// in the objective, so a `FloatExpr` serves
/// [`Model::minimize`](crate::prelude::Model::minimize) and `value`
/// and converts into no constraint.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let x = model.new_var(0..=10);
/// let y = model.new_var(0..=10);
/// model.add((x + y).le(7));
/// model.maximize(0.5 * x + 1.25 * y + 0.1);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloatExpr {
    terms: Vec<(i32, f64)>,
    offset: f64,
}

impl FloatExpr {
    /// The expression with no terms and offset `value`.
    #[must_use]
    pub fn constant(value: f64) -> Self {
        Self {
            terms: Vec::new(),
            offset: value,
        }
    }

    /// Adds `factor * other` to this expression.
    fn add_scaled(&mut self, other: Self, factor: f64) {
        self.terms.extend(
            other
                .terms
                .into_iter()
                .map(|(var, coeff)| (var, coeff * factor)),
        );
        self.offset += other.offset * factor;
    }

    /// The terms, sorted by variable, merged, and without zero coefficients,
    /// as Python writes them into the objective, and the offset.
    pub(crate) fn into_terms(self) -> (Vec<i32>, Vec<f64>, f64) {
        let mut terms = self.terms;
        terms.sort_by_key(|&(var, _)| var);
        let mut merged: Vec<(i32, f64)> = Vec::with_capacity(terms.len());
        for (var, coeff) in terms {
            match merged.last_mut() {
                Some(last) if last.0 == var => last.1 += coeff,
                _ => merged.push((var, coeff)),
            }
        }
        merged.retain(|&(_, coeff)| coeff != 0.0);
        let (vars, coeffs) = merged.into_iter().unzip();
        (vars, coeffs, self.offset)
    }

    /// The value under `solution`, indexed by variable, or `None` if a
    /// variable is out of range.
    #[expect(
        clippy::cast_precision_loss,
        reason = "Python evaluates float expressions in doubles too"
    )]
    pub(crate) fn value_in(&self, solution: &[i64]) -> Option<f64> {
        self.terms
            .iter()
            .try_fold(self.offset, |sum, &(var, coeff)| {
                let value = solution.get(usize::try_from(var).ok()?)?;
                Some(sum + coeff * *value as f64)
            })
    }
}

impl From<f64> for FloatExpr {
    fn from(value: f64) -> Self {
        Self::constant(value)
    }
}

impl From<LinearExpr> for FloatExpr {
    #[expect(
        clippy::cast_precision_loss,
        reason = "the schema stores float objectives as doubles, like Python"
    )]
    fn from(expr: LinearExpr) -> Self {
        let (vars, coeffs, offset) = expr.into_terms();
        Self {
            terms: vars
                .into_iter()
                .zip(coeffs.into_iter().map(|coeff| coeff as f64))
                .collect(),
            offset: offset as f64,
        }
    }
}

impl From<&LinearExpr> for FloatExpr {
    fn from(expr: &LinearExpr) -> Self {
        expr.clone().into()
    }
}

impl From<&Self> for FloatExpr {
    /// Clones the expression, so `response.value(&expr)` keeps `expr`.
    fn from(expr: &Self) -> Self {
        expr.clone()
    }
}

impl From<i64> for FloatExpr {
    fn from(value: i64) -> Self {
        LinearExpr::constant(value).into()
    }
}

impl From<IntVar> for FloatExpr {
    fn from(var: IntVar) -> Self {
        LinearExpr::from(var).into()
    }
}

impl From<BoolVar> for FloatExpr {
    fn from(literal: BoolVar) -> Self {
        LinearExpr::from(literal).into()
    }
}

impl<R: Into<Self>> AddAssign<R> for FloatExpr {
    fn add_assign(&mut self, rhs: R) {
        self.add_scaled(rhs.into(), 1.0);
    }
}

impl<R: Into<Self>> SubAssign<R> for FloatExpr {
    fn sub_assign(&mut self, rhs: R) {
        self.add_scaled(rhs.into(), -1.0);
    }
}

impl MulAssign<f64> for FloatExpr {
    fn mul_assign(&mut self, factor: f64) {
        let expr = std::mem::take(self);
        self.add_scaled(expr, factor);
    }
}

impl<R: Into<Self>> Add<R> for FloatExpr {
    type Output = Self;

    fn add(mut self, rhs: R) -> Self {
        self += rhs;
        self
    }
}

impl<R: Into<Self>> Sub<R> for FloatExpr {
    type Output = Self;

    fn sub(mut self, rhs: R) -> Self {
        self -= rhs;
        self
    }
}

impl Mul<f64> for FloatExpr {
    type Output = Self;

    fn mul(mut self, factor: f64) -> Self {
        self *= factor;
        self
    }
}

impl Add<FloatExpr> for f64 {
    type Output = FloatExpr;

    fn add(self, expr: FloatExpr) -> FloatExpr {
        expr + self
    }
}

impl Sub<FloatExpr> for f64 {
    type Output = FloatExpr;

    fn sub(self, expr: FloatExpr) -> FloatExpr {
        -expr + self
    }
}

impl Mul<FloatExpr> for f64 {
    type Output = FloatExpr;

    fn mul(self, expr: FloatExpr) -> FloatExpr {
        expr * self
    }
}

impl Neg for FloatExpr {
    type Output = Self;

    fn neg(self) -> Self {
        self * -1.0
    }
}

impl<T: Into<Self>> Sum<T> for FloatExpr {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(Self::default(), |sum, item| sum + item)
    }
}

/// The float operators of the types that convert into [`LinearExpr`].
macro_rules! float {
    ($($ty:ty),*) => {$(
        impl Mul<f64> for $ty {
            type Output = FloatExpr;

            fn mul(self, factor: f64) -> FloatExpr {
                FloatExpr::from(self) * factor
            }
        }

        impl Mul<$ty> for f64 {
            type Output = FloatExpr;

            fn mul(self, rhs: $ty) -> FloatExpr {
                FloatExpr::from(rhs) * self
            }
        }

        impl Add<f64> for $ty {
            type Output = FloatExpr;

            fn add(self, rhs: f64) -> FloatExpr {
                FloatExpr::from(self) + rhs
            }
        }

        impl Sub<f64> for $ty {
            type Output = FloatExpr;

            fn sub(self, rhs: f64) -> FloatExpr {
                FloatExpr::from(self) - rhs
            }
        }

        impl Add<$ty> for f64 {
            type Output = FloatExpr;

            fn add(self, rhs: $ty) -> FloatExpr {
                FloatExpr::from(rhs) + self
            }
        }

        impl Sub<$ty> for f64 {
            type Output = FloatExpr;

            fn sub(self, rhs: $ty) -> FloatExpr {
                -FloatExpr::from(rhs) + self
            }
        }

        impl Add<FloatExpr> for $ty {
            type Output = FloatExpr;

            fn add(self, rhs: FloatExpr) -> FloatExpr {
                FloatExpr::from(self) + rhs
            }
        }

        impl Sub<FloatExpr> for $ty {
            type Output = FloatExpr;

            fn sub(self, rhs: FloatExpr) -> FloatExpr {
                FloatExpr::from(self) - rhs
            }
        }
    )*};
}

float!(IntVar, BoolVar, LinearExpr);
