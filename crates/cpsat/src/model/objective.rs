//! Integer and floating-point objectives, like Python's `minimize` and
//! `maximize`.

use buffa::MessageField;

use super::Model;
use crate::expr::LinearExpr;
use crate::float_expr::FloatExpr;
use crate::proto::{CpObjectiveProto, FloatObjectiveProto};

impl Model {
    /// Minimizes `expr`, replacing any objective.
    pub fn minimize(&mut self, expr: impl Into<LinearExpr>) {
        self.set_objective(expr.into(), 1);
    }

    /// Maximizes `expr`, replacing any objective. Like Python, this minimizes
    /// `-expr` with a scaling factor of -1.
    pub fn maximize(&mut self, expr: impl Into<LinearExpr>) {
        self.set_objective(-expr.into(), -1);
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "the schema stores the offset as a double, like Python"
    )]
    fn set_objective(&mut self, expr: LinearExpr, scaling: i8) {
        self.clear_objective();
        let (vars, coeffs, offset) = expr.into_terms();
        self.0.objective = MessageField::some(CpObjectiveProto {
            vars,
            coeffs,
            offset: offset as f64,
            scaling_factor: scaling.into(),
            ..Default::default()
        });
    }

    /// Minimizes a floating-point `expr`, replacing any objective, like
    /// Python's `minimize` with float coefficients.
    pub fn minimize_float(&mut self, expr: impl Into<FloatExpr>) {
        self.set_float_objective(expr.into(), false);
    }

    /// Maximizes a floating-point `expr`, replacing any objective, like
    /// Python's `maximize` with float coefficients.
    pub fn maximize_float(&mut self, expr: impl Into<FloatExpr>) {
        self.set_float_objective(expr.into(), true);
    }

    fn set_float_objective(&mut self, expr: FloatExpr, maximize: bool) {
        self.clear_objective();
        let (vars, coeffs, offset) = expr.into_terms();
        self.0.floating_point_objective = MessageField::some(FloatObjectiveProto {
            vars,
            coeffs,
            offset,
            maximize,
            ..Default::default()
        });
    }

    #[must_use]
    pub fn has_objective(&self) -> bool {
        self.0.objective.is_set() || self.0.floating_point_objective.is_set()
    }

    /// Removes the integer and the floating-point objectives.
    pub fn clear_objective(&mut self) {
        self.0.objective = MessageField::none();
        self.0.floating_point_objective = MessageField::none();
    }
}
