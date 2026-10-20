//! Nonlinear equalities, like Python's `add_max_equality` and its relatives.

use buffa::MessageField;

use super::{Constraint, Kind, exprs};
use crate::expr::LinearExpr;
use crate::proto::LinearArgumentProto;

fn linear_argument(
    kind: fn(Box<LinearArgumentProto>) -> Kind,
    target: impl Into<LinearExpr>,
    exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
) -> Constraint {
    Constraint::of(kind(Box::new(LinearArgumentProto {
        target: MessageField::some(target.into().into()),
        exprs: self::exprs(exprs),
        ..Default::default()
    })))
}

impl Constraint {
    /// `target == max(exprs)`.
    #[doc(alias("add_max_equality", "AddMaxEquality"))]
    #[must_use]
    pub fn max_equality(
        target: impl Into<LinearExpr>,
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
    ) -> Self {
        linear_argument(Kind::LinMax, target, exprs)
    }

    /// `target == min(exprs)`, which writes `-target == max(-exprs)`.
    #[doc(alias("add_min_equality", "AddMinEquality"))]
    #[must_use]
    pub fn min_equality(
        target: impl Into<LinearExpr>,
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
    ) -> Self {
        let negated = exprs.into_iter().map(|expr| -expr.into());
        linear_argument(Kind::LinMax, -target.into(), negated)
    }

    /// `target == |expr|`, which writes `target == max(expr, -expr)`.
    #[doc(alias("add_abs_equality", "AddAbsEquality"))]
    #[must_use]
    pub fn abs_equality(target: impl Into<LinearExpr>, expr: impl Into<LinearExpr>) -> Self {
        let expr = expr.into();
        let negated = -expr.clone();
        linear_argument(Kind::LinMax, target, [expr, negated])
    }

    /// `target == numerator / denominator`, rounded towards zero.
    #[doc(alias("add_division_equality", "AddDivisionEquality"))]
    #[must_use]
    pub fn division_equality(
        target: impl Into<LinearExpr>,
        numerator: impl Into<LinearExpr>,
        denominator: impl Into<LinearExpr>,
    ) -> Self {
        linear_argument(Kind::IntDiv, target, [numerator.into(), denominator.into()])
    }

    /// `target == expr % modulus`, with the sign of `expr` as in C.
    #[doc(alias("add_modulo_equality", "AddModuloEquality"))]
    #[must_use]
    pub fn modulo_equality(
        target: impl Into<LinearExpr>,
        expr: impl Into<LinearExpr>,
        modulus: impl Into<LinearExpr>,
    ) -> Self {
        linear_argument(Kind::IntMod, target, [expr.into(), modulus.into()])
    }

    /// `target == exprs[0] * exprs[1] * ...`.
    #[doc(alias("add_multiplication_equality", "AddMultiplicationEquality"))]
    #[must_use]
    pub fn multiplication_equality(
        target: impl Into<LinearExpr>,
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
    ) -> Self {
        linear_argument(Kind::IntProd, target, exprs)
    }
}
