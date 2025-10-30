// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Symmetry checking for degree sequences using dihedral groups.
//!
//! This module provides functionality to check whether a sequence of face degrees
//! is canonical under the symmetry group of rotations and reflections.
//!
//! ## Module Structure
//!
//! - `canonical`: Canonicality checking under dihedral symmetry (includes dihedral group constants)
//! - `mod`: Public API and re-exports

pub mod canonical;

// Re-export main types and constants
pub use canonical::{
    check_solution_canonicality, check_symmetry, SymmetryType, DIHEDRAL_GROUP_3,
    DIHEDRAL_GROUP_4, DIHEDRAL_GROUP_5, DIHEDRAL_GROUP_6,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::constants::NCOLORS;

    #[test]
    fn test_dihedral_group_3_structure() {
        let d3 = DIHEDRAL_GROUP_3;

        // First 3 elements are rotations
        assert_eq!(d3[0], [0, 1, 2]); // identity
        assert_eq!(d3[1], [1, 2, 0]); // rotate by 1
        assert_eq!(d3[2], [2, 0, 1]); // rotate by 2

        // Next 3 elements are reflections
        assert_eq!(d3[3], [2, 1, 0]); // reflect
        assert_eq!(d3[4], [0, 2, 1]); // reflect + rotate by 1
        assert_eq!(d3[5], [1, 0, 2]); // reflect + rotate by 2
    }

    #[test]
    fn test_dihedral_group_6_structure() {
        let d6 = DIHEDRAL_GROUP_6;

        // First element is identity
        assert_eq!(d6[0], [0, 1, 2, 3, 4, 5]);

        // Second element is rotate by 1
        assert_eq!(d6[1], [1, 2, 3, 4, 5, 0]);

        // Seventh element is reflection (first reflection, no rotation)
        assert_eq!(d6[6], [5, 4, 3, 2, 1, 0]);
    }

    #[test]
    fn test_current_ncolors_dihedral_group() {
        // Test the dihedral group for the current NCOLORS
        #[cfg(feature = "ncolors_3")]
        let group: &[[u8; NCOLORS]] = &DIHEDRAL_GROUP_3;

        #[cfg(feature = "ncolors_4")]
        let group: &[[u8; NCOLORS]] = &DIHEDRAL_GROUP_4;

        #[cfg(feature = "ncolors_5")]
        let group: &[[u8; NCOLORS]] = &DIHEDRAL_GROUP_5;

        #[cfg(any(
            feature = "ncolors_6",
            not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
        ))]
        let group: &[[u8; NCOLORS]] = &DIHEDRAL_GROUP_6;

        // Should have 2 * NCOLORS elements
        assert_eq!(group.len(), 2 * NCOLORS);

        // First element should be identity
        for (i, &value) in group[0].iter().enumerate() {
            assert_eq!(value, i as u8);
        }

        // Last NCOLORS elements should be reflections
        // First reflection should reverse the sequence
        let first_reflection_idx = NCOLORS;
        for (i, &value) in group[first_reflection_idx].iter().enumerate() {
            assert_eq!(value, (NCOLORS - 1 - i) as u8);
        }
    }
}
