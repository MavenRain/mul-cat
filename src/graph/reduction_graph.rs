//! Linear-chain graph for carry-save tree reduction.
//!
//! Vertex `k` represents the state of the partial-product list after
//! `k` reduction rounds.  Edge `k` represents the `k`-th round of
//! 3-to-2 compression.  The graph has `level_count + 1` vertices
//! and `level_count` edges; the only morphism in the free category
//! from `Vertex(0)` to `Vertex(level_count)` is the full composed
//! reduction path.

use comp_cat_rs::collapse::free_category::{
    Edge, FreeCategoryError, Graph, Path, Vertex,
};

/// The linear-chain graph for a carry-save reduction with a fixed
/// number of levels.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct ReductionGraph {
    level_count: usize,
}

impl ReductionGraph {
    /// Construct a reduction graph with the given number of levels.
    pub const fn new(level_count: usize) -> Self {
        Self { level_count }
    }

    /// The number of reduction levels (edges).
    #[must_use]
    pub const fn level_count(self) -> usize {
        self.level_count
    }
}

impl Graph for ReductionGraph {
    fn vertex_count(&self) -> usize {
        self.level_count + 1
    }

    fn edge_count(&self) -> usize {
        self.level_count
    }

    fn source(&self, edge: Edge) -> Result<Vertex, FreeCategoryError> {
        if edge.index() < self.level_count {
            Ok(Vertex::new(edge.index()))
        } else {
            Err(FreeCategoryError::EdgeOutOfBounds {
                edge,
                count: self.level_count,
            })
        }
    }

    fn target(&self, edge: Edge) -> Result<Vertex, FreeCategoryError> {
        if edge.index() < self.level_count {
            Ok(Vertex::new(edge.index() + 1))
        } else {
            Err(FreeCategoryError::EdgeOutOfBounds {
                edge,
                count: self.level_count,
            })
        }
    }
}

/// Build the full path through every edge of the reduction graph,
/// from `Vertex(0)` to `Vertex(level_count)`.
///
/// # Errors
///
/// Returns [`FreeCategoryError`] if singleton path construction or
/// composition fails (should not occur for a well-formed linear chain).
pub fn full_reduction_path(graph: &ReductionGraph) -> Result<Path, FreeCategoryError> {
    (0..graph.level_count())
        .try_fold(Path::identity(Vertex::new(0)), |acc, i| {
            Path::singleton(graph, Edge::new(i)).and_then(|edge_path| acc.compose(edge_path))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_vertex_and_edge_counts() {
        let g = ReductionGraph::new(4);
        assert_eq!(g.vertex_count(), 5);
        assert_eq!(g.edge_count(), 4);
    }

    #[test]
    fn source_and_target_wire_linear_chain() -> Result<(), FreeCategoryError> {
        let g = ReductionGraph::new(3);
        (0..3).try_for_each(|i| {
            let e = Edge::new(i);
            assert_eq!(g.source(e)?, Vertex::new(i));
            assert_eq!(g.target(e)?, Vertex::new(i + 1));
            Ok::<(), FreeCategoryError>(())
        })
    }

    #[test]
    fn out_of_bounds_edge_is_rejected() {
        let g = ReductionGraph::new(2);
        assert!(g.source(Edge::new(2)).is_err());
        assert!(g.target(Edge::new(5)).is_err());
    }

    #[test]
    fn full_path_traverses_every_edge() -> Result<(), FreeCategoryError> {
        let g = ReductionGraph::new(4);
        let path = full_reduction_path(&g)?;
        assert_eq!(path.source(), Vertex::new(0));
        assert_eq!(path.target(), Vertex::new(4));
        assert_eq!(path.len(), 4);
        Ok(())
    }

    #[test]
    fn zero_level_graph_has_identity_path() -> Result<(), FreeCategoryError> {
        let g = ReductionGraph::new(0);
        let path = full_reduction_path(&g)?;
        assert!(path.is_identity());
        assert_eq!(path.source(), Vertex::new(0));
        assert_eq!(path.target(), Vertex::new(0));
        Ok(())
    }
}
