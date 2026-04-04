//! Wallace tree topology.
//!
//! At each reduction level, every contiguous triple of terms is
//! compressed via a 3-to-2 adder and any leftover terms pass
//! through unchanged.  This is the depth-minimizing schedule
//! used in classical Wallace multipliers.

use crate::error::Error;
use crate::interpret::descriptor::CsaGrouping;
use crate::topology::Topology;

/// The Wallace tree topology.
///
/// If `k` terms are present at level `l`, the next level has
/// `2 * (k / 3) + (k % 3)` terms: each triple contributes one
/// carry-shifted-left-by-one and one sum, and any one or two
/// remaining terms pass through.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[must_use]
pub struct Wallace;

/// The next-level term count under the Wallace reduction rule.
const fn next_count(k: usize) -> usize {
    2 * (k / 3) + (k % 3)
}

/// Count reduction levels needed to drive `k` down to at most two.
const fn levels_to_pair(k: usize) -> usize {
    if k > 2 {
        1 + levels_to_pair(next_count(k))
    } else {
        0
    }
}

impl Topology for Wallace {
    fn level_count(&self, initial_term_count: usize) -> usize {
        levels_to_pair(initial_term_count)
    }

    fn term_count_at_level(&self, initial_term_count: usize, level: usize) -> usize {
        (0..level).fold(initial_term_count, |count, _| next_count(count))
    }

    fn level_grouping(
        &self,
        initial_term_count: usize,
        level: usize,
    ) -> Result<CsaGrouping, Error> {
        let total_levels = self.level_count(initial_term_count);
        if level < total_levels {
            let count = self.term_count_at_level(initial_term_count, level);
            let num_triples = count / 3;
            let triples: Vec<[usize; 3]> = (0..num_triples)
                .map(|i| [3 * i, 3 * i + 1, 3 * i + 2])
                .collect();
            let passthroughs: Vec<usize> = (num_triples * 3..count).collect();
            CsaGrouping::new(triples, passthroughs, count)
        } else {
            Err(Error::LevelOutOfBounds {
                level,
                count: total_levels,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_terms_reduces_in_four_levels() {
        assert_eq!(Wallace.level_count(9), 4);
    }

    #[test]
    fn two_terms_need_no_levels() {
        assert_eq!(Wallace.level_count(2), 0);
        assert_eq!(Wallace.level_count(1), 0);
        assert_eq!(Wallace.level_count(0), 0);
    }

    #[test]
    fn term_counts_follow_wallace_schedule() {
        let counts: Vec<usize> = (0..=4).map(|l| Wallace.term_count_at_level(9, l)).collect();
        assert_eq!(counts, vec![9, 6, 4, 3, 2]);
    }

    #[test]
    fn out_of_bounds_level_is_rejected() {
        let result = Wallace.level_grouping(9, 4);
        assert!(result.is_err());
    }

    #[test]
    fn grouping_sizes_are_consistent() -> Result<(), Error> {
        (0..4).try_for_each(|l| {
            let grouping = Wallace.level_grouping(9, l)?;
            let prev = Wallace.term_count_at_level(9, l);
            let next = Wallace.term_count_at_level(9, l + 1);
            assert_eq!(3 * grouping.triples().len() + grouping.passthroughs().len(), prev);
            assert_eq!(grouping.output_count(), next);
            Ok(())
        })
    }

    #[test]
    fn sixty_five_terms_reduces() {
        // Peak column count for 33-element schoolbook
        assert!(Wallace.level_count(65) >= 1);
        assert_eq!(Wallace.term_count_at_level(65, Wallace.level_count(65)), 2);
    }
}
