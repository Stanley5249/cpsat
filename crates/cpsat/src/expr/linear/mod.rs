//! Integer linear expressions, like Python's `LinearExpr`.

mod ops;

use crate::expr::{BoolVar, IntVar};
use crate::proto::LinearExpressionProto;

/// `sum(coeff * var) + offset` with `i64` coefficients.
///
/// Arithmetic operators build one from variables, literals, and constants,
/// and the comparison methods turn it into a linear
/// [`Constraint`](crate::prelude::Constraint). A negated
/// literal `!b` counts as `1 - b`.
///
/// The arithmetic panics when a coefficient or the offset overflows `i64`,
/// and leaves the expression unchanged. Terms of one variable merge only when
/// the expression is written into a constraint, an objective, or an interval,
/// which panics if a merged coefficient overflows. Python wraps some overflows
/// silently and fails on others. `eq`, `lt`, and `le` panic on a constant
/// `i64::MIN`, and `eq`, `ge`, and `gt` on a constant `i64::MAX`, because
/// domains read these limits as infinity. Python raises on these limits too.
///
/// ```
/// use cpsat::prelude::*;
///
/// // From `cp_sat_example.py`.
/// let mut model = CpModel::new();
/// let [x, y, z] = std::array::from_fn(|_| model.new_var(0..=50));
/// model.add((2 * x + 7 * y + 3 * z).le(50));
/// model.add((3 * x - 5 * y + 7 * z).le(45));
/// model.add((5 * x + 2 * y - 6 * z).le(37));
/// let objective: LinearExpr = [2 * x, 2 * y, 3 * z].into_iter().sum();
/// model.maximize(objective);
/// ```
#[derive(Clone, Debug, Default)]
pub struct LinearExpr {
    terms: Vec<(i32, i64)>,
    offset: i64,
}

const OVERFLOW: &str = "linear expression overflows i64";

impl LinearExpr {
    /// The expression with no terms and offset `value`.
    #[must_use]
    pub fn constant(value: i64) -> Self {
        Self {
            terms: Vec::new(),
            offset: value,
        }
    }

    fn term(var: i32, coeff: i64, offset: i64) -> Self {
        Self {
            terms: vec![(var, coeff)],
            offset,
        }
    }

    /// Adds `factor * other` to this expression. It checks every product
    /// before the first change, so an overflow leaves the expression intact.
    fn add_scaled(&mut self, mut other: Self, factor: i64) {
        let offset = self.scaled_offset(&other, factor);
        if self.terms.is_empty() && other.terms.capacity() > self.terms.capacity() {
            // Takes the larger buffer of the other expression.
            for (_, coeff) in &mut other.terms {
                *coeff *= factor;
            }
            self.terms = other.terms;
        } else {
            self.extend_scaled(&other.terms, factor);
        }
        self.offset = offset;
    }

    /// Adds `factor * other` without cloning it, like `add_scaled`.
    fn add_scaled_ref(&mut self, other: &Self, factor: i64) {
        self.offset = self.scaled_offset(other, factor);
        self.extend_scaled(&other.terms, factor);
    }

    /// The offset after adding `factor * other`, checking every product.
    fn scaled_offset(&self, other: &Self, factor: i64) -> i64 {
        other
            .offset
            .checked_mul(factor)
            .and_then(|scaled| self.offset.checked_add(scaled))
            // Multiplying by 1 cannot overflow, so adding skips the scan.
            .filter(|_| factor == 1 || scales(&other.terms, factor))
            .expect(OVERFLOW)
    }

    fn extend_scaled(&mut self, terms: &[(i32, i64)], factor: i64) {
        self.terms
            .extend(terms.iter().map(|&(var, coeff)| (var, coeff * factor)));
    }

    /// Adds `coeff * var + offset`, checking the offset before the change.
    fn push_term(&mut self, var: i32, coeff: i64, offset: i64) {
        self.offset = self.offset.checked_add(offset).expect(OVERFLOW);
        self.terms.push((var, coeff));
    }

    /// The terms, sorted by variable, merged, and without zero coefficients,
    /// as Python writes them into a constraint, and the offset.
    ///
    /// # Panics
    ///
    /// If a merged coefficient overflows `i64`.
    pub(crate) fn into_terms(self) -> (Vec<i32>, Vec<i64>, i64) {
        let mut terms = self.terms;
        terms.sort_unstable_by_key(|&(var, _)| var);
        // Summing in i128 makes the merge independent of the order of equal
        // keys, which the unstable sort leaves open.
        let (vars, coeffs) = terms
            .chunk_by(|a, b| a.0 == b.0)
            .filter_map(|run| {
                let sum: i128 = run.iter().map(|&(_, coeff)| i128::from(coeff)).sum();
                let coeff = i64::try_from(sum).expect(OVERFLOW);
                (coeff != 0).then_some((run[0].0, coeff))
            })
            .unzip();
        (vars, coeffs, self.offset)
    }

    /// The terms in insertion order, unsorted and unmerged, and the offset.
    pub(crate) fn into_raw_parts(self) -> (Vec<(i32, i64)>, i64) {
        (self.terms, self.offset)
    }

    /// The terms in insertion order and the offset, borrowed.
    pub(crate) fn raw_parts(&self) -> (&[(i32, i64)], i64) {
        (&self.terms, self.offset)
    }

    /// The value if the expression has no variables.
    pub(crate) fn constant_value(&self) -> Option<i64> {
        self.terms.is_empty().then_some(self.offset)
    }

    /// The value under `solution`, indexed by variable.
    ///
    /// # Panics
    ///
    /// If a variable is out of range, like indexing a slice, if the value
    /// overflows `i64`, or if a partial sum overflows `i128`, which takes
    /// several products near `2^126`.
    pub(crate) fn value_in(&self, solution: &[i64]) -> i64 {
        // Each product fits i128, so terms that cancel, such as those of
        // `4 * x - 3 * x`, do not overflow on the way to the result unless
        // the products themselves approach the i128 limit.
        let sum = self
            .terms
            .iter()
            .fold(i128::from(self.offset), |sum, &(var, coeff)| {
                let value =
                    solution[usize::try_from(var).expect("a variable index is non-negative")];
                sum.checked_add(i128::from(coeff) * i128::from(value))
                    .expect(OVERFLOW)
            });
        i64::try_from(sum).expect(OVERFLOW)
    }
}

/// Whether every coefficient times `factor` fits `i64`.
fn scales(terms: &[(i32, i64)], factor: i64) -> bool {
    terms
        .iter()
        .all(|&(_, coeff)| coeff.checked_mul(factor).is_some())
}

impl LinearExpr {
    /// The message in the canonical form that Python writes, with terms
    /// sorted and merged. A method instead of `From`, because merging can
    /// fail.
    ///
    /// # Panics
    ///
    /// If a merged coefficient overflows `i64`.
    #[must_use]
    pub fn into_proto(self) -> LinearExpressionProto {
        let (vars, coeffs, offset) = self.into_terms();
        LinearExpressionProto {
            vars,
            coeffs,
            offset,
            ..Default::default()
        }
    }

    /// The expression that `proto` writes, or `None` if its `vars` and
    /// `coeffs` differ in length, which `validate` rejects.
    pub(crate) fn from_proto(proto: &LinearExpressionProto) -> Option<Self> {
        (proto.vars.len() == proto.coeffs.len()).then(|| Self {
            terms: proto
                .vars
                .iter()
                .copied()
                .zip(proto.coeffs.iter().copied())
                .collect(),
            offset: proto.offset,
        })
    }
}

impl From<&Self> for LinearExpr {
    /// Clones the expression, so `solution.value(&expr)` keeps `expr`.
    fn from(expr: &Self) -> Self {
        expr.clone()
    }
}

impl From<i64> for LinearExpr {
    fn from(value: i64) -> Self {
        Self::constant(value)
    }
}

impl From<IntVar> for LinearExpr {
    fn from(var: IntVar) -> Self {
        Self::term(var.0, 1, 0)
    }
}

impl From<BoolVar> for LinearExpr {
    fn from(literal: BoolVar) -> Self {
        if literal.is_negated() {
            Self::term(literal.to_int_var().0, -1, 1)
        } else {
            Self::term(literal.0, 1, 0)
        }
    }
}
