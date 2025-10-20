// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Corner detection for triangle constraint validation.
//!
//! This module implements the Carroll 2000 corner detection algorithm from
//! C code vertex.c. It counts corners on curves using Out/Passed set tracking.
//!
//! **This is separate from:**
//! - Crossing limit checks (in vertices.rs)
//! - Disconnected curve checks (in curve_disconnection.rs)
//!
//! # Algorithm Overview
//!
//! From C vertex.c:57-73, 100-124, 180-191:
//!
//! 1. `detect_corner_and_update_crossing_sets()` - Core algorithm
//!    - Maintains "Out" set (colors currently outside)
//!    - Maintains "Passed" set (colors we've passed)
//!    - Returns true when corner detected
//!
//! 2. `find_corners_by_traversal()` - Walk curve counting corners
//!    - Starts from an edge on central face
//!    - Follows edge->to->next links
//!    - Calls detect_corner at each vertex
//!    - Fails if > MAX_CORNERS (3) corners needed
//!
//! 3. `vertex_corner_check()` - Entry point
//!    - For NCOLORS <= 4: no-op (always succeeds)
//!    - For NCOLORS > 4: validates curve has ≤3 corners
//!
//! # When Called
//!
//! From C venn.c:54 in dynamicCheckEdgeCurvesAndCorners():
//! Called for each edge when a facial cycle is assigned to a face.

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::{constants::NFACES, ColorSet};
use crate::trail::Trail;

use super::errors::PropagationFailure;

/// Maximum corners allowed per curve for triangle diagrams.
const MAX_CORNERS: usize = 3;

/// Core corner detection algorithm that updates crossing sets.
///
/// Ported from C vertex.c:57-73.
///
/// This implements the Carroll 2000 corner detection algorithm using
/// "Out" and "Passed" set tracking. Returns true if a corner is detected.
///
/// # Algorithm
///
/// - If `other` intersects `outside`: Remove `other` from `outside`
///   - If `other` also intersects `passed`: Clear `passed` and return true (corner!)
/// - Otherwise: Add `other` to both `outside` and `passed`
///
/// # Arguments
///
/// * `other` - The other color at this vertex
/// * `outside` - Mutable reference to the "outside" color set
/// * `passed` - Mutable reference to the "passed" color set
///
/// # Returns
///
/// `true` if a corner is detected, `false` otherwise.
fn detect_corner_and_update_crossing_sets(
    other: ColorSet,
    outside: &mut ColorSet,
    passed: &mut ColorSet,
) -> bool {
    // C: if (other & *outside)
    let other_bits = other.bits();
    let outside_bits = outside.bits();

    if (other_bits & outside_bits) != 0 {
        // C: *outside &= ~other;
        *outside = ColorSet::from_bits(outside_bits & !other_bits);

        // C: if (other & *passed)
        let passed_bits = passed.bits();
        if (other_bits & passed_bits) != 0 {
            // C: *passed = 0; return true;
            *passed = ColorSet::empty();
            return true;
        }
    } else {
        // C: *passed |= other; *outside |= other;
        let passed_bits = passed.bits();
        *passed = ColorSet::from_bits(passed_bits | other_bits);
        *outside = ColorSet::from_bits(outside_bits | other_bits);
    }

    // C: return false;
    false
}

/// Finds corners by traversing edges from a starting point.
///
/// Ported from C vertex.c:100-124.
///
/// Walks around a curve following edge->to->next links, calling
/// detect_corner_and_update_crossing_sets at each vertex.
/// Fails if more than MAX_CORNERS (3) corners are needed.
///
/// # Arguments
///
/// * `start_face_id` - Face containing the starting edge
/// * `start_color_idx` - Color index of the starting edge
/// * `depth` - Recursion depth for error messages
/// * `memo` - Immutable MEMO data
/// * `state` - Search state with edge connections
///
/// # Returns
///
/// `Ok(())` if curve has ≤ MAX_CORNERS corners,
/// `Err(PropagationFailure::TooManyCorners)` if > MAX_CORNERS needed.
fn find_corners_by_traversal(
    start_face_id: usize,
    start_color_idx: usize,
    depth: usize,
    memo: &MemoizedData,
    state: &DynamicState,
) -> Result<(), PropagationFailure> {
    // Get starting edge info
    let start_face_colors = memo.faces.get_face(start_face_id).colors;

    // C: COLORSET notMyColor = ~(1u << start->color)
    let not_my_color_bits = !(1u64 << start_color_idx);

    // C: outside = ~start->colors
    // IMPORTANT: Mask to only valid color bits (0..NCOLORS)
    use crate::geometry::constants::NCOLORS;
    let all_colors_mask = (1u64 << NCOLORS) - 1;
    let mut outside = ColorSet::from_bits((!start_face_colors.bits()) & all_colors_mask);

    // C: passed = 0
    let mut passed = ColorSet::empty();

    let mut counter = 0;
    let mut current_face_id = start_face_id;
    let mut current_color_idx = start_color_idx;

    // C: assert(start->reversed->to == NULL ||
    //          (start->colors & notMyColor) == ((NFACES - 1) & notMyColor));
    // This checks that if reversed edge has a 'to', then face is central (all colors except curve color)
    #[cfg(debug_assertions)]
    {
        let adjacent_face_id = memo.faces.get_face(start_face_id).adjacent_faces[start_color_idx];
        let reversed_has_to = state.faces.faces[adjacent_face_id].edge_dynamic[start_color_idx]
            .get_to()
            .is_some();
        if reversed_has_to {
            let expected_central_face = (NFACES - 1) & not_my_color_bits as usize;
            let actual_face_masked = start_face_colors.bits() & not_my_color_bits;
            debug_assert_eq!(
                actual_face_masked, expected_central_face as u64,
                "If reversed edge has 'to', face must be central"
            );
        }
    }

    // C: do { ... } while (current->to != NULL && current != start);
    loop {
        // C: CURVELINK p = current->to;
        let edge_to = state.faces.faces[current_face_id].edge_dynamic[current_color_idx].get_to();

        match edge_to {
            None => {
                break; // current->to == NULL
            }
            Some(link) => {
                // Get vertex - C: p->vertex
                if let Some(vertex) = memo.vertices.get_vertex_by_id(link.vertex_id) {
                    // C: p->vertex->colors & notMyColor
                    let vertex_colors_masked =
                        ColorSet::from_bits(vertex.colors.bits() & not_my_color_bits);

                    // C: if (detectCornerAndUpdateCrossingSets(...))
                    if detect_corner_and_update_crossing_sets(
                        vertex_colors_masked,
                        &mut outside,
                        &mut passed,
                    ) {
                        counter += 1;

                        // C checks BEFORE incrementing: if (counter >= MAX_CORNERS) fail
                        // We increment FIRST, so we check counter > MAX_CORNERS
                        if counter > MAX_CORNERS {
                            return Err(PropagationFailure::TooManyCorners {
                                color: start_color_idx,
                                corner_count: counter,
                                max_allowed: MAX_CORNERS,
                                depth,
                            });
                        }
                    }
                }

                // C: current = p->next;
                current_face_id = link.next.face_id;
                current_color_idx = link.next.color_idx;

                // Check if we're back at start (loop complete)
                if current_face_id == start_face_id && current_color_idx == start_color_idx {
                    break;
                }
            }
        }
    }

    // C: return NULL; (success)
    Ok(())
}

/// Check if a curve requires more than MAX_CORNERS corners.
///
/// Ported from C vertex.c:180-191.
///
/// This is called during the main Venn diagram search for each edge
/// when a facial cycle is assigned to a face.
///
/// For NCOLORS <= 4, this always succeeds (no check needed).
/// For NCOLORS > 4, this validates the curve can be drawn with ≤3 corners.
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data
/// * `state` - Search state with edge connections
/// * `_trail` - Trail for backtracking (unused here)
/// * `face_id` - Face containing the edge to check
/// * `color_idx` - Color index of the edge to check
/// * `depth` - Recursion depth for error messages
///
/// # Returns
///
/// `Ok(())` if curve has ≤ MAX_CORNERS corners,
/// `Err(PropagationFailure::TooManyCorners)` if > MAX_CORNERS needed.
pub fn vertex_corner_check(
    memo: &MemoizedData,
    state: &DynamicState,
    _trail: &mut Trail,
    face_id: usize,
    color_idx: usize,
    depth: usize,
) -> Result<(), PropagationFailure> {
    // C: #if NCOLORS <= 4 return NULL;
    #[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
    {
        // For NCOLORS <= 4, no corner check needed
        let _ = (memo, state, face_id, color_idx, depth);
        Ok(())
    }

    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
    {
        // C: EDGE start = ...
        let start_face_id = face_id;
        let start_color_idx = color_idx;

        // C: if (start->reversed->to != NULL) start = vertexGetCentralEdge(start->color);
        // Check if the reversed edge (on adjacent face) has a 'to' connection
        let adjacent_face_id = memo.faces.get_face(face_id).adjacent_faces[color_idx];
        let reversed_has_to = state.faces.faces[adjacent_face_id].edge_dynamic[color_idx]
            .get_to()
            .is_some();

        // If curve is complete (reversed edge connected) and we're NOT on the central face,
        // start from the central face instead to traverse the complete curve
        let final_start_face = if reversed_has_to && face_id != NFACES - 1 {
            NFACES - 1 // Central/inner face
        } else {
            start_face_id
        };

        // C: return findCornersByTraversal(start, depth, ignore);
        find_corners_by_traversal(final_start_face, start_color_idx, depth, memo, state)
    }
}

/// Count corners for a complete curve (for testing/validation).
///
/// This function is similar to `find_corners_by_traversal` but returns the actual
/// corner count instead of a Result. It should only be called on complete curves.
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data
/// * `state` - Search state with edge connections
/// * `start_face_id` - Face containing the starting edge (typically central face)
/// * `start_color_idx` - Color index of the curve to count
///
/// # Returns
///
/// The number of corners found on the curve.
pub fn count_corners_for_complete_curve(
    memo: &MemoizedData,
    state: &DynamicState,
    start_face_id: usize,
    start_color_idx: usize,
) -> usize {
    // Get starting edge info
    let start_face_colors = memo.faces.get_face(start_face_id).colors;

    // C: COLORSET notMyColor = ~(1u << start->color)
    let not_my_color_bits = !(1u64 << start_color_idx);

    // C: outside = ~start->colors
    // IMPORTANT: Mask to only valid color bits (0..NCOLORS)
    use crate::geometry::constants::NCOLORS;
    let all_colors_mask = (1u64 << NCOLORS) - 1;
    let mut outside = ColorSet::from_bits((!start_face_colors.bits()) & all_colors_mask);

    // C: passed = 0
    let mut passed = ColorSet::empty();

    let mut counter = 0;
    let mut current_face_id = start_face_id;
    let mut current_color_idx = start_color_idx;

    // Walk the curve
    loop {
        // Get edge->to
        let edge_to = state.faces.faces[current_face_id].edge_dynamic[current_color_idx].get_to();

        match edge_to {
            None => {
                break; // current->to == NULL
            }
            Some(link) => {
                // Get vertex and check for corner
                if let Some(vertex) = memo.vertices.get_vertex_by_id(link.vertex_id) {
                    let vertex_colors_masked =
                        ColorSet::from_bits(vertex.colors.bits() & not_my_color_bits);

                    if detect_corner_and_update_crossing_sets(
                        vertex_colors_masked,
                        &mut outside,
                        &mut passed,
                    ) {
                        counter += 1;
                    }
                }

                // Move to next edge
                current_face_id = link.next.face_id;
                current_color_idx = link.next.color_idx;

                // Check if we're back at start (loop complete)
                if current_face_id == start_face_id && current_color_idx == start_color_idx {
                    break;
                }
            }
        }
    }

    counter
}
