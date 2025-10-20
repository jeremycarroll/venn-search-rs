// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Completed color removal optimization.
//!
//! When a color forms a complete closed loop, we can optimize the search by
//! restricting all unassigned faces to cycles omitting that color. This also
//! serves as a disconnection check: if any face needs the completed color,
//! then the curve must be disconnected.

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::CycleSet;
use crate::trail::Trail;

use super::core::restrict_face_cycles;
use super::errors::PropagationFailure;

/// Remove a completed color from further search consideration.
///
/// When a color forms a complete closed loop, we can optimize the search by
/// restricting all unassigned faces to cycles omitting that color.
///
/// This also serves as a disconnection check: if any unassigned face NEEDS
/// the completed color (can't be satisfied without it), then the curve must
/// be disconnected (some edges forming a separate component).
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data
/// * `state` - Mutable search state
/// * `trail` - Trail for backtracking
/// * `color_idx` - Index of the completed color to remove
///
/// # Returns
///
/// `Ok(())` if removal succeeds, `Err(PropagationFailure::DisconnectedCurve)` if any face
/// needs this color (indicating disconnection).
pub(super) fn remove_completed_color_from_search(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    color_idx: usize,
) -> Result<(), PropagationFailure> {
    use crate::geometry::constants::NFACES;

    // Get cycles omitting this color
    let omitting_words = memo.cycles_memo.get_cycles_omitting_one_color(color_idx);
    let omitting_cycleset = CycleSet::from_words(omitting_words);

    // For each unassigned face
    for face_id in 0..NFACES {
        let face = &state.faces.faces[face_id];

        // Skip faces that already have a cycle assigned
        if face.current_cycle().is_some() {
            continue;
        }

        // Check if this face has used this color (edge->to is set)
        let edge_to = face.edge_dynamic[color_idx].get_to();
        if edge_to.is_some() {
            // Face already uses this color, can't restrict it
            continue;
        }

        // Restrict this face to cycles omitting the completed color
        // If this fails, the face needs this color → disconnected curve
        restrict_face_cycles(memo, state, trail, face_id, &omitting_cycleset, 0).map_err(|_| {
            PropagationFailure::DisconnectedCurve {
                color: color_idx,
                edges_visited: 0, // Not applicable here
                total_edges: 0,   // Not applicable here
                depth: 0,
            }
        })?;
    }

    Ok(())
}
