//! Circuit and multiple-circuit constraints over a graph of literal arcs.

use super::Constraint;
use crate::proto::{CircuitConstraintProto, RoutesConstraintProto};
use crate::variable::BoolVar;

/// A Hamiltonian circuit through the arcs whose literals are true, like
/// Python's `add_circuit`. A node outside the circuit needs a true self-loop.
///
/// ```
/// use cpsat::prelude::*;
///
/// let mut model = Model::new();
/// let [a, b, c] = [(); 3].map(|()| model.bool_var());
/// model.add(Circuit::new().arc(0, 1, a).arc(1, 2, b).arc(2, 0, c));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Circuit(CircuitConstraintProto);

impl Circuit {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The arc `tail -> head`, selected when `literal` is true.
    #[must_use]
    pub fn arc(mut self, tail: i32, head: i32, literal: BoolVar) -> Self {
        self.0.tails.push(tail);
        self.0.heads.push(head);
        self.0.literals.push(literal.0);
        self
    }
}

impl FromIterator<(i32, i32, BoolVar)> for Circuit {
    fn from_iter<I: IntoIterator<Item = (i32, i32, BoolVar)>>(arcs: I) -> Self {
        arcs.into_iter()
            .fold(Self::new(), |circuit, (tail, head, literal)| {
                circuit.arc(tail, head, literal)
            })
    }
}

impl From<Circuit> for Constraint {
    fn from(circuit: Circuit) -> Self {
        Self::of(circuit.0)
    }
}

/// Circuits that all pass through node 0, like Python's
/// `add_multiple_circuit`, also called the vehicle routing constraint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MultipleCircuit(RoutesConstraintProto);

impl MultipleCircuit {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The arc `tail -> head`, selected when `literal` is true.
    #[must_use]
    pub fn arc(mut self, tail: i32, head: i32, literal: BoolVar) -> Self {
        self.0.tails.push(tail);
        self.0.heads.push(head);
        self.0.literals.push(literal.0);
        self
    }
}

impl FromIterator<(i32, i32, BoolVar)> for MultipleCircuit {
    fn from_iter<I: IntoIterator<Item = (i32, i32, BoolVar)>>(arcs: I) -> Self {
        arcs.into_iter()
            .fold(Self::new(), |circuit, (tail, head, literal)| {
                circuit.arc(tail, head, literal)
            })
    }
}

impl From<MultipleCircuit> for Constraint {
    fn from(circuit: MultipleCircuit) -> Self {
        Self::of(circuit.0)
    }
}
