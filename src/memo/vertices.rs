// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Vertex-related MEMO data structures.
//!
//! This module computes all possible vertex configurations where curves
//! cross in the Venn diagram.

use crate::geometry::constants::{NCOLORS, NFACES, NPOINTS};
use crate::geometry::Vertex;

/// MEMO data for all possible vertices in the diagram.
///
/// # The 480-Vertex Insight (Mental Somersault Required!)
///
/// A monotone 6-Venn diagram has only **126 actual vertices** (by Euler's formula:
/// V - E + F = 2). However, we preallocate **480 possible vertices** to enable
/// efficient precomputation of vertex relationships.
///
/// **Why 480?** NPOINTS = 2^(NCOLORS-2) × NCOLORS × (NCOLORS-1) = 16 × 6 × 5 = 480
///
/// ## The Indexing Trick (This is the confusing part!)
///
/// **Warning**: Understanding this indexing scheme is like telling your left hand from
/// your right - some people find it super hard. Read slowly!
///
/// Vertices are indexed by `[outside_face][primary_color][secondary_color]` where:
///
/// - **`outside_face`**: The face ID (colorset bitmask) of colors that are
///   **outside BOTH** the primary and secondary curves. This is the "squinting"
///   you have to do - we identify the vertex not by what's crossing, but by
///   what's NOT crossing!
///
/// - **`primary_color`**: The curve that crosses from **inside** secondary to **outside**
///
/// - **`secondary_color`**: The curve that crosses from **outside** primary to **inside**
///
/// **IMPORTANT**: Primary vs secondary matters! Swapping them gives a **different vertex**
/// with opposite orientation. Both `[face][a][b]` and `[face][b][a]` exist as distinct
/// vertices representing the same geometric crossing point viewed from opposite directions.
///
/// Example: Vertex `[0b111100][0][1]` (face {c,d,e,f}, primary=a, secondary=b) is
/// different from `[0b111100][1][0]` (same face, primary=b, secondary=a).
///
/// ## Why This Weird Indexing?
///
/// This scheme allows us to:
/// 1. Uniquely identify all possible vertex configurations
/// 2. Precompute which vertices can exist in any valid diagram
/// 3. Store relationships between vertices, edges, and faces efficiently
/// 4. Enable O(1) lookups during constraint propagation
///
/// **Performance Impact**: This indexing scheme, combined with negative constraints,
/// is a key optimization that reduced search time from **~1 year CPU time (1999
/// implementation)** to **~5 seconds (2025 implementation)**. The seemingly wasteful
/// 70% memory overhead (480 slots, only 126 used) enables dramatic algorithmic speedups.
///
/// See [`vertex.c::getOrInitializeVertex()`](https://github.com/jeremycarroll/venntriangles/blob/main/vertex.c)
/// for the C implementation.
///
/// # Memory Layout
///
/// - `vertices`: **Heap-allocated** via Box
///   - Reason: Large 3D array (NFACES × NCOLORS × NCOLORS elements)
///   - Size: 64 × 6 × 6 × sizeof(Option<Vertex>) ≈ 147 KB for NCOLORS=6
///   - Box keeps only a pointer (8 bytes) on stack, array lives on heap
///   - Prevents stack overflow for large arrays
///   - ~70% of entries remain None in any specific solution
#[derive(Debug, Clone)]
pub struct VerticesMemo {
    /// All possible vertex configurations indexed by outside face and crossing orientation.
    ///
    /// **Indexing**: `vertices[outside_face][primary][secondary]`
    ///
    /// Where:
    /// - `outside_face` = face ID (bitmask) of colors outside BOTH crossing curves
    /// - `primary` = curve crossing from inside secondary to outside
    /// - `secondary` = curve crossing from outside primary to inside
    ///
    /// Returns `Some(Vertex)` if this configuration is valid, `None` otherwise.
    ///
    /// **Note**: Approximately 70% of entries are `None` in any specific diagram.
    /// Only ~126 of 480 possible vertices are actually used (Euler's formula: V - E + F = 2).
    ///
    /// **Heap-allocated** via Box - 3D array is too large for stack (147 KB).
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
