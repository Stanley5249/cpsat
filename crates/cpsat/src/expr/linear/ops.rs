//! The arithmetic operators and `Sum` of [`LinearExpr`].

use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use super::{LinearExpr, OVERFLOW, scales};
use crate::expr::{BoolVar, IntVar};

/// An operand that adds its terms to an expression in place, so `+=` and
/// `Sum` build no one-term expression per variable and clone no borrowed one.
trait Operand {
    /// How many terms each operand adds, to reserve before a sum.
    const TERMS: usize;

    /// Adds `sign * self` to `expr`, leaving `expr` intact on overflow.
    fn add_to(self, expr: &mut LinearExpr, sign: i64);
}

impl Operand for i64 {
    const TERMS: usize = 0;

    fn add_to(self, expr: &mut LinearExpr, sign: i64) {
        expr.offset = self
            .checked_mul(sign)
            .and_then(|value| expr.offset.checked_add(value))
            .expect(OVERFLOW);
    }
}

impl Operand for IntVar {
    const TERMS: usize = 1;

    fn add_to(self, expr: &mut LinearExpr, sign: i64) {
        expr.push_term(self.0, sign, 0);
    }
}

impl Operand for BoolVar {
    const TERMS: usize = 1;

    fn add_to(self, expr: &mut LinearExpr, sign: i64) {
        if self.is_negated() {
            expr.push_term(self.to_int_var().0, -sign, sign);
        } else {
            expr.push_term(self.0, sign, 0);
        }
    }
}

impl Operand for LinearExpr {
    const TERMS: usize = 0;

    fn add_to(self, expr: &mut LinearExpr, sign: i64) {
        expr.add_scaled(self, sign);
    }
}

impl Operand for &LinearExpr {
    const TERMS: usize = 0;

    fn add_to(self, expr: &mut LinearExpr, sign: i64) {
        expr.add_scaled_ref(self, sign);
    }
}

/// The sum of `operands`, reserving their terms up front.
fn sum<T: Operand>(operands: impl Iterator<Item = T>) -> LinearExpr {
    let mut sum = LinearExpr::default();
    sum.terms.reserve(operands.size_hint().0 * T::TERMS);
    for operand in operands {
        operand.add_to(&mut sum, 1);
    }
    sum
}

/// `+=` and `-=` for each [`Operand`].
macro_rules! accumulate {
    ($($ty:ty),*) => {$(
        impl AddAssign<$ty> for LinearExpr {
            fn add_assign(&mut self, rhs: $ty) {
                rhs.add_to(self, 1);
            }
        }

        impl SubAssign<$ty> for LinearExpr {
            fn sub_assign(&mut self, rhs: $ty) {
                rhs.add_to(self, -1);
            }
        }
    )*};
}

accumulate!(i64, IntVar, BoolVar, LinearExpr, &LinearExpr);

/// `Sum` for each owned [`Operand`].
macro_rules! sum {
    ($($ty:ty),*) => {$(
        impl Sum<$ty> for LinearExpr {
            fn sum<I: Iterator<Item = $ty>>(iter: I) -> Self {
                sum(iter)
            }
        }
    )*};
}

sum!(i64, IntVar, BoolVar, LinearExpr);

impl<'a> Sum<&'a Self> for LinearExpr {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        sum(iter)
    }
}

impl MulAssign<i64> for LinearExpr {
    /// Checks every product before the first change, like `add_scaled`.
    fn mul_assign(&mut self, factor: i64) {
        self.offset = self
            .offset
            .checked_mul(factor)
            .filter(|_| scales(&self.terms, factor))
            .expect(OVERFLOW);
        for (_, coeff) in &mut self.terms {
            *coeff *= factor;
        }
    }
}

/// `lhs + rhs` and `lhs - rhs` for each [`Operand`] `rhs`.
macro_rules! add_sub {
    ($lhs:ty; $($rhs:ty),*) => {$(
        impl Add<$rhs> for $lhs {
            type Output = LinearExpr;

            fn add(self, rhs: $rhs) -> LinearExpr {
                let mut expr = LinearExpr::from(self);
                expr += rhs;
                expr
            }
        }

        impl Sub<$rhs> for $lhs {
            type Output = LinearExpr;

            fn sub(self, rhs: $rhs) -> LinearExpr {
                let mut expr = LinearExpr::from(self);
                expr -= rhs;
                expr
            }
        }
    )*};
}

/// The operators and comparisons of the types that convert into
/// [`LinearExpr`].
macro_rules! linear {
    ($($ty:ty),*) => {$(
        add_sub!($ty; i64, IntVar, BoolVar, LinearExpr, &LinearExpr);

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
    )*};
}

linear!(IntVar, BoolVar, LinearExpr);
