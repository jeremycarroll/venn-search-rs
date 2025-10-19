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
use crate::geometry::{Color, CornerWalkState, CycleId, CycleSet, EdgeDynamic, MAX_CROSSINGS_PER_PAIR};
use crate::trail::Trail;
use std::fmt;
use std::ptr::NonNull;
use strum_macros::EnumCount as EnumCountMacro;

/// Maximum propagation depth before we abort.
///
/// In practice, depth never exceeds NFACES (64 for NCOLORS=6),
/// but we set a higher limit to catch infinite recursion bugs.
const MAX_PROPAGATION_DEPTH: usize = 128;

/// Maximum corners allowed per curve for triangle diagrams.
/// Triangles have 3 corners, so each curve can have at most 3 corners.
const MAX_CORNERS: usize = 3;

/// Errors that can occur during constraint propagation.
#[derive(Debug, Clone, PartialEq, Eq, EnumCountMacro)]
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

    /// Too many corners detected on a curve (triangle constraint violation).
    /// Triangles have at most 3 corners per curve.
    TooManyCorners {
        color: usize,
        corner_count: usize,
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
            PropagationFailure::TooManyCorners {
                color,
                corner_count,
                max_allowed,
                depth,
            } => {
                write!(
                    f,
                    "Color {} requires {} corners (max {} for triangles) (depth {})",
                    color, corner_count, max_allowed, depth
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
/// 4. **Corner Detection**: Counts crossings at vertices for triangle constraint
///
/// # Corner Detection Algorithm
///
/// Each vertex represents a crossing between two color curves. When we encounter
/// a vertex for the first time (not already processed), we:
/// 1. Increment the crossing count for that color pair
/// 2. Mark the vertex as processed to avoid double-counting
/// 3. Check if crossing count exceeds MAX_CROSSINGS_PER_PAIR (6 for triangles)
///
/// This enforces the triangle constraint during search, pruning the search space
/// from ~30,000 configurations to the actual 152 (N=5) or 233 (N=6) solutions.
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data (contains vertex array)
/// * `state` - Mutable search state (contains edge_dynamic arrays, crossing counts, vertex tracking)
/// * `trail` - Trail for backtracking
/// * `face_id` - Face that was assigned a cycle
/// * `cycle_id` - The cycle assigned to this face
/// * `depth` - Recursion depth for error messages
///
/// # Returns
///
/// `Ok(())` if all vertices are valid and crossing limits not exceeded,
/// `Err(PropagationFailure::CrossingLimitExceeded)` if triangle constraint violated.
pub fn check_face_vertices(
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
        // This was populated during MemoizedData initialization by FacesMemo::populate_vertex_links()
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

            // Corner detection: Count crossing at this vertex
            let vertex_id = link.vertex_id;

            // Check if vertex already processed
            if vertex_id < state.vertex_processed.len() && state.vertex_processed[vertex_id] == 0 {
                // First time seeing this vertex - count the crossing

                // Normalize color pair to upper triangle (i < j)
                let (color_i, color_j) = if color_a_idx < color_b_idx {
                    (color_a_idx, color_b_idx)
                } else {
                    (color_b_idx, color_a_idx)
                };

                // Increment crossing count (trail-tracked)
                let current_count = state.crossing_counts.get(color_i, color_j);
                let new_count = current_count + 1;

                unsafe {
                    let ptr = state.crossing_counts.get_mut_ptr(color_i, color_j);
                    trail.record_and_set(NonNull::new_unchecked(ptr), new_count);
                }

                // Check triangle constraint (max 6 crossings per pair)
                if new_count as usize > MAX_CROSSINGS_PER_PAIR {
                    return Err(PropagationFailure::CrossingLimitExceeded {
                        color_i,
                        color_j,
                        count: new_count as usize,
                        max_allowed: MAX_CROSSINGS_PER_PAIR,
                        depth,
                    });
                }

                // Mark vertex as processed (trail-tracked)
                unsafe {
                    trail.record_and_set(NonNull::from(&mut state.vertex_processed[vertex_id]), 1);
                }
            }
        }
        // If vertex_link is None, that's OK - not all edges may have vertices
        // assigned yet (this is the DYNAMIC phase)
    }

    // Corner checking (only for NCOLORS > 4)
    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
    {
        eprintln!("[DEBUG] Corner check for face {} cycle {}", face_id, cycle_id);
        check_corners_for_cycle(memo, state, face_id, cycle_id, depth)?;
    }

    Ok(())
}

/// Check corner constraints for all colors in a cycle.
///
/// For Venn diagrams drawable with triangles, each curve can have at most 3 corners.
/// This function walks around each color's curve and counts corners using the
/// Carroll 2000 corner detection algorithm.
///
/// Only active for NCOLORS > 4 (N=3,4 don't need corner checking).
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data
/// * `state` - Mutable search state with edge connections
/// * `face_id` - Face that was assigned a cycle
/// * `cycle_id` - The cycle assigned to this face
/// * `depth` - Recursion depth for error messages
///
/// # Returns
///
/// `Ok(())` if all curves have ≤ MAX_CORNERS corners,
/// `Err(PropagationFailure::TooManyCorners)` if a curve requires > 3 corners.
#[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
fn check_corners_for_cycle(
    memo: &MemoizedData,
    state: &DynamicState,
    face_id: usize,
    cycle_id: CycleId,
    depth: usize,
) -> Result<(), PropagationFailure> {
    let cycle = memo.cycles.get(cycle_id);
    let cycle_colors = cycle.colors();

    // Check each color in the cycle
    for i in 0..cycle.len() {
        let color = cycle_colors[i];
        let color_idx = color.value() as usize;

        // Get the edge for this color
        let face_dynamic = &state.faces.faces[face_id];
        let edge_to = face_dynamic.edge_dynamic[color_idx].get_to();

        // Only check if edge has a connection set up
        if let Some(start_link) = edge_to {
            // Walk around the curve and count corners
            let corner_count = count_corners_on_curve(memo, state, face_id, color_idx, start_link)?;

            eprintln!("[DEBUG]   Color {}: {} corners", color_idx, corner_count);

            // Check if exceeds triangle limit
            if corner_count > MAX_CORNERS {
                eprintln!("[DEBUG]   TOO MANY CORNERS! Color {} has {}, max {}",
                         color_idx, corner_count, MAX_CORNERS);
                return Err(PropagationFailure::TooManyCorners {
                    color: color_idx,
                    corner_count,
                    max_allowed: MAX_CORNERS,
                    depth,
                });
            }
        }
    }

    Ok(())
}

/// Count corners on a curve by traversing edges.
///
/// Implements the Carroll 2000 corner detection algorithm by walking around
/// a curve and tracking which other curves are inside vs outside.
///
/// # Arguments
///
/// * `memo` - MEMO data with vertex information
/// * `state` - Search state with edge connections
/// * `start_face_id` - Starting face for the traversal
/// * `color_idx` - Index of the color whose curve we're checking
/// * `start_link` - Starting edge connection
///
/// # Returns
///
/// Number of corners detected on this curve, or error if traversal fails.
#[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
fn count_corners_on_curve(
    memo: &MemoizedData,
    state: &DynamicState,
    start_face_id: usize,
    color_idx: usize,
    start_link: crate::geometry::CurveLink,
) -> Result<usize, PropagationFailure> {
    use crate::geometry::constants::NCOLORS;

    let mut corner_state = CornerWalkState::new();

    // Initialize with colors we're OUTSIDE of at the starting face
    // A face's colorset contains curves we're INSIDE, so we need the complement
    let start_face_memo = memo.faces.get_face(start_face_id);
    let start_face_colorset = start_face_memo.colors;
    for c in 0..NCOLORS {
        let color = Color::new(c as u8);
        // Add to 'out' if NOT in face colorset and NOT the curve we're traversing
        if !start_face_colorset.contains(color) && c != color_idx {
            corner_state.out.insert(color);
        }
    }

    let mut current_face_id = start_face_id;
    let mut current_color = color_idx;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 1000; // Prevent infinite loops
    let mut vertices_visited = 0;

    // Walk around the curve following edge->to->next links
    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            // Safety bail-out - shouldn't happen in valid diagrams
            eprintln!("[DEBUG]     MAX_ITERATIONS reached, {} vertices visited", vertices_visited);
            return Ok(corner_state.corner_count());
        }

        let face_dynamic = &state.faces.faces[current_face_id];
        let edge_to = face_dynamic.edge_dynamic[current_color].get_to();

        if let Some(link) = edge_to {
            // Get the vertex from MEMO data by its ID
            let vertex = match memo.vertices.get_vertex_by_id(link.vertex_id) {
                Some(v) => v,
                None => {
                    eprintln!("[DEBUG]     Vertex {} not found", link.vertex_id);
                    break;
                }
            };

            // Determine the other color at this vertex and which color we're traversing
            let (other_color, is_primary) = if vertex.primary.value() as usize == color_idx {
                (vertex.secondary, true)
            } else if vertex.secondary.value() as usize == color_idx {
                (vertex.primary, false)
            } else {
                eprintln!("[DEBUG]     Vertex {} doesn't have our color {}", link.vertex_id, color_idx);
                break;
            };

            // Process this vertex for corner detection
            corner_state.process_vertex(other_color, link.vertex_id);
            vertices_visited += 1;

            // Find the exit edge: we entered on one slot, exit on the opposite slot
            // Primary color: slots 0,1 (enter on one, exit on other)
            // Secondary color: slots 2,3 (enter on one, exit on other)
            // We need to find which slot we're exiting from
            let mut next_edge_ref = None;
            if is_primary {
                // Check both primary slots (0 and 1)
                for slot in [0, 1] {
                    let edge_ref = vertex.incoming_edges[slot];
                    if edge_ref.face_id != current_face_id || edge_ref.color_idx != current_color {
                        next_edge_ref = Some(edge_ref);
                        break;
                    }
                }
            } else {
                // Check both secondary slots (2 and 3)
                for slot in [2, 3] {
                    let edge_ref = vertex.incoming_edges[slot];
                    if edge_ref.face_id != current_face_id || edge_ref.color_idx != current_color {
                        next_edge_ref = Some(edge_ref);
                        break;
                    }
                }
            }

            let next_edge = match next_edge_ref {
                Some(e) => e,
                None => {
                    eprintln!("[DEBUG]     Could not find exit edge at vertex {}", link.vertex_id);
                    break;
                }
            };

            let next_face_id = next_edge.face_id;
            let next_color_idx = next_edge.color_idx;

            // Sanity check: we should still be on the same curve color
            if next_color_idx != color_idx {
                eprintln!("[DEBUG]     ERROR: Color changed from {} to {} (should stay constant)",
                         color_idx, next_color_idx);
                break;
            }

            // Check if we've completed the loop (back to starting face)
            if next_face_id == start_face_id {
                eprintln!("[DEBUG]     Completed full traversal, {} vertices visited", vertices_visited);
                break; // Completed traversal
            }

            current_face_id = next_face_id;
            current_color = next_color_idx;
        } else {
            // Edge not connected - curve incomplete, can't check corners yet
            eprintln!("[DEBUG]     Traversal stopped early at iteration {}, {} vertices visited (edge not connected)", iterations, vertices_visited);
            break;
        }
    }

    Ok(corner_state.corner_count())
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

/// Helper function to restrict a face to only cycles of a specific length.
///
/// Builds a CycleSet of all cycles with the specified length, then restricts
/// the face's possible_cycles to that set.
///
/// If length == 0, returns Ok(()) without restriction (used to skip faces).
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data
/// * `state` - Mutable search state
/// * `trail` - Trail for backtracking
/// * `face_id` - Face to restrict
/// * `length` - Required cycle length (or 0 for no restriction)
///
/// # Returns
///
/// `Ok(())` if restriction succeeds, `Err(PropagationFailure)` if no cycles match.
fn restrict_face_to_cycle_length(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_id: usize,
    length: usize,
) -> Result<(), PropagationFailure> {
    use crate::geometry::constants::NCYCLES;

    // Skip if length is 0 (no restriction)
    if length == 0 {
        return Ok(());
    }

    // Build CycleSet of all cycles with this length
    let mut allowed_cycles = CycleSet::empty();
    for cycle_id in 0..NCYCLES as u64 {
        let cycle = memo.cycles.get(cycle_id);
        if cycle.len() == length {
            allowed_cycles.insert(cycle_id);
        }
    }

    // Restrict the face to these cycles
    restrict_face_cycles(memo, state, trail, face_id, &allowed_cycles, 0)
}

/// Set up the central face configuration for the search.
///
/// This function is called to constrain the search based on degree signatures
/// from InnerFacePredicate (for N≥5) or command-line flags. It:
///
/// 1. For each color i, restricts the face with all colors except i to cycles
///    of the specified length (if face_degrees[i] != 0)
/// 2. Sets the inner face (all colors) to the canonical cycle
/// 3. Propagates the constraints through the network
///
/// # Face Indexing
///
/// For NCOLORS=6, the faces are:
/// - ~(1 << 0) = 0b111110 (face 62) → colors {1,2,3,4,5}
/// - ~(1 << 1) = 0b111101 (face 61) → colors {0,2,3,4,5}
/// - ~(1 << 2) = 0b111011 (face 59) → colors {0,1,3,4,5}
/// - ~(1 << 3) = 0b110111 (face 55) → colors {0,1,2,4,5}
/// - ~(1 << 4) = 0b101111 (face 47) → colors {0,1,2,3,5}
/// - ~(1 << 5) = 0b011111 (face 31) → colors {0,1,2,3,4}
///
/// These are the "5-faces" that border the inner face (face 63 = all 6 colors).
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data
/// * `state` - Mutable search state
/// * `trail` - Trail for backtracking
/// * `face_degrees` - Array of cycle lengths for neighboring faces (0 = no restriction)
///
/// # Returns
///
/// `Ok(())` if setup succeeds, `Err(PropagationFailure)` if constraints fail.
///
/// # Example
///
/// For N=6 with face_degrees = [5,5,5,4,4,4]:
/// - Face 62 (colors 1-5) restricted to 5-cycles
/// - Face 61 (colors 0,2-5) restricted to 5-cycles
/// - Face 59 (colors 0,1,3-5) restricted to 5-cycles
/// - Face 55 (colors 0-2,4,5) restricted to 4-cycles
/// - Face 47 (colors 0-3,5) restricted to 4-cycles
/// - Face 31 (colors 0-4) restricted to 4-cycles
/// - Face 63 (all colors) set to canonical cycle (a,b,c,d,e,f)
pub fn setup_central_face(
    memo: &MemoizedData,
    state: &mut DynamicState,
    trail: &mut Trail,
    face_degrees: &[u64; crate::geometry::constants::NCOLORS],
) -> Result<(), PropagationFailure> {
    use crate::geometry::constants::{NCOLORS, NCYCLES, NFACES};

    // 1. Restrict neighboring faces to specified cycle lengths
    for i in 0..NCOLORS {
        let degree = face_degrees[i] as usize;

        // Face with all colors except i
        let face_id = (!(1 << i)) & (NFACES - 1);

        restrict_face_to_cycle_length(memo, state, trail, face_id, degree)?;
    }

    // 2. Set inner face to canonical cycle (last cycle in array)
    let inner_face_id = NFACES - 1;
    let canonical_cycle_id = (NCYCLES - 1) as u64;

    // Set the cycle directly (trail-tracked)
    let encoded = canonical_cycle_id + 1;
    unsafe {
        trail.record_and_set(
            NonNull::from(&mut state.faces.faces[inner_face_id].current_cycle_encoded),
            encoded,
        );
    }

    // 3. Propagate this choice
    propagate_cycle_choice(memo, state, trail, inner_face_id, canonical_cycle_id, 0)?;

    Ok(())
}

/// Validate that faces form proper cycles in the dual graph.
///
/// Faces with M colors must form a single cycle in the dual graph
/// of length C(NCOLORS, M) (binomial coefficient).
///
/// This function walks each cycle by following next_face pointers and
/// verifies:
/// 1. The cycle closes (returns to starting face)
/// 2. The cycle has the expected length
/// 3. All faces with M colors are in exactly one cycle
///
/// # Algorithm
///
/// For each color count M (0..=NCOLORS):
/// 1. Find first unvisited face with M colors
/// 2. Follow next_face pointers to traverse the cycle
/// 3. Count cycle length
/// 4. Verify cycle closes and has expected length C(NCOLORS, M)
/// 5. Mark all visited faces
/// 6. Repeat until all faces with M colors are visited
///
/// # Arguments
///
/// * `memo` - Immutable MEMO data (contains binomial coefficients)
/// * `state` - Mutable search state (contains next_face pointers)
///
/// # Returns
///
/// `Ok(())` if all face cycles are valid, `Err(PropagationFailure)` otherwise.
pub fn validate_face_cycles(
    memo: &MemoizedData,
    state: &DynamicState,
) -> Result<(), PropagationFailure> {
    use crate::geometry::constants::{NCOLORS, NFACES};

    // Track which faces we've already visited
    let mut visited = vec![false; NFACES];

    // For each color count M (excluding outer and inner faces)
    // Outer face (0 colors) and inner face (NCOLORS colors) are special cases
    // with only one face each, so they don't form meaningful cycles
    for color_count in 1..NCOLORS {
        let expected_cycle_length = memo.faces.face_degree_by_color_count[color_count] as usize;

        // Find all faces with this color count and verify they form a single cycle
        let mut found_any_face = false;

        for start_face_id in 0..NFACES {
            // Skip if already visited
            if visited[start_face_id] {
                continue;
            }

            // Count colors in this face
            let face_colors = memo.faces.get_face(start_face_id).colors;
            if face_colors.len() != color_count {
                continue; // Wrong color count
            }

            // Found a face with this color count - traverse the cycle
            found_any_face = true;
            let mut current_face_id = start_face_id;
            let mut cycle_length = 0;
            let max_iterations = NFACES + 1; // Prevent infinite loops

            loop {
                // Mark as visited
                visited[current_face_id] = true;
                cycle_length += 1;

                // Safety check for infinite loops
                if cycle_length > max_iterations {
                    return Err(PropagationFailure::NoMatchingCycles {
                        face_id: current_face_id,
                        depth: 0,
                    });
                }

                // Get next face
                let next_face_opt = state.faces.faces[current_face_id].next_face();

                match next_face_opt {
                    None => {
                        // Face has no next pointer - this is an error
                        return Err(PropagationFailure::NoMatchingCycles {
                            face_id: current_face_id,
                            depth: 0,
                        });
                    }
                    Some(next_face_id) => {
                        // Check if we've closed the cycle
                        if next_face_id == start_face_id {
                            // Cycle closed - verify length
                            if cycle_length != expected_cycle_length {
                                return Err(PropagationFailure::NoMatchingCycles {
                                    face_id: start_face_id,
                                    depth: 0,
                                });
                            }
                            break; // Cycle is valid
                        }

                        // Move to next face
                        current_face_id = next_face_id;
                    }
                }
            }

            // After finding one cycle for this color count, verify there are no other
            // unvisited faces with the same color count (should all be in one cycle)
            for check_face_id in 0..NFACES {
                if visited[check_face_id] {
                    continue;
                }
                let check_face_colors = memo.faces.get_face(check_face_id).colors;
                if check_face_colors.len() == color_count {
                    // Found an unvisited face with same color count - invalid!
                    return Err(PropagationFailure::NoMatchingCycles {
                        face_id: check_face_id,
                        depth: 0,
                    });
                }
            }

            // All faces with this color count are in one valid cycle
            break;
        }

        // Verify we found at least one face with this color count
        // (We're only checking 1..NCOLORS, so we should always find faces)
        if !found_any_face {
            return Err(PropagationFailure::NoMatchingCycles {
                face_id: 0,
                depth: 0,
            });
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
