//! Reduction tree topologies.
//!
//! A [`Topology`] parameterizes the carry-save reduction tree:
//! given an initial term count, it determines how many levels are
//! required and, at each level, which groups of three terms to
//! compress and which terms to pass through.

use crate::error::Error;
use crate::interpret::descriptor::CsaGrouping;

pub mod linear;
pub mod wallace;

/// A reduction tree topology.
///
/// Implementations describe a schedule for reducing `initial_term_count`
/// terms to a carry-save pair (two terms) via repeated 3-to-2
/// compression.  Each level's [`CsaGrouping`] partitions the current
/// term list into triples (to compress) and passthroughs (to carry
/// forward unchanged).
pub trait Topology {
    /// The number of reduction levels required to collapse
    /// `initial_term_count` terms to a carry-save pair.
    fn level_count(&self, initial_term_count: usize) -> usize;

    /// The number of terms present at the start of reduction level
    /// `level`, given an initial term count.
    ///
    /// By convention, `term_count_at_level(k, 0) == k`.
    fn term_count_at_level(&self, initial_term_count: usize, level: usize) -> usize;

    /// The per-level [`CsaGrouping`] that reduces the term list at
    /// `level` to the term list at `level + 1`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::LevelOutOfBounds`] if `level` is beyond the
    /// topology's reduction schedule.
    fn level_grouping(
        &self,
        initial_term_count: usize,
        level: usize,
    ) -> Result<CsaGrouping, Error>;
}
