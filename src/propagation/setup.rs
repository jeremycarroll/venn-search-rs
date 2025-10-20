// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Central face setup and initialization.
//!
//! This module handles the initial configuration of the central face
//! based on degree signatures from InnerFacePredicate or command-line flags.
//! Sets up the inner face and restricts neighboring faces to specific cycle lengths.

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::CycleSet;
use crate::trail::Trail;
use std::ptr::NonNull;

use super::core::{propagate_cycle_choice, restrict_face_cycles};
use super::errors::PropagationFailure;

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
    #[allow(clippy::needless_range_loop)] // i used for bit manipulation
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
