//! Global constraints over lists of expressions.

use buffa::MessageField;

use super::{Constraint, exprs, variables};
use crate::expr::{IntVar, LinearExpr};
use crate::proto::{
    AllDifferentConstraintProto, AutomatonConstraintProto, ElementConstraintProto,
    InverseConstraintProto, TableConstraintProto,
};

impl Constraint {
    /// Every expression takes a different value. CP-SAT accepts affine
    /// expressions `a * x + b` here.
    #[doc(alias("add_all_different", "AddAllDifferent"))]
    #[must_use]
    pub fn all_different(exprs: impl IntoIterator<Item = impl Into<LinearExpr>>) -> Self {
        Self::of(AllDifferentConstraintProto {
            exprs: self::exprs(exprs),
            ..Default::default()
        })
    }

    /// `exprs[index] == target`. CP-SAT accepts affine expressions here.
    #[doc(alias("add_element", "AddElement", "AddVariableElement"))]
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

    /// `direct[i] == j` exactly when `inverse[j] == i`.
    #[doc(alias("add_inverse", "AddInverseConstraint"))]
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

    /// The expressions equal one of the rows. Each row lists one value per
    /// expression.
    ///
    /// # Panics
    ///
    /// If a row's length differs from the number of expressions, as Python
    /// raises, because the flat schema would split the values into other
    /// rows without an error.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// // From `assignment_groups_sat.py`: which pairs of four workers work.
    /// let mut model = Model::new();
    /// let work = [(); 4].map(|()| model.new_var(Bool));
    /// let group = [
    ///     [0, 0, 1, 1],
    ///     [0, 1, 0, 1],
    ///     [0, 1, 1, 0],
    ///     [1, 1, 0, 0],
    ///     [1, 0, 1, 0],
    /// ];
    /// model.add(Constraint::allowed_assignments(work, group));
    /// ```
    #[doc(alias("add_allowed_assignments", "AddAllowedAssignments"))]
    #[must_use]
    pub fn allowed_assignments(
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
        rows: impl IntoIterator<Item = impl IntoIterator<Item = i64>>,
    ) -> Self {
        Self::of(table(exprs, rows))
    }

    /// The expressions equal none of the rows.
    ///
    /// # Panics
    ///
    /// If a row's length differs from the number of expressions, as in
    /// [`allowed_assignments`](Self::allowed_assignments).
    #[doc(alias("add_forbidden_assignments", "AddForbiddenAssignments"))]
    #[must_use]
    pub fn forbidden_assignments(
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
        rows: impl IntoIterator<Item = impl IntoIterator<Item = i64>>,
    ) -> Self {
        Self::of(TableConstraintProto {
            negated: true,
            ..table(exprs, rows)
        })
    }

    /// The expressions spell a word that the automaton accepts. Each transition
    /// is `(tail, label, head)`, in the order of Python's transition triples.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// let mut model = Model::new();
    /// let labels = [(); 3].map(|()| model.new_var(0..=1));
    /// // Accepts words without two consecutive 1s.
    /// let transitions = [(0, 0, 0), (0, 1, 1), (1, 0, 0)];
    /// model.add(Constraint::automaton(labels, 0, [0, 1], transitions));
    /// ```
    #[doc(alias("add_automaton", "AddAutomaton"))]
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

fn table(
    exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
    rows: impl IntoIterator<Item = impl IntoIterator<Item = i64>>,
) -> TableConstraintProto {
    let exprs = self::exprs(exprs);
    let mut values = Vec::new();
    for row in rows {
        let start = values.len();
        values.extend(row);
        let len = values.len() - start;
        assert_eq!(len, exprs.len(), "a table row has one value per expression");
    }
    TableConstraintProto {
        exprs,
        values,
        ..Default::default()
    }
}
