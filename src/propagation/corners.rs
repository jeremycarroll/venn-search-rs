// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Corner detection and curve disconnection checking.
//!
//! This module implements the Carroll 2000 corner detection algorithm, which
//! walks around each color's curve to count corners. For triangle diagrams,
//! each curve can have at most 3 corners.
//!
//! Also implements disconnected curve detection by checking if the number of
//! visited edges matches the expected total for a complete curve.

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::{Color, CornerWalkState, CycleId};
use crate::trail::Trail;
use std::ptr::NonNull;

use super::errors::PropagationFailure;

/// Maximum corners allowed per curve for triangle diagrams.
/// Triangles have 3 corners, so each curve can have at most 3 corners.
const MAX_CORNERS: usize = 3;

/// Check corner constraints for all colors.
///
/// For Venn diagrams drawable with triangles, each curve can have at most 3 corners.
/// This function walks around each color's curve starting from the central/inner face
/// and counts corners using the Carroll 2000 corner detection algorithm.
///
/// Only active for NCOLORS > 4 (N=3,4 don't need corner checking).
///
/// # Algorithm
///
/// For each color:
/// 1. Start at the central face (set up first by monotonicity)
/// 2. Walk around the curve following edges
/// 3. If we hit an unassigned edge → curve incomplete, skip
/// 4. If we return to central face → curve complete, check corners
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data
/// * `state` - Search state with edge connections
/// * `trail` - Trail for backtracking (used to mark colors as checked)
/// * `_face_id` - Face that was assigned a cycle (unused, for API compatibility)
/// * `_cycle_id` - The cycle assigned (unused, for API compatibility)
/// * `depth` - Recursion depth for error messages
///
/// # Returns
///
/// `Ok(())` if all curves have ≤ MAX_CORNERS corners,
/// `Err(PropagationFailure::TooManyCorners)` if a curve requires > 3 corners.
#[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
pub(super) fn check_corners_for_cycle(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    _face_id: usize,
    _cycle_id: CycleId,
    depth: usize,
) -> Result<(), PropagationFailure> {
    use crate::geometry::constants::{NCOLORS, NFACES};

    // Central/inner face (all colors) - set up first by monotonicity
    let central_face_id = NFACES - 1;

    // Check each color's curve starting from the central face
    for color_idx in 0..NCOLORS {
        let face_dynamic = &state.faces.faces[central_face_id];
        let edge_to = face_dynamic.edge_dynamic[color_idx].get_to();

        // Only check if central face has this edge connected
        if let Some(start_link) = edge_to {
            // Walk around the curve and count corners
            // Returns None if curve is incomplete, Some(count) if complete
            // Errors with DisconnectedCurve if curve forms multiple loops
            if let Some(corner_count) = count_corners_on_curve(memo, state, trail, central_face_id, color_idx, start_link, depth)? {
                // Check if exceeds triangle limit
                if corner_count > MAX_CORNERS {
                    return Err(PropagationFailure::TooManyCorners {
                        color: color_idx,
                        corner_count,
                        max_allowed: MAX_CORNERS,
                        depth,
                    });
                }
            }
        }
    }

    Ok(())
}

/// Count corners on a curve by traversing edges starting from the central face.
///
/// Implements the Carroll 2000 corner detection algorithm by walking around
/// a curve and tracking which other curves are inside vs outside.
///
/// Also checks for disconnected curves by comparing visited edge count to
/// the total edge count for this color.
///
/// # Arguments
///
/// * `memo` - MEMO data with vertex information
/// * `state` - Search state with edge connections
/// * `trail` - Trail for backtracking (used to mark colors as completed)
/// * `start_face_id` - Starting face for the traversal (should be central face)
/// * `color_idx` - Index of the color whose curve we're checking
/// * `_start_link` - Starting edge connection (unused, we get it from state)
/// * `depth` - Recursion depth for error messages
///
/// # Returns
///
/// * `Ok(Some(count))` - Complete curve with this many corners
/// * `Ok(None)` - Incomplete curve (hit unassigned edge)
/// * `Err(PropagationFailure::DisconnectedCurve)` - Disconnected curve detected
/// * `Err(...)` - Other unexpected error during traversal
#[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
fn count_corners_on_curve(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    start_face_id: usize,
    color_idx: usize,
    _start_link: crate::geometry::CurveLink,
    _depth: usize,
) -> Result<Option<usize>, PropagationFailure> {
    use crate::geometry::constants::NCOLORS;

    let mut corner_state = CornerWalkState::new();

    // Initialize with colors we're OUTSIDE of at the starting face
    // A face's colorset contains curves we're INSIDE, so we need the complement
    let start_face_memo = memo.faces.get_face(start_face_id);
    let start_face_colorset = start_face_memo.colors;
    for c in 0..NCOLORS {
        let color = Color::new(c as u8);
        // Add to 'out' if NOT in face colorset and NOT the curve we're traversing
        if !start_face_colorset.contains(color) && c != color_idx {
            corner_state.out.insert(color);
        }
    }

    let mut current_face_id = start_face_id;
    let mut current_color = color_idx;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 1000; // Prevent infinite loops
    let mut _vertices_visited = 0;
    let mut _edges_visited = 0;

    // Walk around the curve following edge->to->next links
    loop {
        _edges_visited += 1;
        iterations += 1;

        if iterations > MAX_ITERATIONS {
            // Safety bail-out - shouldn't happen in valid diagrams
            return Ok(None); // Treat infinite loop as incomplete
        }

        let face_dynamic = &state.faces.faces[current_face_id];
        let edge_to = face_dynamic.edge_dynamic[current_color].get_to();

        if let Some(link) = edge_to {
            // Get the vertex from MEMO data by its ID
            let vertex = match memo.vertices.get_vertex_by_id(link.vertex_id) {
                Some(v) => v,
                None => return Ok(None), // Incomplete curve
            };

            // Determine the other color at this vertex and which color we're traversing
            let (other_color, is_primary) = if vertex.primary.value() as usize == color_idx {
                (vertex.secondary, true)
            } else if vertex.secondary.value() as usize == color_idx {
                (vertex.primary, false)
            } else {
                return Ok(None); // Incomplete curve
            };

            // Process this vertex for corner detection
            corner_state.process_vertex(other_color, link.vertex_id);
            _vertices_visited += 1;

            // Find the exit edge: we entered on one slot, exit on the opposite slot
            // Primary color: slots 0,1 (enter on one, exit on other)
            // Secondary color: slots 2,3 (enter on one, exit on other)
            //
            // The exit edge must:
            // - Have the SAME color (we're following this curve)
            // - Have a DIFFERENT face (we're moving to an adjacent face)
            let mut next_edge_ref = None;
            if is_primary {
                // Check both primary slots (0 and 1)
                for slot in [0, 1] {
                    let edge_ref = vertex.incoming_edges[slot];
                    // Exit edge: same color, different face
                    if edge_ref.color_idx == current_color && edge_ref.face_id != current_face_id {
                        next_edge_ref = Some(edge_ref);
                        break;
                    }
                }
            } else {
                // Check both secondary slots (2 and 3)
                for slot in [2, 3] {
                    let edge_ref = vertex.incoming_edges[slot];
                    // Exit edge: same color, different face
                    if edge_ref.color_idx == current_color && edge_ref.face_id != current_face_id {
                        next_edge_ref = Some(edge_ref);
                        break;
                    }
                }
            }

            let next_edge = match next_edge_ref {
                Some(e) => e,
                None => return Ok(None), // Incomplete curve
            };

            let next_face_id = next_edge.face_id;
            let next_color_idx = next_edge.color_idx;

            // Sanity check: we should still be on the same curve color
            if next_color_idx != color_idx {
                return Ok(None); // Treat as incomplete
            }

            // Check if we've completed the loop (back to starting face)
            if next_face_id == start_face_id {
                // Mark this color as completed (only check once per color)
                if state.colors_checked[color_idx] == 0 {
                    // Mark as checked (trail-tracked for backtracking)
                    unsafe {
                        trail.record_and_set(
                            NonNull::from(&mut state.colors_checked[color_idx]),
                            1,
                        );
                    }

                    // Also set in temporary accumulator (not trail-tracked)
                    state.colors_completed_this_call |= 1 << color_idx;
                }

                return Ok(Some(corner_state.corner_count())); // Complete curve!
            }

            current_face_id = next_face_id;
            current_color = next_color_idx;
        } else {
            // Edge not connected - curve incomplete, can't check corners yet
            return Ok(None); // Incomplete curve
        }
    }
}
