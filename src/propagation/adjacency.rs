// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Edge adjacency constraint propagation.
//!
//! This module implements propagation of constraints between faces that
//! share an edge. Uses the direction tables (same_direction, opposite_direction)
//! from cycle data to restrict adjacent faces.

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::CycleId;
use crate::trail::Trail;

use super::core::restrict_face_cycles;
use super::errors::PropagationFailure;

/// Propagate edge adjacency constraints.
///
/// For each edge in the assigned cycle, propagate constraints to adjacent faces
/// based on vertex and edge configuration.
///
/// # Algorithm
///
/// For each edge in the cycle:
/// 1. Get vertex from edge->to pointer
/// 2. Determine aColor (edge color) and bColor (other color at vertex)
/// 3. Find aFace (adjacent through aColor) and abFace (adjacent through aColor AND bColor)
/// 4. Propagate same_direction to abFace (doubly-adjacent)
/// 5. Propagate opposite_direction to aFace (singly-adjacent)
///
/// This uses the direction tables (same_direction, opposite_direction) computed during
/// cycle initialization.
pub(super) fn propagate_edge_adjacency(
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
    let face_colors = face_memo.colors;

    // For each edge in the cycle
    for i in 0..cycle.len() {
        let a_color = cycle_colors[i];
        let a_color_idx = a_color.value() as usize;

        // bColor is simply the next color in the cycle (vertex connects aColor->bColor)
        let next_i = (i + 1) % cycle.len();
        let b_color = cycle_colors[next_i];
        let b_color_idx = b_color.value() as usize;

        // Determine adjacent faces:
        // - aFace: adjacent through aColor only (XOR with a_color bit)
        // - abFace: adjacent through BOTH aColor and bColor (XOR with both bits)
        let a_face_id = face_colors.bits() as usize ^ (1 << a_color_idx);
        let ab_face_id = face_colors.bits() as usize ^ (1 << a_color_idx) ^ (1 << b_color_idx);

        // Get direction cycle sets from the cycle
        let same_dir_cycles = cycle.same_direction(i);
        let opposite_dir_cycles = cycle.opposite_direction(i);

        // Propagate to doubly-adjacent face (same direction)
        restrict_face_cycles(memo, state, trail, ab_face_id, same_dir_cycles, depth)?;

        // Propagate to singly-adjacent face (opposite direction)
        restrict_face_cycles(memo, state, trail, a_face_id, opposite_dir_cycles, depth)?;
    }

    Ok(())
}
