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
/// 2. Sort resulting sequences in descending lexicographic order
/// 3. Compare input with maximum:
///    - If input != max → NonCanonical (reject)
///    - If input == max == second → Equivocal (accept, has symmetry)
///    - If input == max > second → Canonical (accept)
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
