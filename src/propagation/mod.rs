// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Constraint propagation for Venn diagram search.
//!
//! This module implements the cascading constraint propagation algorithm that prunes
//! the search space from ~10^150 configurations to a tractable size.
//!
//! # Algorithm Overview
//!
//! When a face is assigned a cycle:
//! 1. Eliminate incompatible cycles from related faces
//! 2. If any face reduces to **exactly 1 possible cycle**, automatically assign it
//! 3. Recursively propagate that new assignment (CASCADE)
//! 4. Fail immediately if any face has **zero possible cycles**
//!
//! This cascading effect (step 2-3) is what makes the search tractable.
//!
//! # Constraint Types
//!
//! **Edge Adjacency** (uses `cycle_pairs`, `cycle_triples`):
//! - Faces sharing an edge must have compatible cycles
//! - Example: If face uses cycle with edge a→b, then face across that edge
//!   must also have a cycle containing edge a→b
//!
//! **Non-Adjacent Faces** (uses `cycles_omitting_one_color`):
//! - Faces that don't share a color must use cycles omitting that color
//! - Example: If face uses cycle with colors {a,b,c}, then the face adjacent
//!   only through color d must use a cycle omitting d
//!
//! **Non-Vertex-Adjacent Faces** (uses `cycles_omitting_color_pair`):
//! - Faces that don't share a vertex must use cycles omitting certain edges
//! - Example: If cycle doesn't contain edge i→j, then doubly-adjacent face
//!   must use a cycle omitting edge i→j
//!
//! # Depth Tracking
//!
//! The `depth` parameter tracks recursion depth for:
//! - Debugging (failure messages show where constraint originated)
//! - Stack overflow prevention (depth ≤ NFACES = 64)
//! - Statistics (how deep cascades go)

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::{CycleId, CycleSet};
use crate::trail::Trail;
use std::fmt;
use std::ptr::NonNull;

/// Maximum propagation depth before we abort.
///
/// In practice, depth never exceeds NFACES (64 for NCOLORS=6),
/// but we set a higher limit to catch infinite recursion bugs.
const MAX_PROPAGATION_DEPTH: usize = 128;

/// Errors that can occur during constraint propagation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropagationFailure {
    /// Face has no remaining possible cycles after constraint propagation.
    NoMatchingCycles { face_id: usize, depth: usize },

    /// Face is already assigned a cycle that conflicts with new constraints.
    ConflictingConstraints {
        face_id: usize,
        assigned_cycle: CycleId,
        depth: usize,
    },

    /// Propagation depth exceeded (likely infinite recursion bug).
    DepthExceeded { depth: usize },
}

impl fmt::Display for PropagationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropagationFailure::NoMatchingCycles { face_id, depth } => {
                write!(
                    f,
                    "Face {} has no matching cycles (depth {})",
                    face_id, depth
                )
            }
            PropagationFailure::ConflictingConstraints {
                face_id,
                assigned_cycle,
                depth,
            } => {
                write!(
                    f,
                    "Face {} assigned cycle {} conflicts with constraints (depth {})",
                    face_id, assigned_cycle, depth
                )
            }
            PropagationFailure::DepthExceeded { depth } => {
                write!(f, "Propagation depth {} exceeded max", depth)
            }
        }
    }
}

/// Helper function to set a face's possible cycles with trail tracking.
///
/// Only trails words that actually change (optimization).
/// Also updates the cached cycle_count.
fn set_face_possible_cycles(
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
                // Get pointer to word i in the mutable words array
                let ptr = &mut words_mut[i] as *mut u64;
                let ptr = NonNull::new_unchecked(ptr);
                trail.record_and_set(ptr, new_words[i]);
            }
        }
    }

    // Update cached cycle count (also trail-tracked)
    let new_count = new_cycles.len() as u64;
    if face.cycle_count != new_count {
        unsafe {
            let ptr = NonNull::new_unchecked(&mut face.cycle_count);
            trail.record_and_set(ptr, new_count);
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
/// 2. Propagate edge adjacency constraints
/// 3. Propagate non-adjacent face constraints
/// 4. Propagate non-vertex-adjacent face constraints
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

    // Set face's possible_cycles to singleton {cycle_id}
    let mut singleton = CycleSet::empty();
    singleton.insert(cycle_id);

    // Update face's possible cycles (trail-tracked)
    set_face_possible_cycles(state, trail, face_id, singleton);

    // Propagate all constraint types
    propagate_edge_adjacency(memo, state, trail, face_id, cycle_id, depth)?;
    propagate_non_adjacent_faces(memo, state, trail, face_id, cycle_id, depth)?;
    propagate_non_vertex_adjacent_faces(memo, state, trail, face_id, cycle_id, depth)?;

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

        // Assign the forced cycle (trail-tracked)
        let encoded = forced_cycle + 1;
        unsafe {
            let ptr = NonNull::new_unchecked(&mut state.faces.faces[face_id].current_cycle_encoded);
            trail.record_and_set(ptr, encoded);
        }

        // RECURSIVE PROPAGATION - this is the cascade effect!
        propagate_cycle_choice(memo, state, trail, face_id, forced_cycle, depth + 1)?;
    }

    Ok(())
}

/// Propagate edge adjacency constraints.
///
/// For each edge in the assigned cycle, propagate constraints to adjacent faces
/// based on vertex and edge configuration.
///
/// **TODO (PR #11)**: This requires vertex and edge tracking which is not yet
/// implemented. This function will:
/// 1. For each edge in the cycle, get the vertex it connects to
/// 2. Determine which faces are adjacent through that vertex
/// 3. Use `cycle_pairs` lookup from MEMO
/// 4. Restrict adjacent faces to compatible cycles
///
/// For now, this is a no-op stub. Edge adjacency is REQUIRED for correct
/// constraint propagation and will be implemented in PR #11.
fn propagate_edge_adjacency(
    _memo: &MemoizedData,
    _state: &mut DynamicState,
    _trail: &mut Trail,
    _face_id: usize,
    _cycle_id: CycleId,
    _depth: usize,
) -> Result<(), PropagationFailure> {
    // TODO (PR #11): Implement edge/vertex adjacency propagation
    // This requires vertex configuration tracking and edge->to pointers
    // which are set up during vertex checking phase.
    Ok(())
}

/// Propagate constraints to non-adjacent faces.
///
/// For each color NOT in the cycle, restrict faces adjacent only through
/// that color to use cycles omitting that color.
///
/// Uses `cycles_omitting_one_color` from CyclesMemo.
fn propagate_non_adjacent_faces(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    cycle_id: CycleId,
    depth: usize,
) -> Result<(), PropagationFailure> {
    use crate::geometry::constants::NCOLORS;
    use crate::geometry::Color;

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
        let omitting_words = memo.cycles_memo.cycles_omitting_one_color[color_idx];
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
fn propagate_non_vertex_adjacent_faces(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propagation_failure_display() {
        let fail1 = PropagationFailure::NoMatchingCycles {
            face_id: 5,
            depth: 2,
        };
        assert!(format!("{}", fail1).contains("Face 5"));
        assert!(format!("{}", fail1).contains("depth 2"));

        let fail2 = PropagationFailure::ConflictingConstraints {
            face_id: 10,
            assigned_cycle: 42,
            depth: 3,
        };
        assert!(format!("{}", fail2).contains("Face 10"));
        assert!(format!("{}", fail2).contains("cycle 42"));

        let fail3 = PropagationFailure::DepthExceeded { depth: 150 };
        assert!(format!("{}", fail3).contains("150"));
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_max_depth_constant() {
        // Verify MAX_PROPAGATION_DEPTH is reasonable
        assert!(MAX_PROPAGATION_DEPTH >= 64); // At least NFACES for NCOLORS=6
        assert!(MAX_PROPAGATION_DEPTH <= 256); // Not too large
    }
}
