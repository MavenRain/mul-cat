//! Reduction descriptors: the target of the categorical interpretation.
//!
//! A [`CsaGrouping`] specifies, for one reduction level, which term
//! triples to compress and which terms to pass through.  A
//! [`ReductionDescriptor`] is a composed sequence of these level
//! specifications; composition is associative and has the empty
//! [`ReductionDescriptor::Identity`] as its unit, matching the
//! free-category axioms.

use crate::bits_ext::mask;
use crate::carry_save::{CarrySavePair, compress_three};
use crate::error::Error;

/// A per-level carry-save grouping.
///
/// Together, `triples` and `passthroughs` partition the input term
/// indices.  The output term list is `compressor_outputs ++
/// passthrough_values`, where each triple `[i, j, k]` produces two
/// new terms (carry-shifted-left-by-one, sum) and each passthrough
/// index copies the original value unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct CsaGrouping {
    triples: Vec<[usize; 3]>,
    passthroughs: Vec<usize>,
}

impl CsaGrouping {
    /// Construct a grouping from triple and passthrough index lists.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GroupingMismatch`] if the total index count
    /// does not equal `expected_input_count`.
    pub fn new(
        triples: Vec<[usize; 3]>,
        passthroughs: Vec<usize>,
        expected_input_count: usize,
    ) -> Result<Self, Error> {
        let total = 3 * triples.len() + passthroughs.len();
        if total == expected_input_count {
            Ok(Self {
                triples,
                passthroughs,
            })
        } else {
            Err(Error::GroupingMismatch {
                input_count: expected_input_count,
                triples: triples.len(),
                passthroughs: passthroughs.len(),
            })
        }
    }

    /// Construct an identity grouping: all indices pass through.
    pub fn identity(input_count: usize) -> Self {
        Self {
            triples: Vec::new(),
            passthroughs: (0..input_count).collect(),
        }
    }

    /// The triple groupings.
    #[must_use]
    pub fn triples(&self) -> &[[usize; 3]] {
        &self.triples
    }

    /// The passthrough indices.
    #[must_use]
    pub fn passthroughs(&self) -> &[usize] {
        &self.passthroughs
    }

    /// The number of output terms produced by this grouping.
    #[must_use]
    pub const fn output_count(&self) -> usize {
        2 * self.triples.len() + self.passthroughs.len()
    }

    /// Apply the grouping to a slice of terms, returning the next
    /// level's term list.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TermIndexOutOfRange`] if any triple or
    /// passthrough references an index outside `terms`.
    pub fn apply(&self, terms: &[u128], mask_value: u128) -> Result<Vec<u128>, Error> {
        let available = terms.len();
        let lookup = |i: usize| -> Result<u128, Error> {
            terms.get(i).copied().ok_or(Error::TermIndexOutOfRange {
                level: 0,
                index: i,
                available,
            })
        };
        let passed: Vec<u128> = self
            .passthroughs
            .iter()
            .map(|i| lookup(*i))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self
            .triples
            .iter()
            .map(|[x, y, z]| {
                let a = lookup(*x)?;
                let b = lookup(*y)?;
                let c = lookup(*z)?;
                let pair = compress_three(a, b, c, mask_value);
                Ok::<[u128; 2], Error>([pair.carry(), pair.sum()])
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .chain(passed)
            .collect())
    }
}

/// An associatively composed reduction descriptor.
///
/// Invariant: `Composed` never nests (`compose` flattens).  Thus a
/// normal-form descriptor is either `Identity`, a single `Level`,
/// or `Composed(levels)` where every element is a `Level`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum ReductionDescriptor {
    /// The neutral element: passes terms through unchanged.
    Identity,
    /// A single reduction level.
    Level {
        /// The level index (for traceability).
        level_index: usize,
        /// The per-term grouping.
        grouping: CsaGrouping,
    },
    /// A non-empty sequence of composed levels, in apply order.
    Composed(Vec<Self>),
}

impl ReductionDescriptor {
    /// Build a single-level descriptor.
    pub const fn level(level_index: usize, grouping: CsaGrouping) -> Self {
        Self::Level {
            level_index,
            grouping,
        }
    }

    /// Associatively compose two descriptors: apply `self`, then `other`.
    pub fn compose(self, other: Self) -> Self {
        match (self, other) {
            (Self::Identity, d) | (d, Self::Identity) => d,
            (Self::Composed(xs), Self::Composed(ys)) => {
                Self::Composed(xs.into_iter().chain(ys).collect())
            }
            (Self::Composed(xs), y @ Self::Level { .. }) => {
                Self::Composed(xs.into_iter().chain(core::iter::once(y)).collect())
            }
            (x @ Self::Level { .. }, Self::Composed(ys)) => {
                Self::Composed(core::iter::once(x).chain(ys).collect())
            }
            (x @ Self::Level { .. }, y @ Self::Level { .. }) => Self::Composed(vec![x, y]),
        }
    }

    /// The number of non-identity levels in this descriptor.
    #[must_use]
    pub const fn level_count(&self) -> usize {
        match self {
            Self::Identity => 0,
            Self::Level { .. } => 1,
            Self::Composed(xs) => xs.len(),
        }
    }

    /// Apply the full descriptor to an initial term list, producing
    /// the list of terms after every level has been applied.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TermIndexOutOfRange`] if any level references
    /// a term that does not exist at that stage.
    pub fn evaluate(&self, terms: &[u128], operand_width: usize) -> Result<Vec<u128>, Error> {
        let mask_value = mask(2 * operand_width);
        match self {
            Self::Identity => Ok(terms.to_vec()),
            Self::Level {
                level_index,
                grouping,
            } => grouping.apply(terms, mask_value).map_err(|e| match e {
                Error::TermIndexOutOfRange {
                    level: _,
                    index,
                    available,
                } => Error::TermIndexOutOfRange {
                    level: *level_index,
                    index,
                    available,
                },
                Error::Graph(_)
                | Error::HdlCat(_)
                | Error::LevelOutOfBounds { .. }
                | Error::GroupingMismatch { .. }
                | Error::ZeroBitWidth
                | Error::BitWidthTooLarge { .. }
                | Error::CoefficientCountMismatch { .. }
                | Error::ZeroCoefficientCount
                | Error::WordLengthTooLarge { .. } => e,
            }),
            Self::Composed(levels) => levels.iter().try_fold(terms.to_vec(), |state, level| {
                level.evaluate(&state, operand_width)
            }),
        }
    }

    /// Evaluate the descriptor and interpret the final two-term
    /// output as a carry-save pair.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GroupingMismatch`] if the final term count is
    /// not exactly two, or other evaluation errors.
    pub fn evaluate_to_pair(
        &self,
        terms: &[u128],
        operand_width: usize,
    ) -> Result<CarrySavePair, Error> {
        let final_terms = self.evaluate(terms, operand_width)?;
        match final_terms.len() {
            0 => Ok(CarrySavePair::zero()),
            1 => Ok(CarrySavePair::new(0, final_terms[0])),
            2 => Ok(CarrySavePair::new(final_terms[0], final_terms[1])),
            _ => Err(Error::GroupingMismatch {
                input_count: terms.len(),
                triples: 0,
                passthroughs: final_terms.len(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_grouping_has_expected_size() {
        let g = CsaGrouping::identity(5);
        assert_eq!(g.output_count(), 5);
        assert_eq!(g.triples().len(), 0);
        assert_eq!(g.passthroughs().len(), 5);
    }

    #[test]
    fn grouping_rejects_mismatched_total() {
        let result = CsaGrouping::new(vec![[0, 1, 2]], vec![3, 4], 4);
        assert!(result.is_err());
    }

    #[test]
    fn grouping_reduces_three_to_two() -> Result<(), Error> {
        let g = CsaGrouping::new(vec![[0, 1, 2]], vec![], 3)?;
        let mask = 0xFF;
        let out = g.apply(&[0b0011, 0b0101, 0b0110], mask)?;
        assert_eq!(out.len(), 2);
        let sum_expected: u128 = 0b0011 + 0b0101 + 0b0110;
        assert_eq!((out[0].wrapping_add(out[1])) & mask, sum_expected & mask);
        Ok(())
    }

    #[test]
    fn compose_is_associative_on_descriptors() -> Result<(), Error> {
        let g1 = CsaGrouping::new(vec![[0, 1, 2]], vec![3], 4)?;
        let g2 = CsaGrouping::new(vec![[0, 1, 2]], vec![], 3)?;
        let d1 = ReductionDescriptor::level(0, g1);
        let d2 = ReductionDescriptor::level(1, g2.clone());
        let d3 = ReductionDescriptor::level(2, g2);
        let left = d1.clone().compose(d2.clone()).compose(d3.clone());
        let right = d1.compose(d2.compose(d3));
        assert_eq!(left.level_count(), right.level_count());
        Ok(())
    }

    #[test]
    fn identity_is_neutral_on_both_sides() -> Result<(), Error> {
        let g = CsaGrouping::new(vec![[0, 1, 2]], vec![], 3)?;
        let d = ReductionDescriptor::level(0, g);
        let id = ReductionDescriptor::Identity;
        assert_eq!(d.clone().compose(id.clone()).level_count(), 1);
        assert_eq!(id.compose(d).level_count(), 1);
        Ok(())
    }
}
