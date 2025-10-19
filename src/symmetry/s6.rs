// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! S6 symmetry checking for face degree sequences.
//!
//! This module implements canonicality checking for sequences of face degrees
//! under the dihedral group D_NCOLORS (rotations and reflections).

use crate::geometry::constants::NCOLORS;

/// Result of symmetry checking for a face degree sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymmetryType {
    /// Sequence is lexicographically maximal (uniquely canonical).
    Canonical,
    /// Sequence is tied for maximum (has rotational symmetry).
    Equivocal,
    /// Sequence is not maximal (reject - not canonical).
    NonCanonical,
}

/// Check whether a face degree sequence is canonical under dihedral symmetry.
///
/// Algorithm:
/// 1. Apply all 2*NCOLORS permutations from the dihedral group
/// 2. Scan all resulting sequences to find the lexicographically maximal sequence and count how many times it occurs
/// 3. Compare input with maximum:
///    - If input != max → NonCanonical (reject)
///    - If input == max and occurs more than once → Equivocal (accept, has symmetry)
///    - If input == max and occurs only once → Canonical (accept)
///
/// # Examples
///
/// For NCOLORS=6:
/// - `[6,6,3,5,4,3]` → Canonical (unique maximum)
/// - `[6,6,3,4,5,3]` → NonCanonical (reflection of above is larger)
/// - `[5,4,5,4,5,4]` → Equivocal (rotational symmetry)
pub fn check_symmetry(degrees: &[u8; NCOLORS]) -> SymmetryType {
    // Get the appropriate dihedral group for the current NCOLORS using conditional compilation
    #[cfg(feature = "ncolors_3")]
    let group: &[[u8; NCOLORS]] = &super::DIHEDRAL_GROUP_3;

    #[cfg(feature = "ncolors_4")]
    let group: &[[u8; NCOLORS]] = &super::DIHEDRAL_GROUP_4;

    #[cfg(feature = "ncolors_5")]
    let group: &[[u8; NCOLORS]] = &super::DIHEDRAL_GROUP_5;

    #[cfg(any(
        feature = "ncolors_6",
        not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
    ))]
    let group: &[[u8; NCOLORS]] = &super::DIHEDRAL_GROUP_6;

    // Find the lexicographically maximal permutation and count ties
    let mut max_perm = [0u8; NCOLORS];
    let mut max_count = 0;
    let mut first = true;

    for permutation in group.iter() {
        let mut permuted = [0u8; NCOLORS];
        for (i, &perm_idx) in permutation.iter().enumerate() {
            permuted[i] = degrees[perm_idx as usize];
        }
        if first {
            max_perm = permuted;
            max_count = 1;
            first = false;
        } else {
            match permuted.cmp(&max_perm) {
                std::cmp::Ordering::Greater => {
                    max_perm = permuted;
                    max_count = 1;
                }
                std::cmp::Ordering::Equal => {
                    max_count += 1;
                }
                std::cmp::Ordering::Less => {}
            }
        }
    }

    if degrees != &max_perm {
        return SymmetryType::NonCanonical;
    }
    if max_count > 1 {
        return SymmetryType::Equivocal;
    }
    SymmetryType::Canonical
}

/// Apply a dihedral permutation to a face colorset (bitmask).
///
/// # Algorithm
///
/// For each bit position i in the face_id:
/// - If bit i is set in face_id, set bit permutation[i] in the result
///
/// This transforms face IDs under dihedral group operations.
///
/// # Example
///
/// ```text
/// face_id = 0b00011 (colors 0 and 1)
/// permutation = [1, 0, 2, 3, 4, 5] (swap colors 0 and 1)
/// result = 0b00011 (colors 0 and 1, swapped back)
/// ```
fn colorset_permute(face_id: usize, permutation: &[u8; NCOLORS]) -> usize {
    use crate::geometry::constants::NCOLORS;
    let mut result = 0;
    for color_idx in 0..NCOLORS {
        if (face_id & (1 << color_idx)) != 0 {
            result |= 1 << permutation[color_idx];
        }
    }
    result
}

/// Extract face cycle lengths in canonical order (SEQUENCE_ORDER).
///
/// For each face in SEQUENCE_ORDER, gets the cycle length for that face from state.
/// Returns an array of NFACES cycle lengths.
///
/// # Arguments
///
/// * `state` - The DynamicState containing face cycle assignments
///
/// # Returns
///
/// Array of cycle lengths in SEQUENCE_ORDER: [u8; NFACES]
///
/// # Panics
///
/// Panics if any face is unassigned (all faces must have cycles before validation).
fn get_face_degrees_in_canonical_order(
    state: &crate::context::DynamicState,
) -> [u8; crate::geometry::constants::NFACES] {
    use crate::geometry::constants::{NFACES, SEQUENCE_ORDER};

    let mut degrees = [0u8; NFACES];
    for (order_idx, &face_id) in SEQUENCE_ORDER.iter().enumerate() {
        let cycle_id = state.faces.faces[face_id]
            .current_cycle()
            .expect("Face must be assigned before validation");

        // Get cycle length from cycles array
        // For now, we'll need access to MemoizedData to get cycle length
        // We'll pass cycles as a parameter
        degrees[order_idx] = cycle_id as u8; // Placeholder - will be fixed
    }
    degrees
}

/// Check if a complete solution is canonical under dihedral symmetry.
///
/// This is the final validation check that determines if a solution should be
/// counted. It applies all 2*NCOLORS dihedral transformations to the face cycle
/// assignments and checks if the current assignment is lexicographically maximal.
///
/// # Algorithm
///
/// 1. Extract cycle lengths for all faces in SEQUENCE_ORDER
/// 2. For each dihedral permutation:
///    - Apply permutation to face IDs using colorset_permute
///    - Look up permuted faces in INVERSE_SEQUENCE_ORDER
///    - Extract cycle lengths in this permuted order
/// 3. Find lexicographically maximal sequence
/// 4. Compare input with maximum:
///    - If input != max → NonCanonical (reject, don't count)
///    - If input == max and occurs > 1 time → Equivocal (accept, count)
///    - If input == max and occurs == 1 time → Canonical (accept, count)
///
/// # Arguments
///
/// * `state` - The DynamicState with all faces assigned
/// * `memo` - The MemoizedData for cycle lookups
///
/// # Returns
///
/// SymmetryType indicating whether to accept (Canonical/Equivocal) or reject (NonCanonical).
pub fn check_solution_canonicality(
    state: &crate::context::DynamicState,
    memo: &crate::context::MemoizedData,
) -> SymmetryType {
    use crate::geometry::constants::{INVERSE_SEQUENCE_ORDER, NCOLORS, NFACES, SEQUENCE_ORDER};

    // Get the appropriate dihedral group
    #[cfg(feature = "ncolors_3")]
    let group: &[[u8; NCOLORS]] = &super::DIHEDRAL_GROUP_3;

    #[cfg(feature = "ncolors_4")]
    let group: &[[u8; NCOLORS]] = &super::DIHEDRAL_GROUP_4;

    #[cfg(feature = "ncolors_5")]
    let group: &[[u8; NCOLORS]] = &super::DIHEDRAL_GROUP_5;

    #[cfg(any(
        feature = "ncolors_6",
        not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
    ))]
    let group: &[[u8; NCOLORS]] = &super::DIHEDRAL_GROUP_6;

    // Extract cycle lengths for all faces in canonical order
    let mut degrees = [0u8; NFACES];
    for (order_idx, &face_id) in SEQUENCE_ORDER.iter().enumerate() {
        let cycle_id = state.faces.faces[face_id]
            .current_cycle()
            .expect("Face must be assigned before validation");
        let cycle = memo.cycles.get(cycle_id);
        degrees[order_idx] = cycle.len() as u8;
    }

    // Debug: Print once for first solution
    static mut FIRST_CALL: bool = true;
    unsafe {
        if FIRST_CALL {
            eprintln!("[S6] First call to check_solution_canonicality");
            eprintln!("[S6] Degrees: {:?}", &degrees[..10]);
            FIRST_CALL = false;
        }
    }

    // Find lexicographically maximal permutation and count ties
    let mut max_perm = [0u8; NFACES];
    let mut max_count = 0;
    let mut first = true;

    // Debug: Check first permutation
    static mut DEBUG_FIRST: bool = true;

    for (perm_idx, permutation) in group.iter().enumerate() {
        let mut permuted = [0u8; NFACES];

        // Apply this permutation to all face IDs
        for (order_idx, &face_id) in SEQUENCE_ORDER.iter().enumerate() {
            // Transform face_id by permuting its color bits
            let permuted_face_id = colorset_permute(face_id, permutation);

            // Look up where this permuted face appears in canonical order
            let permuted_order_idx = INVERSE_SEQUENCE_ORDER[permuted_face_id];

            // Copy the cycle length from the original order position
            permuted[permuted_order_idx] = degrees[order_idx];

            // Debug: Print first few transformations on first call
            unsafe {
                if DEBUG_FIRST && perm_idx < 2 && order_idx < 3 {
                    eprintln!(
                        "[S6] perm[{}]: face {} → face {} (order {} → {}), degree={}",
                        perm_idx,
                        face_id,
                        permuted_face_id,
                        order_idx,
                        permuted_order_idx,
                        degrees[order_idx]
                    );
                }
            }
        }

        unsafe {
            if DEBUG_FIRST && perm_idx == 0 {
                eprintln!("[S6] Identity permutation: {:?}", &permuted[..10]);
            }
            if DEBUG_FIRST && perm_idx == 1 {
                eprintln!("[S6] Second permutation: {:?}", &permuted[..10]);
                eprintln!("[S6] Comparison: {:?}", permuted.cmp(&degrees));
                DEBUG_FIRST = false;
            }
        }

        if first {
            max_perm = permuted;
            max_count = 1;
            first = false;
        } else {
            match permuted.cmp(&max_perm) {
                std::cmp::Ordering::Greater => {
                    max_perm = permuted;
                    max_count = 1;
                }
                std::cmp::Ordering::Equal => {
                    max_count += 1;
                }
                std::cmp::Ordering::Less => {}
            }
        }
    }

    // Debug: Log the result
    unsafe {
        static mut CALL_COUNT: usize = 0;
        CALL_COUNT += 1;
        if CALL_COUNT <= 5 || CALL_COUNT % 1000 == 0 {
            let result_type = if degrees != max_perm {
                "NonCanonical"
            } else if max_count > 1 {
                "Equivocal"
            } else {
                "Canonical"
            };
            eprintln!(
                "[S6] Solution #{}: {} (max_count={})",
                CALL_COUNT, result_type, max_count
            );
        }
    }

    if degrees != max_perm {
        return SymmetryType::NonCanonical;
    }
    if max_count > 1 {
        return SymmetryType::Equivocal;
    }
    SymmetryType::Canonical
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(any(
        feature = "ncolors_6",
        not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
    ))]
    fn test_canonical_sequence_n6() {
        // [6,6,3,5,4,3] is canonical for NCOLORS=6
        let degrees = [6, 6, 3, 5, 4, 3];
        assert_eq!(check_symmetry(&degrees), SymmetryType::Canonical);
    }

    #[test]
    #[cfg(any(
        feature = "ncolors_6",
        not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
    ))]
    fn test_noncanonical_sequence_n6() {
        // [6,6,3,4,5,3] is the reflection of [6,6,3,5,4,3]
        // and is lexicographically smaller
        let degrees = [6, 6, 3, 4, 5, 3];
        assert_eq!(check_symmetry(&degrees), SymmetryType::NonCanonical);
    }

    #[test]
    #[cfg(any(
        feature = "ncolors_6",
        not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
    ))]
    fn test_equivocal_sequence_n6() {
        // [5,4,5,4,5,4] has rotational symmetry for NCOLORS=6
        let degrees = [5, 4, 5, 4, 5, 4];
        assert_eq!(check_symmetry(&degrees), SymmetryType::Equivocal);
    }

    #[test]
    #[cfg(feature = "ncolors_3")]
    fn test_equivocal_sequence_n3() {
        // [3,3,3] is equivocal (all the same) for NCOLORS=3
        let degrees = [3, 3, 3];
        assert_eq!(check_symmetry(&degrees), SymmetryType::Equivocal);
    }

    #[test]
    fn test_uniform_sequence_is_equivocal() {
        // A sequence with all same values should be equivocal
        // (has rotational and reflective symmetry)
        let value = match NCOLORS {
            3 => 3,
            4 => 4,
            5 => 4,
            6 => 5,
            _ => unreachable!(),
        };

        let degrees = [value; NCOLORS];
        let result = check_symmetry(&degrees);

        // Uniform sequences have full symmetry, so should be Equivocal
        assert_eq!(result, SymmetryType::Equivocal);
    }

    #[test]
    fn test_descending_sequence() {
        // Test a descending sequence - should be Canonical or Equivocal
        let degrees: [u8; NCOLORS] = {
            let mut arr = [0u8; NCOLORS];
            for (i, item) in arr.iter_mut().enumerate() {
                *item = (NCOLORS - i) as u8;
            }
            arr
        };

        let result = check_symmetry(&degrees);
        // Descending sequences are typically canonical unless they have symmetry
        assert_ne!(result, SymmetryType::NonCanonical);
    }
}
