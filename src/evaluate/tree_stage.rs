//! Carry-save tree stage: reduce partial products to a carry-save pair.
//!
//! This is the categorical heart of the multiplier: a
//! [`ReductionDescriptor`] composed via the free-category
//! `interpret` function over a
//! [`ReductionGraph`](crate::graph::reduction_graph::ReductionGraph)
//! is applied to the list of Booth partial products, yielding a
//! single [`CarrySavePair`].

use crate::carry_save::CarrySavePair;
use crate::error::Error;
use crate::interpret::descriptor::ReductionDescriptor;
use crate::interpret::morphism::build_reduction_descriptor;
use crate::topology::Topology;

/// Reduce `terms` to a carry-save pair via the specified topology.
///
/// Under the hood this constructs the
/// [`ReductionGraph`](crate::graph::reduction_graph::ReductionGraph),
/// interprets the full path through the free category, and applies
/// the resulting [`ReductionDescriptor`] to the term list.
///
/// # Errors
///
/// Returns any error raised while constructing the descriptor or
/// applying it (e.g. out-of-range term indices, grouping mismatches).
pub fn reduce_terms<T: Topology>(
    topology: &T,
    terms: &[u128],
    operand_width: usize,
) -> Result<CarrySavePair, Error> {
    let descriptor = build_reduction_descriptor(topology, terms.len())?;
    descriptor.evaluate_to_pair(terms, operand_width)
}

/// Apply a prebuilt reduction descriptor to a term list.
///
/// Useful when the same topology is applied to many inputs of the
/// same size: build the descriptor once and reuse it.
///
/// # Errors
///
/// Returns [`Error::GroupingMismatch`] if the final term count is
/// not two, or propagation errors from descriptor evaluation.
pub fn reduce_with_descriptor(
    descriptor: &ReductionDescriptor,
    terms: &[u128],
    operand_width: usize,
) -> Result<CarrySavePair, Error> {
    descriptor.evaluate_to_pair(terms, operand_width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bits_ext::mask;
    use crate::topology::linear::Linear;
    use crate::topology::wallace::Wallace;

    #[test]
    fn wallace_reduces_simple_terms_correctly() -> Result<(), Error> {
        let terms = [1_u128, 2, 3, 4, 5];
        let pair = reduce_terms(&Wallace, &terms, 8)?;
        assert_eq!(pair.resolve(mask(16)), 15);
        Ok(())
    }

    #[test]
    fn linear_reduces_simple_terms_correctly() -> Result<(), Error> {
        let terms = [1_u128, 2, 3, 4, 5];
        let pair = reduce_terms(&Linear, &terms, 8)?;
        assert_eq!(pair.resolve(mask(16)), 15);
        Ok(())
    }

    #[test]
    fn wallace_and_linear_agree() -> Result<(), Error> {
        let terms: Vec<u128> = (1_u128..=9).collect();
        let wallace_pair = reduce_terms(&Wallace, &terms, 8)?;
        let linear_pair = reduce_terms(&Linear, &terms, 8)?;
        let m = mask(16);
        assert_eq!(wallace_pair.resolve(m), linear_pair.resolve(m));
        assert_eq!(wallace_pair.resolve(m), 45);
        Ok(())
    }

    #[test]
    fn two_terms_yield_those_terms() -> Result<(), Error> {
        let pair = reduce_terms(&Wallace, &[7, 11], 8)?;
        assert_eq!(pair.resolve(mask(16)), 18);
        Ok(())
    }

    #[test]
    fn single_term_pads_with_zero() -> Result<(), Error> {
        let pair = reduce_terms(&Wallace, &[42], 8)?;
        assert_eq!(pair.resolve(mask(16)), 42);
        Ok(())
    }

    #[test]
    fn empty_terms_yield_zero() -> Result<(), Error> {
        let pair = reduce_terms(&Wallace, &[], 8)?;
        assert_eq!(pair.resolve(mask(16)), 0);
        Ok(())
    }
}
