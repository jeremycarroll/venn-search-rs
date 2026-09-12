// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Vertex-related MEMO data structures.
//!
//! This module computes all possible vertex configurations where curves
//! cross in the Venn diagram.

use crate::geometry::constants::{NCOLORS, NFACES, NPOINTS};
use crate::geometry::{Color, ColorSet, Vertex};

/// MEMO data for all possible vertices in the diagram.
///
/// The sparse array is indexed by `[outside_face][primary][secondary]`.
/// `outside_face` is the face bitmask with the two crossing colors removed:
/// it identifies the incident region outside those two curves, while retaining
/// membership in every other curve. Primary and secondary distinguish the two
/// crossing orientations.
///
/// For example, `[0b100][0][1]` is outside curves 0 and 1 and inside curve 2.
/// Its four incident faces have masks 0b100, 0b101, 0b110 and 0b111.
/// Swapping primary and secondary gives a separate possible configuration.
///
/// # Allocation and construction
///
/// For NCOLORS=6 the boxed array has 64 × 6 × 6 = 2304 slots, of which
/// NPOINTS = 2^(6-2) × 6 × 5 = 480 contain possible configurations.
/// Diagonal color pairs and face masks containing a crossing color stay None.
/// The spare slots allow direct indexing without a packed-index calculation.
///
/// The first pass assigns IDs in face/edge/other-color traversal order and
/// creates vertices with initialized placeholder edges. The second pass fills
/// all four incoming slots. The flat lookup then copies the completed vertices
/// in ID order. Both lookups belong to this context and stay fixed during search.
#[derive(Debug, Clone)]
pub struct VerticesMemo {
    /// All possible vertex configurations indexed by outside face and crossing orientation.
    ///
    /// **Indexing**: `vertices[outside_face][primary][secondary]`
    ///
    /// The face mask excludes both crossing colors; primary and secondary select
    /// the orientation. Invalid configurations contain None.
    ///
    /// Heap-allocated because the full face/color grid is large.
    pub vertices: Box<[[[Option<Vertex>; NCOLORS]; NCOLORS]; NFACES]>,

    /// Flat lookup by vertex ID for corner checking.
    /// vertices_by_id[id] gives the vertex with that ID.
    pub vertices_by_id: Vec<Vertex>,
}

/// Check if an edge is clockwise around its face.
///
/// An edge is clockwise if its color is a member of the face's color set.
///
/// # Arguments
///
/// * `edge_color` - The color of the edge
/// * `face_colors` - The colors bounding the face
///
/// # Returns
///
/// True if the edge is clockwise, false otherwise.
#[inline]
pub fn is_edge_clockwise(edge_color: Color, face_colors: ColorSet) -> bool {
    face_colors.contains(edge_color)
}

/// Compute which slot (0-3) an incoming edge occupies in a vertex.
///
/// The slot is determined by:
/// - Whether the edge is clockwise or counterclockwise
/// - Whether the other crossing color is inside or outside the face
///
/// # Slot Mapping
///
/// - Slot 0: Clockwise edge, other color inside face (primary clockwise)
/// - Slot 1: Counterclockwise edge, other color outside face (primary counterclockwise)
/// - Slot 2: Counterclockwise edge, other color inside face (secondary counterclockwise)
/// - Slot 3: Clockwise edge, other color outside face (secondary clockwise)
///
/// See docs/DESIGN.md "Vertex Structure and Edge Organization" for details.
///
/// # Arguments
///
/// * `edge_color` - The color of the edge
/// * `other_color` - The other color crossing at this vertex
/// * `face_colors` - The colors bounding the face
///
/// # Returns
///
/// Slot index (0-3) for this edge in the vertex's incoming_edges array.
pub fn compute_incoming_edge_slot(
    edge_color: Color,
    other_color: Color,
    face_colors: ColorSet,
) -> usize {
    let is_clockwise = is_edge_clockwise(edge_color, face_colors);
    let other_in_face = face_colors.contains(other_color);

    if is_clockwise {
        if other_in_face {
            0 // Primary clockwise, other in face
        } else {
            3 // Secondary clockwise, other not in face
        }
    } else if other_in_face {
        2 // Secondary counterclockwise, other in face
    } else {
        1 // Primary counterclockwise, other not in face
    }
}

/// Determine primary and secondary colors from the incoming edge slot.
///
/// # Slot to Color Mapping
///
/// - Slots 0, 1: Primary = edge_color, Secondary = other_color
/// - Slots 2, 3: Primary = other_color, Secondary = edge_color
///
/// # Arguments
///
/// * `slot` - The slot index (0-3)
/// * `edge_color` - The color of the edge
/// * `other_color` - The other color crossing at this vertex
///
/// # Returns
///
/// Tuple of (primary_color, secondary_color).
pub fn determine_primary_secondary(
    slot: usize,
    edge_color: Color,
    other_color: Color,
) -> (Color, Color) {
    match slot {
        0 | 1 => (edge_color, other_color),
        2 | 3 => (other_color, edge_color),
        _ => unreachable!("Slot must be 0-3, got {}", slot),
    }
}

/// Compute the "outside face" index for vertex indexing.
///
/// Remove the two crossing colors from the face mask, retaining membership in
/// all other curves. This identifies the incident face outside both crossing
/// curves and supplies the first index in the 3D vertex array.
///
/// # Formula
///
/// outside_face = face_colors & ~(1 << primary) & ~(1 << secondary)
///
/// # Arguments
///
/// * `face_colors` - The colors bounding the current face
/// * `primary` - The primary color crossing at the vertex
/// * `secondary` - The secondary color crossing at the vertex
///
/// # Returns
///
/// Face ID (bitmask) with both crossing colors removed.
pub fn compute_outside_face(face_colors: ColorSet, primary: Color, secondary: Color) -> usize {
    let mut outside = face_colors;
    outside.remove(primary);
    outside.remove(secondary);
    outside.bits() as usize
}

/// The table address and incoming slot for a crossing seen from one face.
/// Shared by both vertex passes and face-link construction; not a stored table.
pub(super) struct VertexLocation {
    pub(super) outside_face: usize,
    pub(super) primary: Color,
    pub(super) secondary: Color,
    pub(super) slot: usize,
}

impl VertexLocation {
    pub(super) fn new(edge_color: Color, other_color: Color, face_colors: ColorSet) -> Self {
        let slot = compute_incoming_edge_slot(edge_color, other_color, face_colors);
        let (primary, secondary) = determine_primary_secondary(slot, edge_color, other_color);
        let outside_face = compute_outside_face(face_colors, primary, secondary);
        Self {
            outside_face,
            primary,
            secondary,
            slot,
        }
    }
}

impl VerticesMemo {
    /// Initialize all vertex MEMO data.
    ///
    /// This computes all NPOINTS possible vertex configurations.
    ///
    /// # Algorithm
    ///
    /// **Phase 1: Create vertex structures**
    /// For each face (0..NFACES):
    ///   For each color pair (edge_color, other_color) where edge_color ≠ other_color:
    ///     1. Determine incoming edge slot (0-3) based on edge orientation and face membership
    ///     2. Determine primary/secondary colors from slot
    ///     3. Compute outside_face by removing primary and secondary from the face mask
    ///     4. Create vertex at vertices[outside_face][primary][secondary] if it doesn't exist
    ///
    /// **Phase 2: Populate incoming_edges**
    /// For each face (0..NFACES):
    ///   For each color pair (edge_color, other_color) where edge_color ≠ other_color:
    ///     1-3. Same as Phase 1 to locate the vertex
    ///     4. Set vertex.incoming_edges[slot] = EdgeRef(face_id, edge_color)
    ///
    /// This generates exactly NPOINTS = 2^(NCOLORS-2) × NCOLORS × (NCOLORS-1) vertices.
    pub fn initialize() -> Self {
        use crate::geometry::EdgeRef;

        let total_slots = NFACES * NCOLORS * NCOLORS;
        eprintln!(
            "[VerticesMemo] Allocating {} array slots ({} theoretical possible vertices)...",
            total_slots, NPOINTS
        );

        // Allocate vertex array (Box to keep it on heap)
        let mut vertices = Box::new([[[None; NCOLORS]; NCOLORS]; NFACES]);
        let mut vertex_id_counter = 0;

        eprintln!("[VerticesMemo] Phase 1: Creating vertex structures...");

        // PHASE 1: Create all vertex structures with placeholder incoming_edges
        for face_id in 0..NFACES {
            let face_colors = ColorSet::from_bits(face_id as u64);

            for edge_color_val in 0..NCOLORS {
                let edge_color = Color::new(edge_color_val as u8);

                for other_color_val in 0..NCOLORS {
                    if other_color_val == edge_color_val {
                        continue; // Skip when edge_color == other_color
                    }
                    let other_color = Color::new(other_color_val as u8);

                    let VertexLocation {
                        outside_face,
                        primary,
                        secondary,
                        ..
                    } = VertexLocation::new(edge_color, other_color, face_colors);

                    // Create vertex if it doesn't exist
                    let primary_idx = primary.value() as usize;
                    let secondary_idx = secondary.value() as usize;

                    if vertices[outside_face][primary_idx][secondary_idx].is_none() {
                        let vertex = Vertex::new(
                            vertex_id_counter,
                            primary,
                            secondary,
                            [
                                EdgeRef::new(0, 0),
                                EdgeRef::new(0, 0),
                                EdgeRef::new(0, 0),
                                EdgeRef::new(0, 0),
                            ], // Placeholder EdgeRefs - will be set in Phase 2
                        );

                        vertices[outside_face][primary_idx][secondary_idx] = Some(vertex);
                        vertex_id_counter += 1;
                    }
                }
            }
        }

        assert_eq!(
            vertex_id_counter, NPOINTS,
            "Expected {} vertices, generated {}",
            NPOINTS, vertex_id_counter
        );

        eprintln!(
            "[VerticesMemo] Phase 1 complete: {} vertices created.",
            vertex_id_counter
        );

        eprintln!("[VerticesMemo] Phase 2: Populating incoming_edges...");

        // PHASE 2: Populate incoming_edges for all vertices
        for face_id in 0..NFACES {
            let face_colors = ColorSet::from_bits(face_id as u64);

            for edge_color_val in 0..NCOLORS {
                let edge_color = Color::new(edge_color_val as u8);

                for other_color_val in 0..NCOLORS {
                    if other_color_val == edge_color_val {
                        continue;
                    }
                    let other_color = Color::new(other_color_val as u8);

                    let VertexLocation {
                        outside_face,
                        primary,
                        secondary,
                        slot,
                    } = VertexLocation::new(edge_color, other_color, face_colors);

                    // Get mutable reference to vertex
                    let primary_idx = primary.value() as usize;
                    let secondary_idx = secondary.value() as usize;

                    if let Some(vertex) = &mut vertices[outside_face][primary_idx][secondary_idx] {
                        // Set incoming edge for this slot
                        vertex.incoming_edges[slot] =
                            EdgeRef::new(face_id, edge_color.value() as usize);
                    } else {
                        panic!(
                            "Vertex should exist at [{:?}][{}][{}] (slot {})",
                            outside_face, primary_idx, secondary_idx, slot
                        );
                    }
                }
            }
        }

        eprintln!(
            "[VerticesMemo] Initialization complete: {} vertices with incoming_edges in {} slots.",
            vertex_id_counter, total_slots
        );

        // Build flat lookup by vertex ID
        let mut vertices_by_id = vec![];
        for face in 0..NFACES {
            for primary_idx in 0..NCOLORS {
                for secondary_idx in 0..NCOLORS {
                    if let Some(vertex) = &vertices[face][primary_idx][secondary_idx] {
                        vertices_by_id.push(*vertex);
                    }
                }
            }
        }
        vertices_by_id.sort_by_key(|v| v.id);

        Self {
            vertices,
            vertices_by_id,
        }
    }

    /// Get a vertex configuration by 3D array indexing.
    ///
    /// # Arguments
    ///
    /// * `face_id` - Outside face mask, with both crossing colors removed
    /// * `color_a` - Primary color crossing at this vertex
    /// * `color_b` - Secondary color crossing at this vertex
    ///
    /// # Returns
    ///
    /// The vertex configuration if it exists, None otherwise.
    #[inline]
    pub fn get_vertex(&self, face_id: usize, color_a: usize, color_b: usize) -> Option<&Vertex> {
        self.vertices[face_id][color_a][color_b].as_ref()
    }

    /// Get a vertex by its unique ID.
    ///
    /// # Arguments
    ///
    /// * `vertex_id` - The unique vertex ID (0..NPOINTS-1)
    ///
    /// # Returns
    ///
    /// The vertex with this ID, or None if ID is out of bounds.
    #[inline]
    pub fn get_vertex_by_id(&self, vertex_id: usize) -> Option<&Vertex> {
        self.vertices_by_id.get(vertex_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_edge_clockwise() {
        let color_a = Color::new(0);
        let color_b = Color::new(1);

        // Edge with color 'a' on face {a, b} is clockwise
        let face_ab = ColorSet::from_colors(&[color_a, color_b]);
        assert!(is_edge_clockwise(color_a, face_ab));

        // Edge with color 'a' on face {b} is not clockwise (edge is outside)
        let face_b = ColorSet::from_colors(&[color_b]);
        assert!(!is_edge_clockwise(color_a, face_b));
    }

    #[test]
    fn test_compute_incoming_edge_slot() {
        let edge_color = Color::new(0);
        let other_color = Color::new(1);

        // Case 1: Clockwise edge, other in face → Slot 0
        let face1 = ColorSet::from_colors(&[edge_color, other_color]);
        assert_eq!(
            compute_incoming_edge_slot(edge_color, other_color, face1),
            0
        );

        // Case 2: Counterclockwise edge, other not in face → Slot 1
        let face2 = ColorSet::from_colors(&[Color::new(2)]);
        assert_eq!(
            compute_incoming_edge_slot(edge_color, other_color, face2),
            1
        );

        // Case 3: Counterclockwise edge, other in face → Slot 2
        let face3 = ColorSet::from_colors(&[other_color]);
        assert_eq!(
            compute_incoming_edge_slot(edge_color, other_color, face3),
            2
        );

        // Case 4: Clockwise edge, other not in face → Slot 3
        let face4 = ColorSet::from_colors(&[edge_color]);
        assert_eq!(
            compute_incoming_edge_slot(edge_color, other_color, face4),
            3
        );
    }

    #[test]
    fn test_determine_primary_secondary() {
        let edge_color = Color::new(0);
        let other_color = Color::new(1);

        // Slots 0, 1: Primary = edge_color, Secondary = other_color
        assert_eq!(
            determine_primary_secondary(0, edge_color, other_color),
            (edge_color, other_color)
        );
        assert_eq!(
            determine_primary_secondary(1, edge_color, other_color),
            (edge_color, other_color)
        );

        // Slots 2, 3: Primary = other_color, Secondary = edge_color
        assert_eq!(
            determine_primary_secondary(2, edge_color, other_color),
            (other_color, edge_color)
        );
        assert_eq!(
            determine_primary_secondary(3, edge_color, other_color),
            (other_color, edge_color)
        );
    }

    #[test]
    fn test_compute_outside_face() {
        let primary = Color::new(0);
        let secondary = Color::new(1);

        // Face {0, 1, 2} → outside = {2} = 0b100 = 4
        let face_012 = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        assert_eq!(compute_outside_face(face_012, primary, secondary), 0b100);

        // Face {0, 1} → outside = {} = 0b000 = 0
        let face_01 = ColorSet::from_colors(&[primary, secondary]);
        assert_eq!(compute_outside_face(face_01, primary, secondary), 0);

        // Face {2, 3} → outside = {2, 3} = 0b1100 = 12 (only for NCOLORS >= 4)
        #[cfg(not(feature = "ncolors_3"))]
        {
            let face_23 = ColorSet::from_colors(&[Color::new(2), Color::new(3)]);
            assert_eq!(compute_outside_face(face_23, primary, secondary), 0b1100);
        }
    }

    #[test]
    fn test_vertices_memo_initialization() {
        let memo = VerticesMemo::initialize();

        // Should have allocated the full array
        assert_eq!(memo.vertices.len(), NFACES);
        assert_eq!(memo.vertices[0].len(), NCOLORS);
        assert_eq!(memo.vertices[0][0].len(), NCOLORS);
    }

    #[test]
    fn test_vertex_count() {
        let memo = VerticesMemo::initialize();

        // Count how many vertices were actually generated
        let mut count = 0;
        for face in 0..NFACES {
            for color_a in 0..NCOLORS {
                for color_b in 0..NCOLORS {
                    if memo.vertices[face][color_a][color_b].is_some() {
                        count += 1;
                    }
                }
            }
        }

        // Should generate exactly NPOINTS vertices
        assert_eq!(count, NPOINTS);

        // Verify specific counts for each NCOLORS
        match NCOLORS {
            3 => assert_eq!(count, 12),  // 2 * 3 * 2
            4 => assert_eq!(count, 48),  // 4 * 4 * 3
            5 => assert_eq!(count, 160), // 8 * 5 * 4
            6 => assert_eq!(count, 480), // 16 * 6 * 5
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_no_diagonal_vertices() {
        let memo = VerticesMemo::initialize();

        // Vertices on the diagonal (primary == secondary) should not exist
        for face in 0..NFACES {
            for color in 0..NCOLORS {
                assert!(
                    memo.vertices[face][color][color].is_none(),
                    "Unexpected vertex at [{}][{}][{}]",
                    face,
                    color,
                    color
                );
            }
        }
    }

    #[test]
    fn test_vertex_lookups() {
        let memo = VerticesMemo::initialize();

        // Test that we can retrieve vertices
        let mut found_at_least_one = false;
        for face in 0..NFACES {
            for color_a in 0..NCOLORS {
                for color_b in 0..NCOLORS {
                    if let Some(vertex) = memo.get_vertex(face, color_a, color_b) {
                        found_at_least_one = true;

                        // Verify vertex has correct primary/secondary
                        assert_eq!(vertex.primary.value() as usize, color_a);
                        assert_eq!(vertex.secondary.value() as usize, color_b);

                        // Verify vertex has both colors in its colorset
                        assert!(vertex.colors.contains(vertex.primary));
                        assert!(vertex.colors.contains(vertex.secondary));
                    }
                }
            }
        }

        assert!(found_at_least_one, "Should have found at least one vertex");
    }

    #[test]
    fn test_vertex_ids_unique() {
        let memo = VerticesMemo::initialize();

        // Collect all vertex IDs
        let mut ids = Vec::new();
        for face in 0..NFACES {
            for color_a in 0..NCOLORS {
                for color_b in 0..NCOLORS {
                    if let Some(vertex) = memo.get_vertex(face, color_a, color_b) {
                        ids.push(vertex.id);
                    }
                }
            }
        }

        // All IDs should be unique
        ids.sort();
        for i in 1..ids.len() {
            assert_ne!(ids[i - 1], ids[i], "Found duplicate vertex ID: {}", ids[i]);
        }

        // IDs should be sequential from 0 to NPOINTS-1
        assert_eq!(ids.len(), NPOINTS);
        for (i, &id) in ids.iter().enumerate() {
            assert_eq!(
                id, i,
                "Expected vertex ID {}, found {} at position {}",
                i, id, i
            );
        }
    }

    #[test]
    fn test_incoming_edges_populated() {
        let memo = VerticesMemo::initialize();

        // Check that incoming_edges are not placeholders
        let mut checked_count = 0;
        for face in 0..NFACES {
            for color_a in 0..NCOLORS {
                for color_b in 0..NCOLORS {
                    if let Some(vertex) = memo.get_vertex(face, color_a, color_b) {
                        // Each incoming_edges slot should be populated with non-placeholder EdgeRef
                        for (slot, edge_ref) in vertex.incoming_edges.iter().enumerate() {
                            // EdgeRef should point to a valid face and color
                            assert!(
                                edge_ref.face_id < NFACES,
                                "Vertex [{:?}][{}][{}] slot {} has invalid face_id {}",
                                face,
                                color_a,
                                color_b,
                                slot,
                                edge_ref.face_id
                            );
                            assert!(
                                edge_ref.color_idx < NCOLORS,
                                "Vertex [{:?}][{}][{}] slot {} has invalid color_idx {}",
                                face,
                                color_a,
                                color_b,
                                slot,
                                edge_ref.color_idx
                            );

                            // At least one incoming edge should not be EdgeRef::new(0, 0)
                            // (to ensure we're not just seeing placeholders)
                            if edge_ref.face_id != 0 || edge_ref.color_idx != 0 {
                                checked_count += 1;
                            }
                        }
                    }
                }
            }
        }

        // We should have found many non-placeholder EdgeRefs
        // (Most vertices will have at least one edge not at (0, 0))
        assert!(
            checked_count > NPOINTS * 2,
            "Expected many non-placeholder incoming_edges, found only {}",
            checked_count
        );
    }

    #[test]
    fn test_incoming_edges_reference_correct_colors() {
        let memo = VerticesMemo::initialize();

        // For each vertex, verify that incoming_edges reference edges with
        // colors that match the vertex's crossing colors
        for face in 0..NFACES {
            for color_a in 0..NCOLORS {
                for color_b in 0..NCOLORS {
                    if let Some(vertex) = memo.get_vertex(face, color_a, color_b) {
                        let primary = vertex.primary;
                        let secondary = vertex.secondary;

                        // Each incoming edge should be of color primary or secondary
                        for (slot, edge_ref) in vertex.incoming_edges.iter().enumerate() {
                            let edge_color = Color::new(edge_ref.color_idx as u8);

                            assert!(
                                edge_color == primary || edge_color == secondary,
                                "Vertex [{:?}][{}][{}] slot {}: edge has color {:?}, but vertex has colors {:?} and {:?}",
                                face,
                                color_a,
                                color_b,
                                slot,
                                edge_color,
                                primary,
                                secondary
                            );
                        }

                        // Slots 0, 1 should be primary color
                        // Slots 2, 3 should be secondary color
                        assert_eq!(
                            Color::new(vertex.incoming_edges[0].color_idx as u8),
                            primary,
                            "Slot 0 should be primary color"
                        );
                        assert_eq!(
                            Color::new(vertex.incoming_edges[1].color_idx as u8),
                            primary,
                            "Slot 1 should be primary color"
                        );
                        assert_eq!(
                            Color::new(vertex.incoming_edges[2].color_idx as u8),
                            secondary,
                            "Slot 2 should be secondary color"
                        );
                        assert_eq!(
                            Color::new(vertex.incoming_edges[3].color_idx as u8),
                            secondary,
                            "Slot 3 should be secondary color"
                        );
                    }
                }
            }
        }
    }
}
