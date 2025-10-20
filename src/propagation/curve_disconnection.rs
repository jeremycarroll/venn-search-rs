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

use super::errors::PropagationFailure;

// TODO: Port from C edge.c:24-33
// /// Count the number of edges in a curve by following edge->to->next links.
// fn curve_length(
//     start_edge: EdgeRef,
//     state: &DynamicState,
// ) -> usize {
//     // C code:
//     // for (result = 1, current = edgeFollowForwards(edge); current != edge;
//     //      result++, current = edgeFollowForwards(current)) {
//     //   assert(current != NULL);
//     // }
//     // return result;
//     todo!("Port from C edge.c:24-33")
// }

// TODO: Port from C edge.c:35-52
// /// Check if a curve forms a single connected component.
// ///
// /// Returns error if curve is disconnected (forms multiple separate loops).
// fn check_for_disconnected_curve(
//     edge: EdgeRef,
//     depth: usize,
//     state: &mut DynamicState,
//     trail: &mut Trail,
// ) -> Result<(), PropagationFailure> {
//     // C code:
//     // if (edge->reversed->to != NULL) {
//     //   length = curveLength(edge);
//     //   if (length < EdgeColorCountState[IS_CLOCKWISE_EDGE(edge)][edge->color]) {
//     //     return failureDisconnectedCurve(depth);
//     //   }
//     //   assert(length == EdgeColorCountState[IS_CLOCKWISE_EDGE(edge)][edge->color]);
//     //   if (ColorCompletedState & 1u << edge->color) {
//     //     return NULL;
//     //   }
//     //   ColorCompletedState |= 1u << edge->color;
//     //   trailSetInt(&EdgeCurvesComplete[edge->color], 1);
//     // }
//     // return NULL;
//     todo!("Port from C edge.c:35-52")
// }

// TODO: Port from C edge.c:54-64
// /// Find the starting edge of a curve (or detect it's a loop).
// fn find_start_of_curve(
//     edge: EdgeRef,
//     state: &DynamicState,
// ) -> EdgeRef {
//     // C code:
//     // while ((next = edgeFollowBackwards(current)) != edge) {
//     //   if (next == NULL) {
//     //     return current;
//     //   }
//     //   current = next;
//     // }
//     // return edge;
//     todo!("Port from C edge.c:54-64")
// }

// TODO: Port from C edge.c:76-83
/// Check edges in a cycle for disconnection.
///
/// This is called during the main Venn diagram search for each edge
/// when a facial cycle is assigned to a face.
///
/// If the curve is already marked complete, skips the check.
/// Otherwise checks if the curve forms a single connected component.
pub fn edge_curve_checks(
    _memo: &MemoizedData,
    _state: &mut DynamicState,
    _trail: &mut Trail,
    _face_id: usize,
    _cycle_id: u64,
    _depth: usize,
) -> Result<(), PropagationFailure> {
    // C code:
    // if (EdgeCurvesComplete[edge->color]) {
    //   return NULL;
    // }
    // EDGE start = findStartOfCurve(edge);
    // return dynamicCheckForDisconnectedCurve(start, depth);

    // TODO: Implement
    // 1. Check if this color's curve is already complete
    // 2. If not, find start of curve
    // 3. Call check_for_disconnected_curve
    Ok(())
}
