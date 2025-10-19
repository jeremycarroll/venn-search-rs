// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Non-adjacency constraint propagation.
//!
//! This module implements propagation of constraints between faces that
//! don't share edges or vertices:
//! - Faces not sharing a color must use cycles omitting that color
//! - Faces not sharing a vertex must use cycles omitting certain edges

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::{Color, CycleId, CycleSet};
use crate::trail::Trail;

use super::core::restrict_face_cycles;
use super::errors::PropagationFailure;

/// Propagate constraints to non-adjacent faces.
///
/// For each color NOT in the cycle, restrict faces adjacent only through
/// that color to use cycles omitting that color.
///
/// Uses `cycles_omitting_one_color` from CyclesMemo.
pub(super) fn propagate_non_adjacent_faces(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    cycle_id: CycleId,
    depth: usize,
) -> Result<(), PropagationFailure> {
    use crate::geometry::constants::NCOLORS;

    let cycle = memo.cycles.get(cycle_id);
    let cycle_colorset = cycle.colorset();
    let face = memo.faces.get_face(face_id);
    let face_colors = face.colors;

    // For each color not in the cycle
    for color_idx in 0..NCOLORS {
        let color = Color::new(color_idx as u8);

        if cycle_colorset.contains(color) {
            continue; // Skip colors that are in the cycle
        }

        // Face adjacent only through this color
        // Adjacent face = current face XOR (1 << color)
        let adjacent_face_id = face_colors.bits() as usize ^ (1 << color_idx);

        // Get cycles omitting this color
        let omitting_words = memo.cycles_memo.get_cycles_omitting_one_color(color_idx);
        let omitting_cycleset = CycleSet::from_words(omitting_words);

        // Restrict adjacent face to these cycles
        restrict_face_cycles(
            memo,
            state,
            trail,
            adjacent_face_id,
            &omitting_cycleset,
            depth,
        )?;
    }

    Ok(())
}

/// Propagate constraints to non-vertex-adjacent faces.
///
/// For each pair of colors (i, j) where i < j, if the cycle doesn't contain
/// the directed edge i→j, then faces adjacent through both i and j must use
/// cycles omitting edge i→j.
///
/// Uses `cycles_omitting_color_pair` (upper triangle only) from CyclesMemo.
pub(super) fn propagate_non_vertex_adjacent_faces(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    cycle_id: CycleId,
    depth: usize,
) -> Result<(), PropagationFailure> {
    use crate::geometry::constants::NCOLORS;

    // Read cycle data from immutable memo
    let cycle = memo.cycles.get(cycle_id);
    let cycle_len = cycle.len();
    let cycle_colors = cycle.colors(); // &[Color] - no allocation needed!

    let face = memo.faces.get_face(face_id);
    let face_colors = face.colors;

    // Upper triangle only (i < j)
    for i in 0..NCOLORS {
        for j in (i + 1)..NCOLORS {
            // Check if cycle contains the directed edge i→j
            let mut has_edge_i_to_j = false;
            for edge_idx in 0..cycle_len {
                let next_idx = (edge_idx + 1) % cycle_len;
                if cycle_colors[edge_idx].value() == i as u8
                    && cycle_colors[next_idx].value() == j as u8
                {
                    has_edge_i_to_j = true;
                    break;
                }
            }

            // If cycle has this edge, skip (no restriction needed)
            if has_edge_i_to_j {
                continue;
            }

            // Face adjacent through both colors i and j
            // Adjacent face = current face XOR ((1 << i) | (1 << j))
            let xor_mask = (1 << i) | (1 << j);
            let adjacent_face_id = face_colors.bits() as usize ^ xor_mask;

            // Get cycles omitting edge i→j (upper triangle)
            let omitting_words = *memo.cycles_memo.get_cycles_omitting_color_pair(i, j);
            let omitting_cycleset = CycleSet::from_words(omitting_words);

            // Restrict adjacent face to these cycles
            restrict_face_cycles(
                memo,
                state,
                trail,
                adjacent_face_id,
                &omitting_cycleset,
                depth,
            )?;
        }
    }

    Ok(())
}
