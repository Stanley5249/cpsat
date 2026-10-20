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
/// [`CpModel::minimize`](crate::prelude::CpModel::minimize) and `value`
/// and converts into no constraint. `PartialEq` compares the stored terms in
/// order, so `x + y` and `y + x` differ.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = CpModel::new();
/// let x = model.new_var(0..=10);
/// let y = model.new_var(0..=10);
/// model.add((x + y).le(7));
/// model.maximize(0.5 * x + 1.25 * y + 0.1);
/// ```
#[doc(alias("DoubleLinearExpr"))]
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

    fn term(var: i32, coeff: f64, offset: f64) -> Self {
        Self {
            terms: vec![(var, coeff)],
            offset,
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
        // A stable sort keeps the sum of a run in the original order, so the
        // rounding is deterministic.
        terms.sort_by_key(|&(var, _)| var);
        let (vars, coeffs) = terms
            .chunk_by(|a, b| a.0 == b.0)
            .filter_map(|run| {
                let sum: f64 = run.iter().map(|&(_, coeff)| coeff).sum();
                (sum != 0.0).then_some((run[0].0, sum))
            })
            .unzip();
        (vars, coeffs, self.offset)
    }

    /// The value under `solution`, indexed by variable.
    ///
    /// # Panics
    ///
    /// If a variable is out of range, like indexing a slice.
    #[expect(
        clippy::cast_precision_loss,
        reason = "Python evaluates float expressions in doubles too"
    )]
    pub(crate) fn value_in(&self, solution: &[i64]) -> f64 {
        self.terms.iter().fold(self.offset, |sum, &(var, coeff)| {
            let value = solution[usize::try_from(var).expect("a variable index is non-negative")];
            sum + coeff * value as f64
        })
    }
}

impl From<f64> for FloatExpr {
    fn from(value: f64) -> Self {
        Self::constant(value)
    }
}

impl From<LinearExpr> for FloatExpr {
    /// Converts each coefficient and the offset to `f64`, which rounds a
    /// magnitude above `2^53`. The terms merge later in `f64`, when the
    /// expression enters the objective, so rounded terms can cancel to zero
    /// and drop out, as in Python.
    #[expect(
        clippy::cast_precision_loss,
        reason = "the schema stores float objectives as doubles, like Python"
    )]
    fn from(expr: LinearExpr) -> Self {
        let (terms, offset) = expr.into_raw_parts();
        Self {
            terms: terms
                .into_iter()
                .map(|(var, coeff)| (var, coeff as f64))
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
    /// Clones the expression, so `solution.value(&expr)` keeps `expr`.
    fn from(expr: &Self) -> Self {
        expr.clone()
    }
}

impl From<i64> for FloatExpr {
    #[expect(
        clippy::cast_precision_loss,
        reason = "the schema stores float objectives as doubles, like Python"
    )]
    fn from(value: i64) -> Self {
        Self::constant(value as f64)
    }
}

impl From<IntVar> for FloatExpr {
    fn from(var: IntVar) -> Self {
        Self::term(var.0, 1.0, 0.0)
    }
}

impl From<BoolVar> for FloatExpr {
    /// Reads `!b` as `1 - b`, like [`LinearExpr`].
    fn from(literal: BoolVar) -> Self {
        if literal.is_negated() {
            Self::term(literal.to_int_var().0, -1.0, 1.0)
        } else {
            Self::term(literal.0, 1.0, 0.0)
        }
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
        for (_, coeff) in &mut self.terms {
            *coeff *= factor;
        }
        self.offset *= factor;
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
