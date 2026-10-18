//! Global constraints over lists of expressions.

use buffa::MessageField;

use super::{Constraint, exprs, variables};
use crate::expr::LinearExpr;
use crate::proto::{
    AllDifferentConstraintProto, AutomatonConstraintProto, ElementConstraintProto,
    InverseConstraintProto, TableConstraintProto,
};
use crate::variable::IntVar;

impl Constraint {
    /// Every expression takes a different value. CP-SAT accepts affine
    /// expressions `a * x + b` here.
    #[must_use]
    pub fn all_different(exprs: impl IntoIterator<Item = impl Into<LinearExpr>>) -> Self {
        Self::of(AllDifferentConstraintProto {
            exprs: self::exprs(exprs),
            ..Default::default()
        })
    }

    /// `exprs[index] == target`, like Python's `add_element`. CP-SAT accepts
    /// affine expressions here.
    #[must_use]
    pub fn element(
        index: impl Into<LinearExpr>,
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
        target: impl Into<LinearExpr>,
    ) -> Self {
        Self::of(ElementConstraintProto {
            linear_index: MessageField::some(index.into().into()),
            linear_target: MessageField::some(target.into().into()),
            exprs: self::exprs(exprs),
            ..Default::default()
        })
    }

    /// `direct[i] == j` exactly when `inverse[j] == i`, like Python's
    /// `add_inverse`.
    #[must_use]
    pub fn inverse(
        direct: impl IntoIterator<Item = IntVar>,
        inverse: impl IntoIterator<Item = IntVar>,
    ) -> Self {
        Self::of(InverseConstraintProto {
            f_direct: variables(direct),
            f_inverse: variables(inverse),
            ..Default::default()
        })
    }

    /// The expressions equal one of the rows, like Python's
    /// `add_allowed_assignments`. Each row lists one value per expression, and
    /// the validator rejects a row of the wrong length.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// let mut model = Model::new();
    /// let x = model.int_var(0..=5);
    /// let y = model.int_var(0..=5);
    /// model.add(Constraint::allowed_assignments([x, y], [[1, 2], [3, 4]]));
    /// ```
    #[must_use]
    pub fn allowed_assignments(
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
        rows: impl IntoIterator<Item = impl IntoIterator<Item = i64>>,
    ) -> Self {
        Self::of(TableConstraintProto {
            exprs: self::exprs(exprs),
            values: rows.into_iter().flatten().collect(),
            ..Default::default()
        })
    }

    /// The expressions equal none of the rows, like Python's
    /// `add_forbidden_assignments`.
    #[must_use]
    pub fn forbidden_assignments(
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
        rows: impl IntoIterator<Item = impl IntoIterator<Item = i64>>,
    ) -> Self {
        Self::of(TableConstraintProto {
            exprs: self::exprs(exprs),
            values: rows.into_iter().flatten().collect(),
            negated: true,
            ..Default::default()
        })
    }

    /// The expressions spell a word that the automaton accepts, like Python's
    /// `add_automaton`. Each transition is `(tail, label, head)`, in the order
    /// of Python's transition triples.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// let mut model = Model::new();
    /// let labels = [(); 3].map(|()| model.int_var(0..=1));
    /// // Accepts words without two consecutive 1s.
    /// let transitions = [(0, 0, 0), (0, 1, 1), (1, 0, 0)];
    /// model.add(Constraint::automaton(labels, 0, [0, 1], transitions));
    /// ```
    #[must_use]
    pub fn automaton(
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
        starting_state: i64,
        final_states: impl IntoIterator<Item = i64>,
        transitions: impl IntoIterator<Item = (i64, i64, i64)>,
    ) -> Self {
        let mut automaton = AutomatonConstraintProto {
            starting_state,
            final_states: final_states.into_iter().collect(),
            exprs: self::exprs(exprs),
            ..Default::default()
        };
        for (tail, label, head) in transitions {
            automaton.transition_tail.push(tail);
            automaton.transition_label.push(label);
            automaton.transition_head.push(head);
        }
        Self::of(automaton)
    }
}
