//! Integer and floating-point objectives, like Python's `minimize` and
//! `maximize`.

use buffa::MessageField;

use super::CpModel;
use crate::expr::{FloatExpr, LinearExpr};
use crate::proto::{CpObjectiveProto, FloatObjectiveProto};

mod sealed {
    pub trait Sealed {}
}

/// What `minimize` and `maximize` accept. An integer expression writes the
/// integer objective, and a [`FloatExpr`] or `f64` writes the floating-point
/// objective, where Python picks by the coefficients at run time.
pub trait Objective: sealed::Sealed {
    #[doc(hidden)]
    fn set(self, model: &mut CpModel, maximize: bool);
}

impl<T: Into<LinearExpr>> sealed::Sealed for T {}
impl<T: Into<LinearExpr>> Objective for T {
    fn set(self, model: &mut CpModel, maximize: bool) {
        // Like Python, maximizing minimizes `-expr` with a scaling factor
        // of -1.
        let expr = self.into();
        if maximize {
            model.set_objective(-expr, -1);
        } else {
            model.set_objective(expr, 1);
        }
    }
}

impl sealed::Sealed for FloatExpr {}
impl Objective for FloatExpr {
    fn set(self, model: &mut CpModel, maximize: bool) {
        model.set_float_objective(self, maximize);
    }
}

impl sealed::Sealed for &FloatExpr {}
impl Objective for &FloatExpr {
    fn set(self, model: &mut CpModel, maximize: bool) {
        model.set_float_objective(self.clone(), maximize);
    }
}

impl sealed::Sealed for f64 {}
impl Objective for f64 {
    fn set(self, model: &mut CpModel, maximize: bool) {
        model.set_float_objective(self.into(), maximize);
    }
}

impl CpModel {
    /// Minimizes `expr`, replacing any objective.
    pub fn minimize(&mut self, expr: impl Objective) {
        expr.set(self, false);
    }

    /// Maximizes `expr`, replacing any objective.
    pub fn maximize(&mut self, expr: impl Objective) {
        expr.set(self, true);
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

    /// Whether the model has an objective.
    #[doc(alias("HasObjective"))]
    #[must_use]
    pub fn has_objective(&self) -> bool {
        self.0.objective.is_set() || self.0.floating_point_objective.is_set()
    }

    /// Removes the integer and the floating-point objectives.
    #[doc(alias("ClearObjective"))]
    pub fn clear_objective(&mut self) {
        self.0.objective = MessageField::none();
        self.0.floating_point_objective = MessageField::none();
    }
}
