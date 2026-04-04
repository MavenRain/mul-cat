//! Graph morphism from reduction levels to reduction descriptors.
//!
//! This is the bridge between the abstract free category on
//! [`ReductionGraph`] and the concrete reduction descriptors: it
//! assigns each edge to the [`CsaGrouping`] dictated by the chosen
//! [`Topology`].  The universal property of the free category then
//! extends this assignment to a unique functor via [`interpret`].
//!
//! [`CsaGrouping`]: crate::interpret::descriptor::CsaGrouping

use crate::error::Error;
use crate::graph::reduction_graph::{ReductionGraph, full_reduction_path};
use crate::interpret::descriptor::ReductionDescriptor;
use crate::topology::Topology;
use comp_cat_rs::collapse::free_category::{
    Edge, GraphMorphism, Vertex, interpret,
};

/// Morphism that maps each reduction-graph edge to a single-level
/// [`ReductionDescriptor`] dictated by the supplied topology.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct ReductionMorphism<'a, T: Topology> {
    topology: &'a T,
    initial_term_count: usize,
}

impl<'a, T: Topology> ReductionMorphism<'a, T> {
    /// Create a new morphism.
    pub const fn new(topology: &'a T, initial_term_count: usize) -> Self {
        Self {
            topology,
            initial_term_count,
        }
    }

    /// The topology this morphism interprets.
    #[must_use]
    pub const fn topology(&self) -> &'a T {
        self.topology
    }

    /// The number of initial terms (Vertex 0 state size).
    #[must_use]
    pub const fn initial_term_count(&self) -> usize {
        self.initial_term_count
    }
}

impl<T: Topology> GraphMorphism<ReductionGraph> for ReductionMorphism<'_, T> {
    type Object = usize;
    type Morphism = ReductionDescriptor;

    fn map_vertex(&self, v: Vertex) -> usize {
        self.topology
            .term_count_at_level(self.initial_term_count, v.index())
    }

    fn map_edge(&self, e: Edge) -> ReductionDescriptor {
        self.topology
            .level_grouping(self.initial_term_count, e.index())
            .map_or(ReductionDescriptor::Identity, |g| {
                ReductionDescriptor::level(e.index(), g)
            })
    }
}

/// Build the composed [`ReductionDescriptor`] for the given topology
/// and initial term count via the free-category interpretation.
///
/// # Errors
///
/// Returns [`Error::Graph`] if path construction fails, or an error
/// from the topology when grouping cannot be produced.
pub fn build_reduction_descriptor<T: Topology>(
    topology: &T,
    initial_term_count: usize,
) -> Result<ReductionDescriptor, Error> {
    let level_count = topology.level_count(initial_term_count);
    let graph = ReductionGraph::new(level_count);
    let path = full_reduction_path(&graph)?;
    let morphism = ReductionMorphism::new(topology, initial_term_count);
    (0..level_count)
        .try_for_each(|i| topology.level_grouping(initial_term_count, i).map(|_| ()))?;
    Ok(interpret::<ReductionGraph, _>(
        &morphism,
        &path,
        |_| ReductionDescriptor::Identity,
        ReductionDescriptor::compose,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::wallace::Wallace;

    #[test]
    fn build_descriptor_for_nine_terms_has_four_levels() -> Result<(), Error> {
        let d = build_reduction_descriptor(&Wallace, 9)?;
        assert_eq!(d.level_count(), 4);
        Ok(())
    }

    #[test]
    fn build_descriptor_for_two_terms_is_identity() -> Result<(), Error> {
        let d = build_reduction_descriptor(&Wallace, 2)?;
        assert_eq!(d.level_count(), 0);
        Ok(())
    }
}
