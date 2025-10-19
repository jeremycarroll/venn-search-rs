// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Core propagation orchestration functions.
//!
//! This module contains the main entry points for constraint propagation:
//! - `propagate_cycle_choice` - Propagate a cycle assignment through the network
//! - `restrict_face_cycles` - Restrict a face's possible cycles and handle cascading
//! - `set_face_possible_cycles` - Helper to update face cycles with trail tracking

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::{CycleId, CycleSet};
use crate::trail::Trail;
use std::ptr::NonNull;

use super::adjacency::propagate_edge_adjacency;
use super::color_removal::remove_completed_color_from_search;
use super::errors::PropagationFailure;
use super::non_adjacency::{propagate_non_adjacent_faces, propagate_non_vertex_adjacent_faces};
use super::vertices::check_face_vertices;

/// Maximum propagation depth before we abort.
///
/// In practice, depth never exceeds NFACES (64 for NCOLORS=6),
/// but we set a higher limit to catch infinite recursion bugs.
const MAX_PROPAGATION_DEPTH: usize = 128;

/// Helper function to set a face's possible cycles with trail tracking.
///
/// Only trails words that actually change (optimization).
/// Also updates the cached cycle_count.
pub(super) fn set_face_possible_cycles(
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    new_cycles: CycleSet,
) {
    use crate::geometry::constants::CYCLESET_LENGTH;

    let face = &mut state.faces.faces[face_id];

    // Copy old words to avoid borrow checker issues
    let old_words = *face.possible_cycles.words();
    let new_words = *new_cycles.words();

    // Trail only modified words
    unsafe {
        let words_mut = face.possible_cycles.words_mut();
        for i in 0..CYCLESET_LENGTH {
            if old_words[i] != new_words[i] {
                // Record change on trail (NonNull::from provides null checking)
                trail.record_and_set(NonNull::from(&mut words_mut[i]), new_words[i]);
            }
        }
    }

    // Update cached cycle count (also trail-tracked)
    let new_count = new_cycles.len() as u64;
    if face.cycle_count != new_count {
        unsafe {
            trail.record_and_set(NonNull::from(&mut face.cycle_count), new_count);
        }
    }
}

/// Propagate a cycle choice for a face through the constraint network.
///
/// This is the main entry point called after assigning a cycle to a face.
/// It restricts the face to a singleton cycle set and propagates all constraints.
///
/// # Algorithm
///
/// 1. Set face's possible_cycles to singleton {cycle_id}
/// 2. Update crossing counts (triangle constraint)
/// 3. Propagate edge adjacency constraints
/// 4. Propagate non-adjacent face constraints
/// 5. Propagate non-vertex-adjacent face constraints
/// 6. Check for completed colors and remove them from search (optimization + disconnection check)
///
/// Each propagation step may trigger recursive propagation if faces reduce to singletons.
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data (cycles, faces, lookup tables)
/// * `state` - Mutable search state
/// * `trail` - Trail for backtracking
/// * `face_id` - Face that was assigned a cycle
/// * `cycle_id` - The cycle assigned to this face
/// * `depth` - Recursion depth (0 for initial assignment)
///
/// # Returns
///
/// `Ok(())` if propagation succeeds, `Err(PropagationFailure)` if constraints fail.
pub fn propagate_cycle_choice(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    cycle_id: CycleId,
    depth: usize,
) -> Result<(), PropagationFailure> {
    // Check depth limit
    if depth > MAX_PROPAGATION_DEPTH {
        return Err(PropagationFailure::DepthExceeded { depth });
    }

    // Reset temporary accumulator at top level
    if depth == 0 {
        state.colors_completed_this_call = 0;
    }

    // Set face's possible_cycles to singleton {cycle_id}
    let mut singleton = CycleSet::empty();
    singleton.insert(cycle_id);

    // Update face's possible cycles (trail-tracked)
    set_face_possible_cycles(state, trail, face_id, singleton);

    // Check and configure vertices for this cycle (sets edge->to pointers)
    // This also enforces corner detection (triangle constraint) by counting
    // crossings at vertices and checking the MAX_CROSSINGS_PER_PAIR limit.
    check_face_vertices(memo, state, trail, face_id, cycle_id, depth)?;

    // Set next/previous face pointers for dual graph cycles
    // These pointers link faces with the same number of colors into cycles
    if let Some(next_face) = memo.faces.next_face_by_cycle[face_id][cycle_id as usize] {
        let next_encoded = (next_face + 1) as u64;
        unsafe {
            trail.record_and_set(
                NonNull::from(&mut state.faces.faces[face_id].next_face_id_encoded),
                next_encoded,
            );
        }
    }
    if let Some(prev_face) = memo.faces.previous_face_by_cycle[face_id][cycle_id as usize] {
        let prev_encoded = (prev_face + 1) as u64;
        unsafe {
            trail.record_and_set(
                NonNull::from(&mut state.faces.faces[face_id].previous_face_id_encoded),
                prev_encoded,
            );
        }
    }

    // Propagate all constraint types
    propagate_edge_adjacency(memo, state, trail, face_id, cycle_id, depth)?;
    propagate_non_adjacent_faces(memo, state, trail, face_id, cycle_id, depth)?;
    propagate_non_vertex_adjacent_faces(memo, state, trail, face_id, cycle_id, depth)?;

    // TODO: Check for completed colors and remove them from search
    //
    // This optimization/disconnection check should be called after face choices during search,
    // but NOT during setup_central_face. Currently we can't distinguish between setup and search
    // at depth==0.
    //
    // The C code does this in dynamicFaceBacktrackableChoice (the engine-level choice function),
    // not in dynamicFaceChoice (our propagate_cycle_choice equivalent). We should implement this
    // when we add VennPredicate engine integration.
    //
    // For now, this means we find 7 solutions instead of 6 for test_55433 (one invalid with
    // disconnected curve).
    //
    // Uncomment this code when implementing VennPredicate:
    if depth == 0 && state.colors_completed_this_call != 0 {
        use crate::geometry::constants::NCOLORS;
        for color_idx in 0..NCOLORS {
            if (state.colors_completed_this_call & (1 << color_idx)) != 0 {
                remove_completed_color_from_search(memo, state, trail, color_idx)?;
            }
        }
    }

    Ok(())
}

/// Restrict a face's possible cycles and handle cascading propagation.
///
/// This is the workhorse function that:
/// 1. Checks if face is already assigned (validates constraint)
/// 2. Intersects current possible_cycles with allowed_cycles
/// 3. Detects failure (empty result)
/// 4. **KEY**: If result is singleton, auto-assigns and recursively propagates
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data
/// * `state` - Mutable search state
/// * `trail` - Trail for backtracking
/// * `face_id` - Face to restrict
/// * `allowed_cycles` - CycleSet of cycles that satisfy the constraint
/// * `depth` - Recursion depth
///
/// # Returns
///
/// `Ok(())` if restriction succeeds, `Err(PropagationFailure)` if constraints fail.
///
/// # Cascading Behavior
///
/// If the intersection results in exactly 1 cycle, this function:
/// 1. Assigns the forced cycle (trail-tracked)
/// 2. Calls `propagate_cycle_choice()` recursively to propagate the new assignment
///
/// This cascading is **critical** for search tractability - one assignment can
/// trigger a chain reaction that assigns many other faces automatically.
pub fn restrict_face_cycles(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    allowed_cycles: &CycleSet,
    depth: usize,
) -> Result<(), PropagationFailure> {
    // Check depth limit
    if depth > MAX_PROPAGATION_DEPTH {
        return Err(PropagationFailure::DepthExceeded { depth });
    }

    // 1. Check if face is already assigned
    let current_cycle = state.faces.faces[face_id].current_cycle();
    if let Some(assigned_cycle) = current_cycle {
        // Face already has a cycle - verify it's compatible
        if !allowed_cycles.contains(assigned_cycle) {
            return Err(PropagationFailure::ConflictingConstraints {
                face_id,
                assigned_cycle,
                depth,
            });
        }
        return Ok(());
    }

    // 2. Intersect current possible_cycles with allowed_cycles
    let old_cycles = state.faces.faces[face_id].possible_cycles;
    let new_cycles = old_cycles.intersection(allowed_cycles);

    // 3. Check for failure (empty result)
    if new_cycles.is_empty() {
        return Err(PropagationFailure::NoMatchingCycles { face_id, depth });
    }

    // 4. Update cycles (trail-tracked)
    if old_cycles != new_cycles {
        set_face_possible_cycles(state, trail, face_id, new_cycles);
    }

    // 5. KEY: If singleton, auto-assign and cascade
    if new_cycles.len() == 1 {
        let forced_cycle = new_cycles.iter().next().unwrap();

        // Check depth limit before recursive call
        if depth + 1 > MAX_PROPAGATION_DEPTH {
            return Err(PropagationFailure::DepthExceeded { depth: depth + 1 });
        }

        // Assign the forced cycle (trail-tracked)
        let encoded = forced_cycle + 1;
        unsafe {
            trail.record_and_set(
                NonNull::from(&mut state.faces.faces[face_id].current_cycle_encoded),
                encoded,
            );
        }

        // RECURSIVE PROPAGATION - this is the cascade effect!
        propagate_cycle_choice(memo, state, trail, face_id, forced_cycle, depth + 1)?;
    }

    Ok(())
}
