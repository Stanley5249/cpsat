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
}

/// The expressions must equal one of the rows, or none of them, like Python's
/// `add_allowed_assignments` and `add_forbidden_assignments`.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let x = model.int_var(0..=5);
/// let y = model.int_var(0..=5);
/// model.add(Table::allowed([x, y]).row([1, 2]).row([3, 4]));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table(TableConstraintProto);

impl Table {
    #[must_use]
    pub fn allowed(exprs: impl IntoIterator<Item = impl Into<LinearExpr>>) -> Self {
        Self(TableConstraintProto {
            exprs: self::exprs(exprs),
            ..Default::default()
        })
    }

    #[must_use]
    pub fn forbidden(exprs: impl IntoIterator<Item = impl Into<LinearExpr>>) -> Self {
        Self(TableConstraintProto {
            negated: true,
            ..Self::allowed(exprs).0
        })
    }

    /// One tuple of values, in the order of the expressions. The validator
    /// rejects a row of the wrong length.
    #[must_use]
    pub fn row(mut self, values: impl IntoIterator<Item = i64>) -> Self {
        self.0.values.extend(values);
        self
    }
}

impl From<Table> for Constraint {
    fn from(table: Table) -> Self {
        Self::of(table.0)
    }
}

/// The expressions spell a word that the automaton accepts, like Python's
/// `add_automaton`.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let labels = [(); 3].map(|()| model.int_var(0..=1));
/// // Accepts words without two consecutive 1s.
/// model.add(
///     Automaton::new(labels, 0, [0, 1])
///         .transition(0, 0, 0)
///         .transition(0, 1, 1)
///         .transition(1, 0, 0),
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Automaton(AutomatonConstraintProto);

impl Automaton {
    #[must_use]
    pub fn new(
        exprs: impl IntoIterator<Item = impl Into<LinearExpr>>,
        starting_state: i64,
        final_states: impl IntoIterator<Item = i64>,
    ) -> Self {
        Self(AutomatonConstraintProto {
            starting_state,
            final_states: final_states.into_iter().collect(),
            exprs: self::exprs(exprs),
            ..Default::default()
        })
    }

    /// Moves from state `tail` to state `head` on the value `label`, in the
    /// order of Python's transition triples.
    #[must_use]
    pub fn transition(mut self, tail: i64, label: i64, head: i64) -> Self {
        self.0.transition_tail.push(tail);
        self.0.transition_label.push(label);
        self.0.transition_head.push(head);
        self
    }
}

impl From<Automaton> for Constraint {
    fn from(automaton: Automaton) -> Self {
        Self::of(automaton.0)
    }
}
