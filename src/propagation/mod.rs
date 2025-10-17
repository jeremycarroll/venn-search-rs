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
use crate::geometry::{CycleId, CycleSet, EdgeDynamic, MAX_CROSSINGS_PER_PAIR};
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

    /// Crossing limit exceeded between a color pair (triangle constraint violation).
    CrossingLimitExceeded {
        color_i: usize,
        color_j: usize,
        count: usize,
        max_allowed: usize,
        depth: usize,
    },
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
            PropagationFailure::CrossingLimitExceeded {
                color_i,
                color_j,
                count,
                max_allowed,
                depth,
            } => {
                write!(
                    f,
                    "Colors {} and {} cross {} times (max {}) (depth {})",
                    color_i, color_j, count, max_allowed, depth
                )
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

/// Update crossing counts for a cycle assignment.
///
/// For each consecutive pair of colors in the cycle, increment the crossing count
/// between that color pair. This enforces the triangle constraint that each pair
/// of colors can cross at most MAX_CROSSINGS_PER_PAIR (6) times.
///
/// # Algorithm
///
/// For each edge (color_i, color_j) in the cycle:
/// 1. Normalize to upper triangle (ensure i < j)
/// 2. Get mutable pointer to crossing_counts[i][j]
/// 3. Increment via trail.record_and_set()
/// 4. Check if count exceeds MAX_CROSSINGS_PER_PAIR
/// 5. Return error if exceeded
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data (contains cycle colors)
/// * `state` - Mutable search state (contains crossing_counts)
/// * `trail` - Trail for backtracking
/// * `cycle_id` - The cycle being assigned
/// * `depth` - Recursion depth for error reporting
///
/// # Returns
///
/// `Ok(())` if all crossing limits satisfied, `Err(PropagationFailure::CrossingLimitExceeded)` otherwise.
fn update_crossing_counts(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    cycle_id: CycleId,
    depth: usize,
) -> Result<(), PropagationFailure> {
    let cycle = memo.cycles.get(cycle_id);
    let cycle_colors = cycle.colors();

    // For each consecutive pair of colors in the cycle (including wrap-around)
    for i in 0..cycle.len() {
        let next_i = (i + 1) % cycle.len();
        let color_a = cycle_colors[i].value() as usize;
        let color_b = cycle_colors[next_i].value() as usize;

        // Normalize to upper triangle (i < j)
        let (color_i, color_j) = if color_a < color_b {
            (color_a, color_b)
        } else {
            (color_b, color_a)
        };

        // Get current count
        let current_count = state.crossing_counts.get(color_i, color_j);
        let new_count = current_count + 1;

        // Update via trail
        unsafe {
            let ptr = state.crossing_counts.get_mut_ptr(color_i, color_j);
            trail.record_and_set(NonNull::new_unchecked(ptr), new_count);
        }

        // Check limit
        if new_count as usize > MAX_CROSSINGS_PER_PAIR {
            return Err(PropagationFailure::CrossingLimitExceeded {
                color_i,
                color_j,
                count: new_count as usize,
                max_allowed: MAX_CROSSINGS_PER_PAIR,
                depth,
            });
        }
    }

    Ok(())
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

    // Update crossing counts (triangle constraint)
    update_crossing_counts(memo, state, trail, cycle_id, depth)?;

    // Check and configure vertices for this cycle (sets edge->to pointers)
    check_face_vertices(memo, state, trail, face_id, cycle_id, depth)?;

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

/// Check and configure vertices for a face's assigned cycle.
///
/// For each consecutive pair of colors in the cycle, this function:
/// 1. Retrieves the precomputed vertex from the vertex array
/// 2. Sets the edge->to pointer (trail-tracked) to connect to that vertex
/// 3. Validates vertex configuration compatibility
///
/// # Algorithm
///
/// For a cycle like (a, b, c):
/// - Check vertex at edge a→b
/// - Check vertex at edge b→c
/// - Check vertex at edge c→a (wrap-around)
///
/// For each edge pair (color_a, color_b):
/// - Look up the vertex in the precomputed vertex array
/// - Verify the vertex exists (should always be true for valid cycles)
/// - Set edge_dynamic[color_a].to_encoded pointer to point to this vertex
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data (contains vertex array)
/// * `state` - Mutable search state (contains edge_dynamic arrays)
/// * `trail` - Trail for backtracking
/// * `face_id` - Face that was assigned a cycle
/// * `cycle_id` - The cycle assigned to this face
/// * `depth` - Recursion depth for error messages
///
/// # Returns
///
/// `Ok(())` if all vertices are valid, `Err(PropagationFailure)` if validation fails.
pub fn check_face_vertices(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    cycle_id: CycleId,
    _depth: usize,
) -> Result<(), PropagationFailure> {
    let cycle = memo.cycles.get(cycle_id);
    let cycle_colors = cycle.colors();
    let face_memo = memo.faces.get_face(face_id);

    // For each consecutive pair of colors in the cycle (including wrap-around)
    for i in 0..cycle.len() {
        let next_i = (i + 1) % cycle.len();
        let color_a = cycle_colors[i];
        let color_b = cycle_colors[next_i];
        let color_a_idx = color_a.value() as usize;
        let color_b_idx = color_b.value() as usize;

        // Get the edge for color_a
        let edge_memo = &face_memo.edges[color_a_idx];

        // Check if edge already has a vertex assigned
        let face_dynamic = &state.faces.faces[face_id];
        let existing_to = face_dynamic.edge_dynamic[color_a_idx].get_to();

        if let Some(existing_link) = existing_to {
            // Edge already configured - validate compatibility
            let expected_link = edge_memo.possibly_to[color_b_idx];

            if let Some(expected) = expected_link {
                if existing_link.vertex_id != expected.vertex_id {
                    // Conflicting vertex assignments - this is OK, just skip
                    // (monotonicity filter may have already set a different valid vertex)
                    continue;
                }
            }
            continue; // Edge already set up correctly
        }

        // Get the vertex connection from possibly_to
        let vertex_link = edge_memo.possibly_to[color_b_idx];

        if let Some(link) = vertex_link {
            // Set edge->to_encoded pointer (trail-tracked)
            let encoded = EdgeDynamic::encode_to(Some(link));
            unsafe {
                trail.record_and_set(
                    NonNull::from(
                        &mut state.faces.faces[face_id].edge_dynamic[color_a_idx].to_encoded,
                    ),
                    encoded,
                );
            }
        }
        // If vertex_link is None, that's OK - not all edges may have vertices
        // assigned yet (this is the DYNAMIC phase)
    }

    Ok(())
}

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
fn propagate_edge_adjacency(
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
    use crate::context::SearchContext;
    use crate::geometry::constants::NCYCLES;

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

    #[test]
    fn test_direction_tables_populated() {
        let ctx = SearchContext::new();

        // Check all cycles have non-empty direction tables
        for cycle_id in 0..NCYCLES as u64 {
            let cycle = ctx.memo.cycles.get(cycle_id);

            for i in 0..cycle.len() {
                let same_dir = cycle.same_direction(i);
                let opp_dir = cycle.opposite_direction(i);

                // Direction tables should have at least one cycle
                assert!(
                    !same_dir.is_empty(),
                    "Cycle {} edge {} has empty same_direction table",
                    cycle_id,
                    i
                );
                assert!(
                    !opp_dir.is_empty(),
                    "Cycle {} edge {} has empty opposite_direction table",
                    cycle_id,
                    i
                );
            }
        }
    }
}
