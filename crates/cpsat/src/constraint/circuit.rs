//! Circuit and multiple-circuit constraints over a graph of literal arcs.

use super::{Constraint, literals};
use crate::proto::{CircuitConstraintProto, RoutesConstraintProto};
use crate::variable::BoolVar;

/// Splits `(tail, head, literal)` arcs into the schema's parallel lists.
fn arcs(arcs: impl IntoIterator<Item = (i32, i32, BoolVar)>) -> (Vec<i32>, Vec<i32>, Vec<i32>) {
    let mut tails = Vec::new();
    let mut heads = Vec::new();
    let mut selected = Vec::new();
    for (tail, head, literal) in arcs {
        tails.push(tail);
        heads.push(head);
        selected.push(literal);
    }
    (tails, heads, literals(selected))
}

impl Constraint {
    /// A Hamiltonian circuit through the arcs whose literals are true, like
    /// Python's `add_circuit`. Each arc is `(tail, head, literal)`, and a node
    /// outside the circuit needs a true self-loop.
    ///
    /// ```
    /// use cpsat::prelude::*;
    ///
    /// let mut model = Model::new();
    /// let [a, b, c] = [(); 3].map(|()| model.bool_var());
    /// model.add(Constraint::circuit([(0, 1, a), (1, 2, b), (2, 0, c)]));
    /// ```
    #[must_use]
    pub fn circuit(arcs: impl IntoIterator<Item = (i32, i32, BoolVar)>) -> Self {
        let (tails, heads, literals) = self::arcs(arcs);
        Self::of(CircuitConstraintProto {
            tails,
            heads,
            literals,
            ..Default::default()
        })
    }

    /// Circuits that all pass through node 0, like Python's
    /// `add_multiple_circuit`, also called the vehicle routing constraint.
    #[must_use]
    pub fn multiple_circuit(arcs: impl IntoIterator<Item = (i32, i32, BoolVar)>) -> Self {
        let (tails, heads, literals) = self::arcs(arcs);
        Self::of(RoutesConstraintProto {
            tails,
            heads,
            literals,
            ..Default::default()
        })
    }
}
