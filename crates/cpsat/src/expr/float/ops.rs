//! The arithmetic operators and `Sum` of [`FloatExpr`].

use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use super::FloatExpr;
use crate::expr::{BoolVar, IntVar, LinearExpr};

/// An operand that adds its terms to an expression in place, like the
/// integer `Operand` of [`LinearExpr`].
trait Operand {
    /// How many terms each operand adds, to reserve before a sum.
    const TERMS: usize;

    /// Adds `sign * self` to `expr`.
    fn add_to(self, expr: &mut FloatExpr, sign: f64);
}

impl Operand for f64 {
    const TERMS: usize = 0;

    fn add_to(self, expr: &mut FloatExpr, sign: f64) {
        expr.offset += self * sign;
    }
}

impl Operand for i64 {
    const TERMS: usize = 0;

    #[expect(
        clippy::cast_precision_loss,
        reason = "the schema stores float objectives as doubles, like Python"
    )]
    fn add_to(self, expr: &mut FloatExpr, sign: f64) {
        expr.offset += self as f64 * sign;
    }
}

impl Operand for IntVar {
    const TERMS: usize = 1;

    fn add_to(self, expr: &mut FloatExpr, sign: f64) {
        expr.terms.push((self.0, sign));
    }
}

impl Operand for BoolVar {
    const TERMS: usize = 1;

    fn add_to(self, expr: &mut FloatExpr, sign: f64) {
        if self.is_negated() {
            expr.terms.push((self.to_int_var().0, -sign));
            expr.offset += sign;
        } else {
            expr.terms.push((self.0, sign));
        }
    }
}

impl Operand for LinearExpr {
    const TERMS: usize = 0;

    fn add_to(self, expr: &mut FloatExpr, sign: f64) {
        expr.add_linear(&self, sign);
    }
}

impl Operand for &LinearExpr {
    const TERMS: usize = 0;

    fn add_to(self, expr: &mut FloatExpr, sign: f64) {
        expr.add_linear(self, sign);
    }
}

impl Operand for FloatExpr {
    const TERMS: usize = 0;

    fn add_to(self, expr: &mut FloatExpr, sign: f64) {
        expr.add_scaled(self, sign);
    }
}

impl Operand for &FloatExpr {
    const TERMS: usize = 0;

    fn add_to(self, expr: &mut FloatExpr, sign: f64) {
        expr.add_scaled_ref(self, sign);
    }
}

/// The sum of `operands`, reserving their terms up front.
fn sum<T: Operand>(operands: impl Iterator<Item = T>) -> FloatExpr {
    let mut sum = FloatExpr::default();
    sum.terms.reserve(operands.size_hint().0 * T::TERMS);
    for operand in operands {
        operand.add_to(&mut sum, 1.0);
    }
    sum
}

/// `+=`, `-=`, `+`, and `-` for each [`Operand`].
macro_rules! accumulate {
    ($($ty:ty),*) => {$(
        impl AddAssign<$ty> for FloatExpr {
            fn add_assign(&mut self, rhs: $ty) {
                rhs.add_to(self, 1.0);
            }
        }

        impl SubAssign<$ty> for FloatExpr {
            fn sub_assign(&mut self, rhs: $ty) {
                rhs.add_to(self, -1.0);
            }
        }

        impl Add<$ty> for FloatExpr {
            type Output = Self;

            fn add(mut self, rhs: $ty) -> Self {
                self += rhs;
                self
            }
        }

        impl Sub<$ty> for FloatExpr {
            type Output = Self;

            fn sub(mut self, rhs: $ty) -> Self {
                self -= rhs;
                self
            }
        }
    )*};
}

accumulate!(
    f64,
    i64,
    IntVar,
    BoolVar,
    LinearExpr,
    &LinearExpr,
    FloatExpr,
    &FloatExpr
);

/// `Sum` for each owned [`Operand`].
macro_rules! sum {
    ($($ty:ty),*) => {$(
        impl Sum<$ty> for FloatExpr {
            fn sum<I: Iterator<Item = $ty>>(iter: I) -> Self {
                sum(iter)
            }
        }
    )*};
}

sum!(f64, i64, IntVar, BoolVar, LinearExpr, FloatExpr);

impl<'a> Sum<&'a LinearExpr> for FloatExpr {
    fn sum<I: Iterator<Item = &'a LinearExpr>>(iter: I) -> Self {
        sum(iter)
    }
}

impl<'a> Sum<&'a Self> for FloatExpr {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        sum(iter)
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
