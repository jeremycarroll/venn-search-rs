// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Post-search validation of face cycles.
//!
//! This module validates that faces form proper cycles in the dual graph.
//! Faces with M colors must form a single cycle of length C(NCOLORS, M).

use crate::context::{DynamicState, MemoizedData};

use super::errors::PropagationFailure;

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
    let mut visited = [false; NFACES];

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
