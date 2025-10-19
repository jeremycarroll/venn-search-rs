// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Vertex configuration and linking.
//!
//! This module handles setting up edge->to pointers to connect edges to vertices,
//! and enforces the triangle constraint by counting crossings at vertices.

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::{CycleId, EdgeDynamic, MAX_CROSSINGS_PER_PAIR};
use crate::trail::Trail;
use std::ptr::NonNull;

use super::corners::check_corners_for_cycle;
use super::errors::PropagationFailure;

/// Check and configure vertices for a face's assigned cycle.
///
/// For each consecutive pair of colors in the cycle, this function:
/// 1. Retrieves the precomputed vertex from the vertex array
/// 2. Sets the edge->to pointer (trail-tracked) to connect to that vertex
/// 3. Validates vertex configuration compatibility
/// 4. **Corner Detection**: Counts crossings at vertices for triangle constraint
///
/// # Corner Detection Algorithm
///
/// Each vertex represents a crossing between two color curves. When we encounter
/// a vertex for the first time (not already processed), we:
/// 1. Increment the crossing count for that color pair
/// 2. Mark the vertex as processed to avoid double-counting
/// 3. Check if crossing count exceeds MAX_CROSSINGS_PER_PAIR (6 for triangles)
///
/// This enforces the triangle constraint during search, pruning the search space
/// from ~30,000 configurations to the actual 152 (N=5) or 233 (N=6) solutions.
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data (contains vertex array)
/// * `state` - Mutable search state (contains edge_dynamic arrays, crossing counts, vertex tracking)
/// * `trail` - Trail for backtracking
/// * `face_id` - Face that was assigned a cycle
/// * `cycle_id` - The cycle assigned to this face
/// * `depth` - Recursion depth for error messages
///
/// # Returns
///
/// `Ok(())` if all vertices are valid and crossing limits not exceeded,
/// `Err(PropagationFailure::CrossingLimitExceeded)` if triangle constraint violated.
pub(super) fn check_face_vertices(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    cycle_id: CycleId,
    depth: usize,
) -> Result<(), PropagationFailure> {
    let cycle = memo.cycles.get(cycle_id);
    let cycle_colors = cycle.colors();
    let face_memo = memo.faces.get_face(face_id);

    // For each consecutive pair of colors in the cycle (including wrap-around)
    for i in 0..cycle.len() {
        let next_i = (i + 1) % cycle.len();
        let color_a = cycle_colors[i];
        let color_b = cycle_colors[next_i];
        let color_a_idx = color_a.value() as usize;
        let color_b_idx = color_b.value() as usize;

        // Get the edge for color_a
        let edge_memo = &face_memo.edges[color_a_idx];

        // Check if edge already has a vertex assigned
        let face_dynamic = &state.faces.faces[face_id];
        let existing_to = face_dynamic.edge_dynamic[color_a_idx].get_to();

        if let Some(existing_link) = existing_to {
            // Edge already configured - validate compatibility
            let expected_link = edge_memo.possibly_to[color_b_idx];

            if let Some(expected) = expected_link {
                if existing_link.vertex_id != expected.vertex_id {
                    // Conflicting vertex assignments - this is OK, just skip
                    // (monotonicity filter may have already set a different valid vertex)
                    continue;
                }
            }
            continue; // Edge already set up correctly
        }

        // Get the vertex connection from possibly_to
        // This was populated during MemoizedData initialization by FacesMemo::populate_vertex_links()
        let vertex_link = edge_memo.possibly_to[color_b_idx];

        if let Some(link) = vertex_link {
            // Set edge->to_encoded pointer (trail-tracked)
            let encoded = EdgeDynamic::encode_to(Some(link));
            unsafe {
                trail.record_and_set(
                    NonNull::from(
                        &mut state.faces.faces[face_id].edge_dynamic[color_a_idx].to_encoded,
                    ),
                    encoded,
                );
            }

            // Increment edge count for this color and direction (trail-tracked)
            // Direction 0 = clockwise (face contains the color)
            // Direction 1 = counterclockwise (face doesn't contain the color)
            let face_colors = face_memo.colors;
            let direction = if face_colors.contains(color_a) { 0 } else { 1 };
            let current_count = state.edge_color_counts[direction][color_a_idx];
            unsafe {
                trail.record_and_set(
                    NonNull::from(&mut state.edge_color_counts[direction][color_a_idx]),
                    current_count + 1,
                );
            }

            // Corner detection: Count crossing at this vertex
            let vertex_id = link.vertex_id;

            // Check if vertex already processed
            if vertex_id < state.vertex_processed.len() && state.vertex_processed[vertex_id] == 0 {
                // First time seeing this vertex - count the crossing

                // Normalize color pair to upper triangle (i < j)
                let (color_i, color_j) = if color_a_idx < color_b_idx {
                    (color_a_idx, color_b_idx)
                } else {
                    (color_b_idx, color_a_idx)
                };

                // Increment crossing count (trail-tracked)
                let current_count = state.crossing_counts.get(color_i, color_j);
                let new_count = current_count + 1;

                unsafe {
                    let ptr = state.crossing_counts.get_mut_ptr(color_i, color_j);
                    trail.record_and_set(NonNull::new_unchecked(ptr), new_count);
                }

                // Check triangle constraint (max 6 crossings per pair)
                if new_count as usize > MAX_CROSSINGS_PER_PAIR {
                    return Err(PropagationFailure::CrossingLimitExceeded {
                        color_i,
                        color_j,
                        count: new_count as usize,
                        max_allowed: MAX_CROSSINGS_PER_PAIR,
                        depth,
                    });
                }

                // Mark vertex as processed (trail-tracked)
                unsafe {
                    trail.record_and_set(NonNull::from(&mut state.vertex_processed[vertex_id]), 1);
                }
            }
        }
        // If vertex_link is None, that's OK - not all edges may have vertices
        // assigned yet (this is the DYNAMIC phase)
    }

    // Corner checking (only for NCOLORS > 4)
    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
    check_corners_for_cycle(memo, state, trail, face_id, cycle_id, depth)?;

    Ok(())
}
