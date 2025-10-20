// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Disconnected curve detection during Venn diagram search.
//!
//! This module implements disconnected curve checking from C code edge.c.
//! It detects when a curve forms multiple separate components instead of
//! a single connected loop.
//!
//! **This is separate from:**
//! - Crossing limit checks (in vertices.rs)
//! - Corner detection checks (in corner_detection.rs)
//!
//! # Algorithm Overview
//!
//! From C edge.c:24-52, 54-83:
//!
//! 1. `curve_length()` - Count edges in curve by following links
//!    - Starts from an edge
//!    - Follows edge->to->next until back at start
//!    - Returns count
//!
//! 2. `check_for_disconnected_curve()` - Compare curve length to total
//!    - If edge->reversed->to != NULL (closed curve):
//!      - Count curve length
//!      - Compare to EdgeColorCountState (total edges assigned)
//!      - If length < count → disconnected!
//!      - If equal → mark color as complete
//!    - This detects separate components
//!
//! 3. `find_start_of_curve()` - Find starting edge
//!    - Walks backwards to find start (or detects loop)
//!
//! 4. `edge_curve_checks()` - Entry point
//!    - If curve already complete, skip
//!    - Otherwise check for disconnection
//!
//! # When Called
//!
//! From C venn.c:53 in dynamicCheckEdgeCurvesAndCorners():
//! Called for each edge when a facial cycle is assigned to a face.

use crate::context::{DynamicState, MemoizedData};
use crate::trail::Trail;
use std::ptr::NonNull;

use super::errors::PropagationFailure;

/// Follow an edge forward (edge->to->next).
///
/// Returns None if edge->to is NULL (edge not connected).
/// Otherwise returns the next edge in the curve.
#[allow(dead_code)]
fn edge_follow_forwards(
    face_id: usize,
    color_idx: usize,
    state: &DynamicState,
) -> Option<(usize, usize)> {
    // C: if (edge->to == NULL) return NULL;
    let edge_to = state.faces.faces[face_id].edge_dynamic[color_idx].get_to()?;

    // C: return edge->to->next;
    Some((edge_to.next.face_id, edge_to.next.color_idx))
}

/// Follow an edge backward (find edge that leads to this one).
///
/// Returns None if can't go backwards (edge->reversed->to is NULL).
/// Otherwise returns the previous edge in the curve.
#[allow(dead_code)]
fn edge_follow_backwards(
    face_id: usize,
    color_idx: usize,
    memo: &MemoizedData,
    state: &DynamicState,
) -> Option<(usize, usize)> {
    // C: EDGE edgeFollowBackwards(EDGE edge) {
    //      EDGE reversedNext = edgeFollowForwards(edge->reversed);
    //      return reversedNext == NULL ? NULL : reversedNext->reversed;
    //    }

    // Get reversed edge (on adjacent face, same color)
    let adjacent_face_id = memo.faces.get_face(face_id).adjacent_faces[color_idx];

    // Follow forwards from reversed edge
    let (next_face, _next_color) = edge_follow_forwards(adjacent_face_id, color_idx, state)?;

    // Return its reverse (back to original direction)
    // NOTE: We return color_idx (the original color) because we're following the SAME color curve
    let prev_adjacent = memo.faces.get_face(next_face).adjacent_faces[color_idx];
    Some((prev_adjacent, color_idx))
}

/// Count the number of edges in a curve by following edge->to->next links.
///
/// Ported from C edge.c:24-33.
///
/// # Arguments
///
/// * `start_face_id` - Face containing the starting edge
/// * `start_color_idx` - Color of the edge
/// * `state` - Search state with edge connections
///
/// # Returns
///
/// Number of edges in the curve (starting from this edge).
#[allow(dead_code)]
fn curve_length(start_face_id: usize, start_color_idx: usize, state: &DynamicState) -> usize {
    // C: for (result = 1, current = edgeFollowForwards(edge); current != edge;
    //         result++, current = edgeFollowForwards(current)) {
    //      assert(current != NULL);
    //    }
    //    return result;

    let mut result = 1;
    let mut current = edge_follow_forwards(start_face_id, start_color_idx, state);

    while let Some((face_id, color_idx)) = current {
        // Check if back at start
        if face_id == start_face_id && color_idx == start_color_idx {
            break;
        }

        result += 1;
        current = edge_follow_forwards(face_id, color_idx, state);

        // C code has assert(current != NULL), but we handle None by breaking
        if current.is_none() {
            break;
        }
    }

    result
}

/// Find the starting edge of a curve (or detect it's a loop).
///
/// Ported from C edge.c:54-64.
///
/// Walks backwards until finding the start (can't go further back)
/// or detecting a loop (back at original edge).
///
/// # Arguments
///
/// * `face_id` - Face containing the edge
/// * `color_idx` - Color of the edge
/// * `memo` - Immutable MEMO data
/// * `state` - Search state with edge connections
///
/// # Returns
///
/// (face_id, color_idx) of the starting edge.
#[allow(dead_code)]
fn find_start_of_curve(
    face_id: usize,
    color_idx: usize,
    memo: &MemoizedData,
    state: &DynamicState,
) -> (usize, usize) {
    // C: while ((next = edgeFollowBackwards(current)) != edge) {
    //      if (next == NULL) {
    //        return current;
    //      }
    //      current = next;
    //    }
    //    return edge;

    let start_face = face_id;
    let start_color = color_idx;
    let mut current_face = face_id;
    let mut current_color = color_idx;

    while let Some((next_face, next_color)) =
        edge_follow_backwards(current_face, current_color, memo, state)
    {
        // Check if back at original edge (loop detected)
        if next_face == start_face && next_color == start_color {
            break;
        }

        current_face = next_face;
        current_color = next_color;
    }

    (current_face, current_color)
}

/// Check if a curve forms a single connected component.
///
/// Ported from C edge.c:35-52.
///
/// Compares the curve length (by following edges) to the total number of
/// edges assigned for this color. If they don't match, the curve is
/// disconnected (forms multiple separate loops).
///
/// # Arguments
///
/// * `face_id` - Face containing the edge
/// * `color_idx` - Color of the edge
/// * `depth` - Recursion depth for error messages
/// * `memo` - Immutable MEMO data
/// * `state` - Mutable search state
/// * `trail` - Trail for tracking completed colors
///
/// # Returns
///
/// `Ok(())` if curve is connected or incomplete,
/// `Err(PropagationFailure::DisconnectedCurve)` if disconnected.
#[allow(dead_code)]
fn check_for_disconnected_curve(
    face_id: usize,
    color_idx: usize,
    depth: usize,
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
) -> Result<(), PropagationFailure> {
    // C: if (edge->reversed->to != NULL)
    // Check if reversed edge is connected (forming a closed curve)
    let adjacent_face_id = memo.faces.get_face(face_id).adjacent_faces[color_idx];
    let reversed_has_to = state.faces.faces[adjacent_face_id].edge_dynamic[color_idx]
        .get_to()
        .is_some();

    eprintln!(
        "DEBUG check_for_disconnected: face={}, color={}, adjacent={}, reversed_has_to={}",
        face_id, color_idx, adjacent_face_id, reversed_has_to
    );

    if reversed_has_to {
        // We have a colored cycle in the FISC
        // C: length = curveLength(edge);
        eprintln!(
            "DEBUG: Computing curve_length from face={}, color={}",
            face_id, color_idx
        );
        let length = curve_length(face_id, color_idx, state);
        eprintln!("DEBUG: curve_length={}", length);

        // C: if (length < EdgeColorCountState[IS_CLOCKWISE_EDGE(edge)][edge->color])
        // Check against the edge count for THIS edge's direction
        let face_colors = memo.faces.get_face(face_id).colors;
        let is_clockwise = face_colors.contains(crate::geometry::Color::new(color_idx as u8));
        let direction = if is_clockwise { 0 } else { 1 };
        let total_edges = state.edge_color_counts[direction][color_idx] as usize;

        eprintln!(
            "DEBUG: color={}, length={}, total_edges={} (direction={}), check={}",
            color_idx,
            length,
            total_edges,
            direction,
            length < total_edges && total_edges > 0
        );

        // Only fail if there's an actual mismatch (not when both are 0 during early setup)
        if length < total_edges && total_edges > 0 {
            // C: return failureDisconnectedCurve(depth);
            return Err(PropagationFailure::DisconnectedCurve {
                color: color_idx,
                edges_visited: length,
                total_edges,
                depth,
            });
        }

        // C: assert(length == EdgeColorCountState[IS_CLOCKWISE_EDGE(edge)][edge->color]);
        // If curve_length > edge_count, this indicates a problem with our edge tracking
        // or curve traversal. This shouldn't happen - treat it as disconnection.
        if length > total_edges {
            eprintln!(
                "WARNING: curve_length ({}) > edge_count ({}) for color {}",
                length, total_edges, color_idx
            );
            return Err(PropagationFailure::DisconnectedCurve {
                color: color_idx,
                edges_visited: length,
                total_edges,
                depth,
            });
        }

        // C: if (ColorCompletedState & 1u << edge->color) return NULL;
        // Check if already marked as complete
        if (state.colors_completed_this_call & (1 << color_idx)) != 0 {
            return Ok(());
        }

        // C: ColorCompletedState |= 1u << edge->color;
        state.colors_completed_this_call |= 1 << color_idx;

        // C: trailSetInt(&EdgeCurvesComplete[edge->color], 1);
        // Mark this color's curve as complete (trail-tracked)
        unsafe {
            trail.record_and_set(NonNull::from(&mut state.colors_checked[color_idx]), 1);
        }
    }

    // C: return NULL;
    Ok(())
}

/// Check edges in a cycle for disconnection.
///
/// Ported from C edge.c:76-83.
///
/// This is called during the main Venn diagram search for each edge
/// when a facial cycle is assigned to a face.
///
/// If the curve is already marked complete, skips the check.
/// Otherwise checks if the curve forms a single connected component.
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data
/// * `state` - Mutable search state
/// * `trail` - Trail for backtracking
/// * `face_id` - Face containing the edge
/// * `color_idx` - Color of the edge to check
/// * `depth` - Recursion depth for error messages
///
/// # Returns
///
/// `Ok(())` if curve is connected or incomplete,
/// `Err(PropagationFailure::DisconnectedCurve)` if disconnected.
#[allow(dead_code)]
pub fn edge_curve_checks(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    color_idx: usize,
    depth: usize,
) -> Result<(), PropagationFailure> {
    // C: if (EdgeCurvesComplete[edge->color]) return NULL;
    // Check if this color's curve is already marked complete
    if state.colors_checked[color_idx] != 0 {
        return Ok(());
    }

    // C: EDGE start = findStartOfCurve(edge);
    let (start_face, start_color) = find_start_of_curve(face_id, color_idx, memo, state);

    eprintln!(
        "DEBUG disconnection check: face={}, color={} -> start_face={}, start_color={}",
        face_id, color_idx, start_face, start_color
    );

    // C: return dynamicCheckForDisconnectedCurve(start, depth);
    check_for_disconnected_curve(start_face, start_color, depth, memo, state, trail)
}
