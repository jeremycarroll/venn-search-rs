// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Face-related MEMO data structures.
//!
//! This module computes all immutable face-related data:
//! - Face degree expectations (binomial coefficients)
//! - Face adjacency tables
//! - Cycle-to-face relationship lookups (next/previous by cycle ID)
//! - Monotonicity constraints
//!
//! # Monotonicity and Convex Curves
//!
//! This implementation searches for **monotone Venn diagrams**, which can be drawn
//! with convex curves. Triangles are convex, so any diagram drawable with triangles
//! must be monotone.
//!
//! A monotone diagram has the property that each facial cycle crosses each curve
//! at most once. This constraint eliminates many invalid configurations and is
//! enforced during MEMO initialization by filtering out non-monotone cycles.

use crate::geometry::constants::{NCOLORS, NCYCLES, NFACES};
use crate::geometry::{Color, ColorSet, CycleSet, EdgeMemo, EdgeRef, Face, FaceId};
use crate::memo::vertices::{
    compute_incoming_edge_slot, compute_outside_face, determine_primary_secondary,
};

/// Type alias for face adjacency lookup tables.
///
/// Maps face_id → cycle_id → adjacent face (or None if invalid).
type FaceAdjacencyTable = Vec<Vec<Option<FaceId>>>;

/// MEMO data for all faces in the diagram.
///
/// This structure is computed once during initialization and contains
/// all precomputed face-related lookup tables needed for efficient
/// constraint propagation.
///
/// # Memory Layout
///
/// - `faces`: **Heap-allocated** Vec (NFACES = 64 for NCOLORS=6)
///   - Reason: Variable size depending on NCOLORS; too large for stack
///   - Size: 64 × sizeof(Face) ≈ 5 KB
///
/// - `face_degree_by_color_count`: **Stack-allocated** array (7 elements for NCOLORS=6)
///   - Reason: Small fixed-size array (NCOLORS+1 elements); efficient on stack
///   - Size: 7 × 8 bytes = 56 bytes
///
#[derive(Debug, Clone)]
pub struct FacesMemo {
    /// All faces in the diagram (NFACES = 2^NCOLORS faces).
    ///
    /// Indexed by face ID (0..NFACES), where face ID is the bitmask
    /// of colors bounding that face.
    ///
    /// **Heap-allocated** via Vec to handle variable NFACES (8 for N=3, 64 for N=6).
    pub faces: Vec<Face>,

    /// Expected cycle length for faces with k colors.
    ///
    /// `face_degree_by_color_count[k]` = C(NCOLORS, k) = number of ways to
    /// choose k items from NCOLORS items.
    ///
    /// This is used to validate that face degree signatures are feasible.
    ///
    /// For NCOLORS=6:
    /// - [0] = 1
    /// - [1] = 6
    /// - [2] = 15
    /// - [3] = 20
    /// - [4] = 15
    /// - [5] = 6
    /// - [6] = 1
    ///
    /// **Stack-allocated** array - small and fixed size (NCOLORS+1 ≤ 7 elements).
    pub face_degree_by_color_count: [u64; NCOLORS + 1],

    /// Next face when traversing a cycle.
    ///
    /// `next_face_by_cycle[face_id][cycle_id]` gives the face you reach by traversing
    /// cycle_id forward from face_id.
    ///
    /// Only valid for cycles that are valid for the face (monotone property satisfied).
    /// Returns None for invalid cycle/face combinations.
    ///
    /// **Heap-allocated** via Vec - NFACES × NCYCLES is too large for stack (64 × 394 ≈ 25 KB).
    pub next_face_by_cycle: FaceAdjacencyTable,

    /// Previous face when traversing a cycle backward.
    ///
    /// `previous_face_by_cycle[face_id][cycle_id]` gives the face you came from when
    /// traversing cycle_id backward to face_id.
    ///
    /// Only valid for cycles that are valid for the face (monotone property satisfied).
    /// Returns None for invalid cycle/face combinations.
    ///
    /// **Heap-allocated** via Vec - NFACES × NCYCLES is too large for stack (64 × 394 ≈ 25 KB).
    pub previous_face_by_cycle: FaceAdjacencyTable,
}

impl FacesMemo {
    /// Initialize all face MEMO data.
    ///
    /// This computes:
    /// 1. Binomial coefficients for face degree validation
    /// 2. All NFACES faces with their color sets
    /// 3. Edges and adjacency relationships for each face
    /// 4. Monotonicity constraints (which cycles are valid for which faces)
    /// 5. Next/previous face lookups by cycle ID
    ///
    /// # Arguments
    ///
    /// * `cycles` - The global array of all possible cycles
    pub fn initialize(cycles: &crate::memo::CyclesArray) -> Self {
        eprintln!("[FacesMemo] Computing binomial coefficients...");
        let face_degree_by_color_count = compute_binomial_coefficients();

        eprintln!("[FacesMemo] Creating {} faces with edges...", NFACES);
        let mut faces = Vec::with_capacity(NFACES);
        for face_id in 0..NFACES {
            faces.push(create_face_with_edges(face_id));
        }

        eprintln!("[FacesMemo] Applying monotonicity constraints...");
        let (next_face_by_cycle, previous_face_by_cycle) =
            apply_monotonicity_constraints(&mut faces, cycles);

        eprintln!("[FacesMemo] Initialization complete.");

        Self {
            faces,
            face_degree_by_color_count,
            next_face_by_cycle,
            previous_face_by_cycle,
        }
    }

    /// Get a face by its ID.
    #[inline]
    pub fn get_face(&self, face_id: FaceId) -> &Face {
        &self.faces[face_id]
    }

    /// Populate possibly_to vertex linkages for all edges.
    ///
    /// This must be called AFTER vertices are initialized, as it links
    /// edges to vertices based on which color pairs can meet.
    ///
    /// # Algorithm
    ///
    /// For each face:
    ///   For each edge with color C:
    ///     For each potential next color C':
    ///       1. Locate the vertex where C and C' cross using the same indexing as VerticesMemo
    ///       2. If vertex exists, create CurveLink and set edge.possibly_to[C'] = Some(link)
    ///
    /// This enables corner detection during search - when assigning a facial cycle,
    /// we can look up vertices at color transitions and count crossings.
    ///
    /// # Arguments
    ///
    /// * `vertices` - The initialized VerticesMemo with all possible vertices
    pub fn populate_vertex_links(&mut self, vertices: &crate::memo::VerticesMemo) {
        use crate::geometry::{Color, CurveLink, EdgeRef};

        eprintln!("[FacesMemo] Populating vertex links for all edges...");

        let mut link_count = 0;

        for face_id in 0..NFACES {
            let face_colors = self.faces[face_id].colors;

            for edge_color_idx in 0..NCOLORS {
                let edge_color = Color::new(edge_color_idx as u8);

                // For each potential next color in a cycle
                for next_color_idx in 0..NCOLORS {
                    if next_color_idx == edge_color_idx {
                        continue; // Skip same color
                    }
                    let next_color = Color::new(next_color_idx as u8);

                    // Locate the vertex where edge_color and next_color cross
                    // Using the same logic as VerticesMemo::initialize()

                    // 1. Compute incoming edge slot
                    let slot = compute_incoming_edge_slot(edge_color, next_color, face_colors);

                    // 2. Determine primary/secondary
                    let (primary, secondary) =
                        determine_primary_secondary(slot, edge_color, next_color);

                    // 3. Compute outside face
                    let outside_face = compute_outside_face(face_colors, primary, secondary);

                    // 4. Look up vertex
                    let primary_idx = primary.value() as usize;
                    let secondary_idx = secondary.value() as usize;

                    if let Some(vertex) =
                        vertices.get_vertex(outside_face, primary_idx, secondary_idx)
                    {
                        // Compute the next edge: the adjacent face across both colors
                        // When traversing a curve of edge_color, we stay on edge_color across the vertex
                        let edge_color_bit = 1u64 << edge_color_idx;
                        let next_color_bit = 1u64 << next_color_idx;
                        let xor_mask = edge_color_bit | next_color_bit;
                        let next_face_id = (face_colors.bits() ^ xor_mask) as usize;

                        // The next edge is on the adjacent face with the SAME color (edge_color)
                        // When traversing edge_color's curve, we continue on edge_color after crossing the vertex
                        let next_edge_ref = EdgeRef::new(next_face_id, edge_color_idx);
                        let link = CurveLink::new(next_edge_ref, vertex.id);

                        // Set possibly_to for this edge
                        self.faces[face_id].edges[edge_color_idx]
                            .set_possibly_to(next_color_idx, Some(link));
                        link_count += 1;
                    }
                }
            }
        }

        eprintln!(
            "[FacesMemo] Vertex links complete: {} edge->vertex links populated.",
            link_count
        );
    }
}

/// Compute binomial coefficients C(NCOLORS, k) for k=0..NCOLORS.
///
/// Uses the recurrence relation:
/// C(n, k) = C(n, k-1) * (n - k + 1) / k
///
/// Starting with C(n, 0) = 1.
///
/// # Algorithm
///
/// Uses the recurrence relation C(n, k) = C(n, k-1) * (n - k + 1) / k:
fn compute_binomial_coefficients() -> [u64; NCOLORS + 1] {
    let mut coefficients = [0u64; NCOLORS + 1];
    coefficients[0] = 1;

    for i in 0..NCOLORS {
        coefficients[i + 1] = coefficients[i] * (NCOLORS - i) as u64 / (i + 1) as u64;
    }

    coefficients
}

/// Create a face with the given ID, including edges and adjacency.
///
/// The face ID is interpreted as a bitmask of colors:
/// - Bit i set → color i bounds this face
/// - Face 0 = outer face (no colors, unbounded)
/// - Face NFACES-1 = inner face (all colors)
///
/// # Arguments
///
/// * `face_id` - The face identifier (0..NFACES)
///
/// # Returns
///
/// A Face with:
/// - ID set to face_id
/// - Colors set from bitmask
/// - Edges initialized (one per color, with reversed references)
/// - Adjacent faces computed via XOR
/// - Possible cycles initialized to all cycles with matching colors
fn create_face_with_edges(face_id: FaceId) -> Face {
    // Convert face ID bitmask to ColorSet
    let mut colors = ColorSet::empty();
    for i in 0..NCOLORS {
        if (face_id & (1 << i)) != 0 {
            colors.insert(Color::new(i as u8));
        }
    }

    // Compute adjacent faces (face XOR (1 << color))
    let mut adjacent_faces = [0; NCOLORS];
    for (i, item) in adjacent_faces.iter_mut().enumerate().take(NCOLORS) {
        *item = face_id ^ (1 << i);
    }

    // Create edges for this face (one per color)
    let mut edges = [EdgeMemo::new(Color::new(0), colors, EdgeRef::new(0, 0)); NCOLORS];

    for color_idx in 0..NCOLORS {
        let color = Color::new(color_idx as u8);

        // Reversed edge is in the adjacent face (across this color)
        let reversed_face_id = adjacent_faces[color_idx];
        let reversed_edge_ref = EdgeRef::new(reversed_face_id, color_idx);

        edges[color_idx] = EdgeMemo::new(color, colors, reversed_edge_ref);
    }

    // Start with all possible cycles for this color count
    // (Will be filtered by monotonicity constraints)
    let possible_cycles = CycleSet::full();

    Face::new(face_id, colors, possible_cycles, edges, adjacent_faces)
}

/// Check if a cycle is valid for a face.
///
/// A cycle is valid if it has some colors inside the face and some colors outside.
/// This ensures the cycle actually crosses the face boundary.
fn is_cycle_valid_for_face(cycle_colors: ColorSet, face_colors: ColorSet) -> bool {
    let inside = cycle_colors.bits() & face_colors.bits();
    let outside = cycle_colors.bits() & !face_colors.bits();

    inside != 0 && outside != 0
}

/// Check if an edge transition occurs between two consecutive colors in a cycle.
///
/// An edge transition occurs when one color is inside the face and the other is outside.
/// Returns true if a transition occurs, and updates next_face or previous_face accordingly.
///
/// # Arguments
///
/// * `color1` - First color in the edge
/// * `color2` - Second color in the edge
/// * `face_colors` - The colorset of the current face
/// * `previous_face` - Output: face we came from (if color1 is outside)
/// * `next_face` - Output: face we're going to (if color1 is inside)
fn check_edge_transition(
    color1: Color,
    color2: Color,
    face_colors: ColorSet,
    previous_face: &mut Option<FaceId>,
    next_face: &mut Option<FaceId>,
) -> bool {
    let color1_inside = face_colors.contains(color1);
    let color2_inside = face_colors.contains(color2);

    // No transition if both colors have same status
    if color1_inside == color2_inside {
        return false;
    }

    // Compute the XOR to get the adjacent face
    let color1_bit = 1u64 << color1.value();
    let color2_bit = 1u64 << color2.value();
    let xor_mask = color1_bit | color2_bit;
    let adjacent_face = (face_colors.bits() ^ xor_mask) as usize;

    if color1_inside {
        // Outbound transition: color1 is in, color2 is out
        if next_face.is_none() {
            *next_face = Some(adjacent_face);
        }
    } else {
        // Inbound transition: color1 is out, color2 is in
        if previous_face.is_none() {
            *previous_face = Some(adjacent_face);
        }
    }

    true
}

/// Check if a cycle has exactly two edge transitions.
///
/// A monotone cycle must cross the face boundary exactly twice:
/// once entering and once exiting.
///
/// # Returns
///
/// Returns Some((previous_face, next_face)) if exactly two transitions found,
/// None otherwise.
fn check_exactly_two_transitions(
    cycle: &crate::geometry::Cycle,
    face_colors: ColorSet,
) -> Option<(FaceId, FaceId)> {
    let mut transition_count = 0;
    let mut previous_face = None;
    let mut next_face = None;

    let colors = cycle.colors();
    let len = cycle.len();

    // Check wrap-around edge (last to first)
    if check_edge_transition(
        colors[len - 1],
        colors[0],
        face_colors,
        &mut previous_face,
        &mut next_face,
    ) {
        transition_count += 1;
    }

    // Check all consecutive edges
    for i in 1..len {
        if check_edge_transition(
            colors[i - 1],
            colors[i],
            face_colors,
            &mut previous_face,
            &mut next_face,
        ) {
            transition_count += 1;

            // Early exit if we find too many transitions
            if transition_count > 2 {
                return None;
            }
        }
    }

    if transition_count == 2 {
        // Invariant: transition_count == 2 guarantees both faces are set
        Some((previous_face.unwrap(), next_face.unwrap()))
    } else {
        None
    }
}

/// Apply monotonicity constraints to filter invalid cycles.
///
/// For each face, for each cycle:
/// 1. Check if cycle is valid for this face (has right colors, correct transitions)
/// 2. If valid, compute next/previous faces for this cycle
/// 3. If invalid, remove from possible_cycles
///
/// # Monotonicity and Convex Curves
///
/// This constraint is fundamental to drawing Venn diagrams with **convex curves**.
/// Since **triangles are convex**, any diagram drawable with triangles must be monotone.
///
/// A monotone Venn diagram has the property that each facial cycle crosses each curve
/// at most once. This means:
/// - A cycle for face {a,b,c} must have colors from {a,b,c}
/// - The cycle must have exactly 2 edge transitions (in/out of face)
/// - The next and previous faces are determined by which edges transition
///
/// Non-monotone diagrams (where cycles can cross curves multiple times) cannot be
/// drawn with convex curves and are excluded from this search
///
/// # Special Cases
///
/// - **Outer face (0)**: Can only have cycles of length NCOLORS (full 6-cycles)
///   - Monotonicity requires the outer boundary to cross each curve exactly once
/// - **Inner face (NFACES-1)**: Can only have cycles of length NCOLORS (full 6-cycles)
///   - Monotonicity requires the inner boundary to cross each curve exactly once
///   - (Non-monotone diagrams can have 4- or 5-cycles, but not with convex curves)
///   - The inner face will later be assigned the canonical cycle (0,1,2,3,4,5)
///     for symmetry breaking (done during search, not here)
///
/// # Returns
///
/// Returns (next_face_by_cycle, previous_face_by_cycle) lookup tables.
fn apply_monotonicity_constraints(
    faces: &mut [Face],
    cycles: &crate::memo::CyclesArray,
) -> (FaceAdjacencyTable, FaceAdjacencyTable) {
    let mut next_by_cycle = vec![vec![None; NCYCLES]; NFACES];
    let mut previous_by_cycle = vec![vec![None; NCYCLES]; NFACES];

    // Handle regular faces (not outer or inner)
    for face_id in 1..(NFACES - 1) {
        let face = &mut faces[face_id];
        let face_colors = face.colors;

        for cycle_id in 0..NCYCLES {
            let cycle = cycles.get(cycle_id as u64);
            let cycle_colors = cycle.colorset();

            // Check if cycle is valid for this face
            if !is_cycle_valid_for_face(cycle_colors, face_colors) {
                face.possible_cycles.remove(cycle_id as u64);
                continue;
            }

            // Check for exactly two edge transitions
            if let Some((prev_face, next_face)) = check_exactly_two_transitions(cycle, face_colors)
            {
                // Valid monotone cycle - record adjacency
                next_by_cycle[face_id][cycle_id] = Some(next_face);
                previous_by_cycle[face_id][cycle_id] = Some(prev_face);
            } else {
                // Invalid - remove from possible cycles
                face.possible_cycles.remove(cycle_id as u64);
            }
        }
    }

    // Handle outer face (0): Can only have full NCOLORS-cycles
    // Forms a cycle of length 1 (points to itself)
    filter_cycles_by_length(&mut faces[0], cycles, NCOLORS);
    for cycle_id in 0..NCYCLES {
        if faces[0].possible_cycles.contains(cycle_id as u64) {
            next_by_cycle[0][cycle_id] = Some(0); // Points to itself
            previous_by_cycle[0][cycle_id] = Some(0);
        }
    }

    // Handle inner face (NFACES-1): Can only have full NCOLORS-cycles
    // Forms a cycle of length 1 (points to itself)
    // The inner face will be assigned cycle (0,1,2,3,4,5) during search for symmetry breaking
    filter_cycles_by_length(&mut faces[NFACES - 1], cycles, NCOLORS);
    for cycle_id in 0..NCYCLES {
        if faces[NFACES - 1].possible_cycles.contains(cycle_id as u64) {
            next_by_cycle[NFACES - 1][cycle_id] = Some(NFACES - 1); // Points to itself
            previous_by_cycle[NFACES - 1][cycle_id] = Some(NFACES - 1);
        }
    }

    (next_by_cycle, previous_by_cycle)
}

/// Filter a face to only allow cycles of a specific length.
///
/// This is used for the outer and inner faces, which can only have
/// cycles that use all NCOLORS colors.
fn filter_cycles_by_length(face: &mut Face, cycles: &crate::memo::CyclesArray, length: usize) {
    for cycle_id in 0..NCYCLES {
        let cycle = cycles.get(cycle_id as u64);
        if cycle.len() != length {
            face.possible_cycles.remove(cycle_id as u64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binomial_coefficients() {
        let coeffs = compute_binomial_coefficients();

        // C(n, 0) = 1
        assert_eq!(coeffs[0], 1);

        // C(n, n) = 1
        assert_eq!(coeffs[NCOLORS], 1);

        // For NCOLORS=6:
        // C(6,1)=6, C(6,2)=15, C(6,3)=20, C(6,4)=15, C(6,5)=6
        #[cfg(any(
            feature = "ncolors_6",
            not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
        ))]
        {
            assert_eq!(coeffs[1], 6);
            assert_eq!(coeffs[2], 15);
            assert_eq!(coeffs[3], 20);
            assert_eq!(coeffs[4], 15);
            assert_eq!(coeffs[5], 6);
        }

        // For NCOLORS=3:
        // C(3,1)=3, C(3,2)=3
        #[cfg(feature = "ncolors_3")]
        {
            assert_eq!(coeffs[1], 3);
            assert_eq!(coeffs[2], 3);
        }
    }

    #[test]
    fn test_create_face_color_mapping() {
        // Face 0 = outer face (no colors)
        let face0 = create_face_with_edges(0);
        assert_eq!(face0.id, 0);
        assert_eq!(face0.colors.len(), 0);

        // Face 1 = {color 0}
        let face1 = create_face_with_edges(1);
        assert_eq!(face1.id, 1);
        assert_eq!(face1.colors.len(), 1);
        assert!(face1.colors.contains(Color::new(0)));

        // Face 3 = {color 0, color 1}
        let face3 = create_face_with_edges(3);
        assert_eq!(face3.id, 3);
        assert_eq!(face3.colors.len(), 2);
        assert!(face3.colors.contains(Color::new(0)));
        assert!(face3.colors.contains(Color::new(1)));

        // Face NFACES-1 = inner face (all colors)
        let face_inner = create_face_with_edges(NFACES - 1);
        assert_eq!(face_inner.id, NFACES - 1);
        assert_eq!(face_inner.colors.len(), NCOLORS);
    }

    #[test]
    fn test_faces_memo_initialization() {
        let cycles = crate::memo::CyclesArray::generate();
        let memo = FacesMemo::initialize(&cycles);

        // Should create exactly NFACES faces
        assert_eq!(memo.faces.len(), NFACES);

        // Outer face should exist
        let outer = memo.get_face(0);
        assert_eq!(outer.colors.len(), 0);

        // Inner face should exist
        let inner = memo.get_face(NFACES - 1);
        assert_eq!(inner.colors.len(), NCOLORS);

        // Binomial coefficients should be computed
        assert_eq!(memo.face_degree_by_color_count[0], 1);
        assert_eq!(memo.face_degree_by_color_count[NCOLORS], 1);
    }

    #[test]
    fn test_outer_and_inner_face_cycle_constraints() {
        let cycles = crate::memo::CyclesArray::generate();
        let memo = FacesMemo::initialize(&cycles);

        // Outer face (0) can only have NCOLORS-length cycles
        let outer = memo.get_face(0);
        for cycle_id in 0..NCYCLES as u64 {
            let cycle = cycles.get(cycle_id);
            if outer.possible_cycles.contains(cycle_id) {
                assert_eq!(
                    cycle.len(),
                    NCOLORS,
                    "Outer face cycle {} has wrong length {}",
                    cycle_id,
                    cycle.len()
                );
            }
        }

        // Inner face (NFACES-1) can only have NCOLORS-length cycles
        let inner = memo.get_face(NFACES - 1);
        for cycle_id in 0..NCYCLES as u64 {
            let cycle = cycles.get(cycle_id);
            if inner.possible_cycles.contains(cycle_id) {
                assert_eq!(
                    cycle.len(),
                    NCOLORS,
                    "Inner face cycle {} has wrong length {}",
                    cycle_id,
                    cycle.len()
                );
            }
        }

        // Both should have at least one possible cycle
        assert!(
            !outer.possible_cycles.is_empty(),
            "Outer face has no possible cycles"
        );
        assert!(
            !inner.possible_cycles.is_empty(),
            "Inner face has no possible cycles"
        );
    }
}
