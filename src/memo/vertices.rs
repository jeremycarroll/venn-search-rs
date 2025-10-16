// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Vertex-related MEMO data structures.
//!
//! This module computes all possible vertex configurations where curves
//! cross in the Venn diagram.

use crate::geometry::constants::{NCOLORS, NFACES, NPOINTS};
use crate::geometry::{Color, ColorSet, Vertex};

/// MEMO data for all possible vertices in the diagram.
///
/// # The Vertex Allocation Strategy (Mental Somersault Required!)
///
/// A monotone 6-Venn diagram has only **126 actual vertices** (by Euler's formula:
/// V - E + F = 2, with V=126, E=189, F=64).
///
/// ## Theoretical vs Actual Allocation
///
/// - **Theoretical possible vertices**: NPOINTS = 2^(NCOLORS-2) × NCOLORS × (NCOLORS-1) = 16 × 6 × 5 = **480**
/// - **Actual array allocation**: NFACES × NCOLORS × NCOLORS = 64 × 6 × 6 = **2304 slots**
/// - **Precomputed vertices**: **480** (all theoretically possible configurations)
/// - **Used in any specific solution**: **126** (from Euler's formula: V - E + F = 2)
///
/// **Memory utilization during search**: 480 of 2304 slots are Some(Vertex) (21% utilization).
/// The remaining 1824 slots stay None. This enables **O(1) indexing** with simple 3D array lookup.
///
/// ## Why Allocate 2304 Instead of 480?
///
/// The array includes:
/// 1. **Diagonal entries** (primary == secondary): These are never valid vertices, but
///    including them simplifies indexing (no need to subtract 1 or remap indices).
/// 2. **All NFACES combinations**: Not all faces will have vertices for all color pairs,
///    but computing which faces are valid would require complex precomputation.
///
/// **Trade-off**: We sacrifice ~140 KB of memory to gain:
/// - Simple 3D array indexing: `vertices[face][primary][secondary]`
/// - No conditional logic in hot path lookups
/// - Faster constraint propagation during search
///
/// **Alternative considered**: Use `[NFACES][NCOLORS][NCOLORS-1]` to exclude diagonal,
/// reducing to 1920 slots (~34% smaller). However, this would require index remapping
/// (`secondary - (secondary > primary ? 1 : 0)`) on every lookup. The current approach
/// prioritizes simplicity and performance over memory efficiency
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
/// memory overhead (2304 slots, 480 precomputed, 126 used per solution) enables
/// dramatic algorithmic speedups through simple O(1) array indexing.
///
/// See [`vertex.c::getOrInitializeVertex()`](https://github.com/jeremycarroll/venntriangles/blob/main/vertex.c)
/// for the C implementation.
///
/// # Memory Layout
///
/// - `vertices`: **Heap-allocated** via Box
///   - Reason: Large 3D array (NFACES × NCOLORS × NCOLORS = 2304 elements for NCOLORS=6)
///   - Size: 64 × 6 × 6 × sizeof(Option<Vertex>) ≈ 147 KB for NCOLORS=6
///   - Box keeps only a pointer (8 bytes) on stack, array lives on heap
///   - Prevents stack overflow for large arrays
///   - 480 of 2304 slots precomputed (21% utilization), rest remain None
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
    /// **Note**: After precomputation, 480 of 2304 slots contain possible vertex configurations.
    /// In any specific solution, only 126 vertices are actually used (Euler's formula: V - E + F = 2).
    ///
    /// **Heap-allocated** via Box - 3D array is too large for stack (147 KB).
    pub vertices: Box<[[[Option<Vertex>; NCOLORS]; NCOLORS]; NFACES]>,
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
fn is_edge_clockwise(edge_color: Color, face_colors: ColorSet) -> bool {
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
fn compute_incoming_edge_slot(
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
fn determine_primary_secondary(
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
/// The outside face is the set of colors that are outside BOTH the primary
/// and secondary curves. This is used as the first index in the 3D vertex array.
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
/// Face ID (bitmask) of colors outside both crossing curves.
fn compute_outside_face(face_colors: ColorSet, primary: Color, secondary: Color) -> usize {
    let mut outside = face_colors;
    outside.remove(primary);
    outside.remove(secondary);
    outside.bits() as usize
}

impl VerticesMemo {
    /// Initialize all vertex MEMO data.
    ///
    /// This computes all NPOINTS possible vertex configurations.
    ///
    /// # Algorithm
    ///
    /// For each face (0..NFACES):
    ///   For each color pair (edge_color, other_color) where edge_color ≠ other_color:
    ///     1. Determine incoming edge slot (0-3) based on edge orientation and face membership
    ///     2. Determine primary/secondary colors from slot
    ///     3. Compute outside_face = colors outside BOTH primary and secondary
    ///     4. Get or create vertex at vertices[outside_face][primary][secondary]
    ///     5. Record this edge in the vertex's incoming_edges array
    ///
    /// This generates exactly NPOINTS = 2^(NCOLORS-2) × NCOLORS × (NCOLORS-1) vertices.
    ///
    /// # Note
    ///
    /// The incoming_edges array is set to placeholder EdgeIds (0) since edges don't
    /// exist yet. Phase 7 (VennPredicate) will create the full edge infrastructure
    /// and properly connect vertices to edges.
    pub fn initialize() -> Self {
        let total_slots = NFACES * NCOLORS * NCOLORS;
        eprintln!(
            "[VerticesMemo] Allocating {} array slots ({} theoretical possible vertices)...",
            total_slots, NPOINTS
        );

        // Allocate vertex array (Box to keep it on heap)
        let mut vertices = Box::new([[[None; NCOLORS]; NCOLORS]; NFACES]);
        let mut vertex_id_counter = 0;

        eprintln!("[VerticesMemo] Computing vertex configurations...");

        // Iterate through all (face, edge_color, other_color) combinations
        for face_id in 0..NFACES {
            // Convert face ID to ColorSet (face ID is a bitmask of colors)
            let face_colors = ColorSet::from_bits(face_id as u64);

            for edge_color_val in 0..NCOLORS {
                let edge_color = Color::new(edge_color_val as u8);

                for other_color_val in 0..NCOLORS {
                    if other_color_val == edge_color_val {
                        continue; // Skip when edge_color == other_color
                    }
                    let other_color = Color::new(other_color_val as u8);

                    // Compute vertex parameters using helper functions
                    let _slot =
                        compute_incoming_edge_slot(edge_color, other_color, face_colors);
                    let (primary, secondary) =
                        determine_primary_secondary(_slot, edge_color, other_color);
                    let outside_face = compute_outside_face(face_colors, primary, secondary);

                    // Get or create vertex at [outside_face][primary][secondary]
                    let primary_idx = primary.value() as usize;
                    let secondary_idx = secondary.value() as usize;

                    if vertices[outside_face][primary_idx][secondary_idx].is_none() {
                        // Create new vertex
                        let vertex = Vertex::new(
                            vertex_id_counter,
                            primary,
                            secondary,
                            [0, 0, 0, 0], // Placeholder EdgeIds - will be set in Phase 7
                        );

                        vertices[outside_face][primary_idx][secondary_idx] = Some(vertex);
                        vertex_id_counter += 1;
                    }

                    // Note: In the C code, we would also set incoming_edges[slot] here.
                    // In Rust, we skip this for now since EdgeIds don't exist yet.
                    // Phase 7 will properly connect vertices to edges.
                }
            }
        }

        assert_eq!(
            vertex_id_counter, NPOINTS,
            "Expected {} vertices, generated {}",
            NPOINTS, vertex_id_counter
        );

        eprintln!(
            "[VerticesMemo] Generated {} vertices (21% utilization of {} slots).",
            vertex_id_counter, total_slots
        );

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

        // Face {2, 3} → outside = {2, 3} = 0b1100 = 12
        let face_23 = ColorSet::from_colors(&[Color::new(2), Color::new(3)]);
        assert_eq!(compute_outside_face(face_23, primary, secondary), 0b1100);
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
            3 => assert_eq!(count, 12), // 2 * 3 * 2
            4 => assert_eq!(count, 48), // 4 * 4 * 3
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
            assert_ne!(
                ids[i - 1],
                ids[i],
                "Found duplicate vertex ID: {}",
                ids[i]
            );
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
}
