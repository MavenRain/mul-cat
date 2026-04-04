//! Linear reduction topology.
//!
//! At each level, exactly one triple of terms is compressed and
//! every other term passes through.  This is the latency-maximising
//! schedule; it uses the smallest number of compressors active at
//! any instant and so is useful as a reference model.

use crate::error::Error;
use crate::interpret::descriptor::CsaGrouping;
use crate::topology::Topology;

/// The linear reduction topology: one 3-to-2 compression per level.
///
/// If `k` terms are present at level `l` (for `k >= 3`), the next
/// level has `k - 1` terms: the first three are compressed into
/// two, and the remaining `k - 3` pass through.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[must_use]
pub struct Linear;

impl Topology for Linear {
    fn level_count(&self, initial_term_count: usize) -> usize {
        if initial_term_count >= 3 {
            initial_term_count - 2
        } else {
            0
        }
    }

    fn term_count_at_level(&self, initial_term_count: usize, level: usize) -> usize {
        initial_term_count
            .checked_sub(level)
            .map_or(2, |c| if c >= 2 { c } else { 2 })
    }

    fn level_grouping(
        &self,
        initial_term_count: usize,
        level: usize,
    ) -> Result<CsaGrouping, Error> {
        let total_levels = self.level_count(initial_term_count);
        if level < total_levels {
            let count = self.term_count_at_level(initial_term_count, level);
            CsaGrouping::new(vec![[0, 1, 2]], (3..count).collect(), count)
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
    fn nine_terms_reduces_in_seven_levels() {
        assert_eq!(Linear.level_count(9), 7);
    }

    #[test]
    fn two_terms_need_no_levels() {
        assert_eq!(Linear.level_count(2), 0);
        assert_eq!(Linear.level_count(1), 0);
        assert_eq!(Linear.level_count(0), 0);
    }

    #[test]
    fn term_counts_decrease_by_one_per_level() {
        let counts: Vec<usize> = (0..=7).map(|l| Linear.term_count_at_level(9, l)).collect();
        assert_eq!(counts, vec![9, 8, 7, 6, 5, 4, 3, 2]);
    }

    #[test]
    fn grouping_always_has_single_triple() -> Result<(), Error> {
        (0..7).try_for_each(|l| {
            let g = Linear.level_grouping(9, l)?;
            assert_eq!(g.triples().len(), 1);
            Ok(())
        })
    }

    #[test]
    fn out_of_bounds_level_is_rejected() {
        assert!(Linear.level_grouping(9, 7).is_err());
    }
}
