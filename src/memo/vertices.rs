// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Vertex-related MEMO data structures.
//!
//! This module computes all possible vertex configurations where curves
//! cross in the Venn diagram.

use crate::geometry::constants::{NCOLORS, NFACES, NPOINTS};
use crate::geometry::Vertex;

/// MEMO data for all possible vertices in the diagram.
///
/// Vertices represent crossing points between curves. There are NPOINTS
/// possible vertices (480 for NCOLORS=6).
///
/// # Indexing
///
/// Vertices are indexed by `[face_id][color_a][color_b]`, representing
/// the vertex at the edge of `face_id` where colors `color_a` and `color_b`
/// cross.
#[derive(Debug, Clone)]
pub struct VerticesMemo {
    /// All possible vertex configurations.
    ///
    /// `vertices[face_id][color_a][color_b]` = vertex where colors a and b
    /// cross at the boundary of face face_id.
    ///
    /// None if this configuration is impossible.
    pub vertices: Box<[[[Option<Vertex>; NCOLORS]; NCOLORS]; NFACES]>,
}

impl VerticesMemo {
    /// Initialize all vertex MEMO data.
    ///
    /// This computes all NPOINTS possible vertex configurations.
    ///
    /// # Algorithm
    ///
    /// For each face, for each pair of colors (a, b):
    /// 1. Check if colors a and b both bound this face
    /// 2. If so, compute the 4 incoming edges at this vertex
    /// 3. Set primary/secondary colors and crossing orientation
    ///
    /// This ports the logic from `vertex.c::initializePoints()`.
    pub fn initialize() -> Self {
        eprintln!("[VerticesMemo] Initializing {} vertices...", NPOINTS);

        // Allocate vertex array (Box to keep it on heap)
        let vertices = Box::new([[[None; NCOLORS]; NCOLORS]; NFACES]);

        // TODO: Compute actual vertex configurations
        // This requires:
        // 1. Port initializeVertexIncomingEdge() logic
        // 2. Determine which 4 edges meet at each vertex
        // 3. Set primary/secondary colors
        //
        // For now, just return empty array

        eprintln!("[VerticesMemo] WARNING: Vertex computation not yet implemented (TODO)");

        Self { vertices }
    }

    /// Get a vertex configuration.
    ///
    /// # Arguments
    ///
    /// * `face_id` - The face at whose boundary the vertex lies
    /// * `color_a` - First color crossing at this vertex
    /// * `color_b` - Second color crossing at this vertex
    ///
    /// # Returns
    ///
    /// The vertex configuration if it exists, None otherwise.
    #[inline]
    pub fn get_vertex(&self, face_id: usize, color_a: usize, color_b: usize) -> Option<&Vertex> {
        self.vertices[face_id][color_a][color_b].as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertices_memo_initialization() {
        let memo = VerticesMemo::initialize();

        // Should have allocated the full array
        assert_eq!(memo.vertices.len(), NFACES);
        assert_eq!(memo.vertices[0].len(), NCOLORS);
        assert_eq!(memo.vertices[0][0].len(), NCOLORS);
    }

    #[test]
    fn test_get_vertex_out_of_bounds() {
        let memo = VerticesMemo::initialize();

        // Should return None for most vertices (since we haven't computed them yet)
        assert_eq!(memo.get_vertex(0, 0, 1), None);
    }
}
