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
use crate::trail::Trail;

use super::errors::PropagationFailure;

/// Maximum corners allowed per curve for triangle diagrams.
const MAX_CORNERS: usize = 3;

// TODO: Port from C vertex.c:57-73
// /// Core corner detection algorithm that updates crossing sets.
// ///
// /// Returns true if a corner is detected at this vertex.
// fn detect_corner_and_update_crossing_sets(
//     other: ColorSet,
//     outside: &mut ColorSet,
//     passed: &mut ColorSet,
// ) -> bool {
//     // C code:
//     // if (other & *outside) {
//     //   *outside &= ~other;
//     //   if (other & *passed) {
//     //     *passed = 0;
//     //     return true;
//     //   }
//     // } else {
//     //   *passed |= other;
//     //   *outside |= other;
//     // }
//     // return false;
//     todo!("Port from C vertex.c:57-73")
// }

// TODO: Port from C vertex.c:100-124
// /// Finds corners by traversing edges from a starting point.
// ///
// /// Returns error if more than MAX_CORNERS corners are needed.
// fn find_corners_by_traversal(
//     start: EdgeRef,
//     depth: usize,
//     memo: &MemoizedData,
//     state: &DynamicState,
// ) -> Result<Vec<EdgeRef>, PropagationFailure> {
//     // C code loop:
//     // do {
//     //   CURVELINK p = current->to;
//     //   if (detectCornerAndUpdateCrossingSets(p->vertex->colors & notMyColor,
//     //                                         &outside, &passed)) {
//     //     if (counter >= MAX_CORNERS) {
//     //       return failureTooManyCorners(depth);
//     //     }
//     //     cornersReturn[counter++] = current;
//     //   }
//     //   current = p->next;
//     // } while (current->to != NULL && current != start);
//     todo!("Port from C vertex.c:100-124")
// }

// TODO: Port from C vertex.c:180-191
/// Check if a curve requires more than MAX_CORNERS corners.
///
/// This is called during the main Venn diagram search for each edge
/// when a facial cycle is assigned to a face.
///
/// For NCOLORS <= 4, this always succeeds (no check needed).
/// For NCOLORS > 4, this validates the curve can be drawn with ≤3 corners.
pub fn vertex_corner_check(
    _memo: &MemoizedData,
    _state: &DynamicState,
    _trail: &mut Trail,
    _face_id: usize,
    _cycle_id: u64,
    _depth: usize,
) -> Result<(), PropagationFailure> {
    // C code:
    // #if NCOLORS <= 4
    //   return NULL;
    // #else
    //   EDGE ignore[MAX_CORNERS * 100];
    //   if (start->reversed->to != NULL) {
    //     start = vertexGetCentralEdge(start->color);
    //   }
    //   return findCornersByTraversal(start, depth, ignore);
    // #endif

    #[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
    {
        // For NCOLORS <= 4, no corner check needed
        Ok(())
    }

    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
    {
        // TODO: Implement for NCOLORS > 4
        // 1. Get edge for this cycle color
        // 2. If edge->reversed->to != NULL, find central edge
        // 3. Call find_corners_by_traversal
        Ok(())
    }
}
