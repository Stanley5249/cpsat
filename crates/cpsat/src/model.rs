//! Helpers on the generated model types for the common cases of Python's
//! `CpModel`. Every field stays public, so anything else is a direct edit of
//! the Protobuf message.
//!
//! Variables and literals are `i32` indices into [`CpModelProto::variables`],
//! as in the schema. A negative literal `-i - 1` means "not `i`", which
//! [`not`] computes.

use buffa::MessageField;

use crate::proto::constraint_proto::Constraint;
use crate::proto::{
    ConstraintProto, CpModelProto, CpObjectiveProto, IntegerVariableProto, LinearConstraintProto,
};

/// Returns the negation of a Boolean literal, like Python's `literal.Not()`.
///
/// Bitwise not equals `-literal - 1` and never overflows.
#[must_use]
pub const fn not(literal: i32) -> i32 {
    !literal
}

impl IntegerVariableProto {
    /// A variable with the domain `[lower, upper]`.
    #[must_use]
    pub fn new(lower: i64, upper: i64) -> Self {
        Self {
            domain: vec![lower, upper],
            ..Default::default()
        }
    }
}

impl LinearConstraintProto {
    /// `sum(coeff * var)` must lie in `domain`, a flat list of disjoint
    /// intervals `[min_0, max_0, min_1, max_1, ...]`.
    #[must_use]
    pub fn new(terms: &[(i32, i64)], domain: Vec<i64>) -> Self {
        let (vars, coeffs) = terms.iter().copied().unzip();
        Self {
            vars,
            coeffs,
            domain,
            ..Default::default()
        }
    }
}

impl From<LinearConstraintProto> for ConstraintProto {
    fn from(linear: LinearConstraintProto) -> Self {
        Self {
            constraint: Some(Constraint::from(linear)),
            ..Default::default()
        }
    }
}

impl CpModelProto {
    /// Adds an integer variable with the domain `[lower, upper]` and returns
    /// its index, like Python's `new_int_var`.
    pub fn add_int_var(&mut self, lower: i64, upper: i64) -> i32 {
        self.add_variable(IntegerVariableProto::new(lower, upper))
    }

    /// Adds a Boolean variable and returns its index, like Python's
    /// `new_bool_var`.
    pub fn add_bool_var(&mut self) -> i32 {
        self.add_int_var(0, 1)
    }

    /// Adds a variable and returns its index.
    ///
    /// # Panics
    ///
    /// Panics if the model already holds `i32::MAX` variables, which the
    /// schema cannot index.
    pub fn add_variable(&mut self, variable: IntegerVariableProto) -> i32 {
        let index =
            i32::try_from(self.variables.len()).expect("the schema indexes variables with i32");
        self.variables.push(variable);
        index
    }

    /// Adds a constraint and returns it, so the caller can set fields such as
    /// `enforcement_literal`.
    pub fn add_constraint(
        &mut self,
        constraint: impl Into<ConstraintProto>,
    ) -> &mut ConstraintProto {
        let index = self.constraints.len();
        self.constraints.push(constraint.into());
        &mut self.constraints[index]
    }

    /// Adds `lower <= sum(coeff * var) <= upper`.
    pub fn add_linear(
        &mut self,
        terms: &[(i32, i64)],
        lower: i64,
        upper: i64,
    ) -> &mut ConstraintProto {
        self.add_constraint(LinearConstraintProto::new(terms, vec![lower, upper]))
    }

    /// Minimizes `sum(coeff * var)`, replacing any objective.
    pub fn minimize(&mut self, terms: &[(i32, i64)]) {
        let (vars, coeffs) = terms.iter().copied().unzip();
        self.objective = MessageField::some(CpObjectiveProto {
            vars,
            coeffs,
            ..Default::default()
        });
    }

    /// Maximizes `sum(coeff * var)`, replacing any objective. Like Python,
    /// this minimizes the negated terms with a scaling factor of -1.
    pub fn maximize(&mut self, terms: &[(i32, i64)]) {
        let (vars, coeffs) = terms.iter().map(|&(var, coeff)| (var, -coeff)).unzip();
        self.objective = MessageField::some(CpObjectiveProto {
            vars,
            coeffs,
            scaling_factor: -1.0,
            ..Default::default()
        });
    }

    /// Adds a solution hint, like Python's `add_hint`.
    pub fn add_hint(&mut self, var: i32, value: i64) {
        let hint = self.solution_hint.get_or_insert_default();
        hint.vars.push(var);
        hint.values.push(value);
    }

    /// Removes every solution hint, like Python's `clear_hints`.
    pub fn clear_hints(&mut self) {
        self.solution_hint = MessageField::none();
    }
}
