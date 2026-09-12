// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Immutable precomputed data (Tier 1: MEMO).

use crate::memo::{CyclesArray, CyclesMemo, FacesMemo, VerticesMemo};

/// Immutable precomputed data (Tier 1: MEMO).
///
/// Each SearchContext eagerly constructs and owns its own tables. Construction
/// fills cycle directions and face/vertex links before returning; search then
/// reads these tables without changing them. Clone copies the owned allocations.
#[derive(Debug, Clone)]
pub struct MemoizedData {
    /// All possible facial cycles (NCYCLES = 394 for NCOLORS=6)
    pub cycles: CyclesArray,

    /// Cycle-related MEMO data (lookup tables for constraint propagation)
    pub cycles_memo: CyclesMemo,

    /// All face-related MEMO data (binomial coefficients, adjacency, etc.)
    pub faces: FacesMemo,

    /// All vertex-related MEMO data (crossing point configurations)
    pub vertices: VerticesMemo,
}

impl MemoizedData {
    /// Initialize all MEMO data structures.
    ///
    /// Computes all immutable precomputed data needed for the search.
    /// This is called once at SearchContext creation.
    pub fn new() -> Self {
        eprintln!("[MemoizedData] Initializing all MEMO structures...");

        let mut cycles = CyclesArray::generate();
        let cycles_memo = CyclesMemo::initialize(&mut cycles);
        let mut faces = FacesMemo::initialize(&cycles);
        let vertices = VerticesMemo::initialize();

        // Complete cross-references after both faces and vertices exist.
        faces.populate_vertex_links(&vertices);

        eprintln!(
            "[MemoizedData] Initialization complete ({} cycles, {} faces, {} possible vertices)",
            cycles.len(),
            faces.faces.len(),
            vertices.vertices_by_id.len()
        );

        Self {
            cycles,
            cycles_memo,
            faces,
            vertices,
        }
    }
}

impl Default for MemoizedData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::constants::NPOINTS;
    use crate::geometry::{CurveLink, EdgeRef};

    #[test]
    fn test_vertex_tables_and_face_links_agree() {
        let memo = MemoizedData::new();
        assert_eq!(memo.vertices.vertices_by_id.len(), NPOINTS);

        for (outside, primary_rows) in memo.vertices.vertices.iter().enumerate() {
            for (primary, secondary_row) in primary_rows.iter().enumerate() {
                for (secondary, entry) in secondary_row.iter().enumerate() {
                    let Some(vertex) = entry else { continue };
                    assert_eq!(memo.vertices.get_vertex_by_id(vertex.id), Some(vertex));

                    // The four incident regions differ only in the crossing colors.
                    let primary_bit = 1 << primary;
                    let secondary_bit = 1 << secondary;
                    assert_eq!(outside & (primary_bit | secondary_bit), 0);
                    assert_eq!(
                        vertex.incoming_edges,
                        [
                            EdgeRef::new(outside | primary_bit | secondary_bit, primary),
                            EdgeRef::new(outside, primary),
                            EdgeRef::new(outside | primary_bit, secondary),
                            EdgeRef::new(outside | secondary_bit, secondary),
                        ]
                    );

                    for incoming in vertex.incoming_edges {
                        let other = if incoming.color_idx == primary {
                            secondary
                        } else {
                            primary
                        };
                        // Continuing along a curve crosses only the other curve.
                        let next =
                            EdgeRef::new(incoming.face_id ^ (1 << other), incoming.color_idx);
                        let edge = &memo.faces.faces[incoming.face_id].edges[incoming.color_idx];
                        assert_eq!(
                            edge.possibly_to[other],
                            Some(CurveLink::new(next, vertex.id))
                        );
                    }
                }
            }
        }

        for face in &memo.faces.faces {
            for (color, edge) in face.edges.iter().enumerate() {
                assert!(edge.possibly_to[color].is_none());
            }
        }
    }
}
