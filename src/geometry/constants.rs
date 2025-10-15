// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Compile-time constants for Venn diagram geometry.
//!
//! This module defines NCOLORS (number of curves) and all derived constants.
//! NCOLORS can be configured at compile time via cargo features.
//!
//! # Supported NCOLORS values
//!
//! - 3: Simple 3-Venn diagrams (2 cycles)
//! - 4: 4-Venn diagrams (14 cycles)
//! - 5: 5-Venn diagrams (74 cycles)
//! - 6: 6-Venn diagrams (394 cycles) - **default**
//!
//! # Example
//!
//! ```bash
//! # Default: NCOLORS=6
//! cargo build
//!
//! # Build for NCOLORS=4
//! cargo build --features ncolors_4
//! ```

/// Number of colors (curves) in the Venn diagram.
///
/// This is configurable at compile time via cargo features:
/// - `ncolors_3` → NCOLORS=3
/// - `ncolors_4` → NCOLORS=4
/// - `ncolors_5` → NCOLORS=5
/// - `ncolors_6` → NCOLORS=6 (explicit)
/// - (default) → NCOLORS=6 (when no feature specified)
#[cfg(not(any(
    feature = "ncolors_3",
    feature = "ncolors_4",
    feature = "ncolors_5",
    feature = "ncolors_6"
)))]
pub const NCOLORS: usize = 6;

#[cfg(feature = "ncolors_3")]
pub const NCOLORS: usize = 3;

#[cfg(feature = "ncolors_4")]
pub const NCOLORS: usize = 4;

#[cfg(feature = "ncolors_5")]
pub const NCOLORS: usize = 5;

#[cfg(feature = "ncolors_6")]
pub const NCOLORS: usize = 6;

/// Total number of faces in the diagram (2^NCOLORS).
///
/// Each face corresponds to a subset of the NCOLORS curves.
/// For NCOLORS=6, this is 64 faces.
pub const NFACES: usize = 1 << NCOLORS;

/// Maximum number of colors in any cycle (equals NCOLORS).
pub const MAX_CYCLE_LENGTH: usize = NCOLORS;

/// Compute factorial at compile time.
const fn factorial(n: usize) -> usize {
    match n {
        0 | 1 => 1,
        _ => n * factorial(n - 1),
    }
}

/// Compute binomial coefficient (n choose k) at compile time.
///
/// Returns the number of ways to choose k items from n items.
const fn choose(n: usize, k: usize) -> usize {
    if k > n {
        0
    } else if k == 0 || k == n {
        1
    } else {
        factorial(n) / (factorial(k) * factorial(n - k))
    }
}

/// Total number of possible facial cycles for the current NCOLORS.
///
/// # What is a facial cycle?
///
/// A facial cycle is a cyclic sequence of colors (curve labels) that bound a face
/// in the Venn diagram. For example, "(abc)" means the face is bounded by curves
/// a, b, and c in that cyclic order.
///
/// # Counting facial cycles
///
/// A cycle can have between 3 and NCOLORS colors (a face must be bounded by at
/// least 3 curves). Each face has **at most one edge of each color** - this is a
/// fundamental lemma of simple Venn diagrams (see docs/MATH.md Initial Observations).
/// Therefore, cycles are simply subsets of colors arranged in cyclic order.
///
/// For each possible cycle length k:
///
/// 1. **Choose which colors**: C(NCOLORS, k) ways to select k colors
/// 2. **Order them cyclically**: (k-1)! ways to arrange k items in a cycle
///    - Linear orderings: k!
///    - But rotations are equivalent: divide by k
///    - Result: k!/k = (k-1)!
///
/// For NCOLORS=6, we count cycles of length 3, 4, 5, and 6:
/// - 3-color cycles: C(6,3) × 2! = 20 × 2 = 40
/// - 4-color cycles: C(6,4) × 3! = 15 × 6 = 90
/// - 5-color cycles: C(6,5) × 4! = 6 × 24 = 144
/// - 6-color cycles: C(6,6) × 5! = 1 × 120 = 120
/// - **Total: 394 cycles**
///
/// # Formula
///
/// NCYCLES = Σ(k=3 to NCOLORS) C(NCOLORS, k) × (k-1)!
///
/// This can be rewritten as:
/// - For k colors omitted: C(NCOLORS, k) × (NCOLORS-k-1)!
/// - Summing over k=0 to NCOLORS-3 gives the same result
/// - This explains the pattern C(n,0)×(n-1)! + C(n,1)×(n-2)! + ...
///
/// # Values
///
/// - NCOLORS=3: 2 cycles (only length-3 cycles possible)
/// - NCOLORS=4: 14 cycles
/// - NCOLORS=5: 74 cycles
/// - NCOLORS=6: 394 cycles
pub const NCYCLES: usize = match NCOLORS {
    3 => choose(3, 0) * factorial(2),
    4 => choose(4, 0) * factorial(3) + choose(4, 1) * factorial(2),
    5 => choose(5, 0) * factorial(4) + choose(5, 1) * factorial(3) + choose(5, 2) * factorial(2),
    6 => {
        choose(6, 0) * factorial(5)
            + choose(6, 1) * factorial(4)
            + choose(6, 2) * factorial(3)
            + choose(6, 3) * factorial(2)
    }
    _ => panic!("Unsupported NCOLORS value"),
};

/// Number of u64 words needed to represent a set of NCYCLES cycles as a bitset.
///
/// Each cycle has a unique ID in 0..NCYCLES, and we use a bitset to represent
/// sets of cycles efficiently.
pub const CYCLESET_LENGTH: usize = NCYCLES.div_ceil(64);

/// Maximum number of vertices (points) in the diagram.
///
/// Formula: 2^(NCOLORS-2) * NCOLORS * (NCOLORS-1)
///
/// For NCOLORS=6: 2^4 * 6 * 5 = 16 * 30 = 480 vertices
pub const NPOINTS: usize = (1 << (NCOLORS - 2)) * NCOLORS * (NCOLORS - 1);

/// Total degree of the NCOLORS symmetric faces that border the central face.
///
/// The central face is the region inside all NCOLORS curves. The NCOLORS faces
/// bordering it are the "codimension-1" faces - each one excludes exactly one curve.
/// These faces have rotational symmetry under permutation of which curve is excluded.
///
/// # Formula
///
/// `2 * C(NCOLORS, NCOLORS-1) + C(NCOLORS, NCOLORS-2)`
///
/// # Examples
///
/// - **NCOLORS=3**: 3 two-sided faces (A∩B, B∩C, A∩C), each degree 3 → total 9
/// - **NCOLORS=6**: 6 five-sided faces, degrees sum to 27
pub const TOTAL_CENTRAL_NEIGHBOR_DEGREE: usize =
    2 * choose(NCOLORS, NCOLORS - 1) + choose(NCOLORS, NCOLORS - 2);

/// Compile-time assertion that we're on a 64-bit architecture.
///
/// The trail system and various bitset operations assume 64-bit pointers and words.
/// This assertion will cause a compile-time error on 32-bit systems.
const _: () = assert!(
    std::mem::size_of::<usize>() == 8,
    "64-bit architecture required"
);

/// Compile-time assertion that u64 is 8 bytes.
///
/// The trail system relies on u64 being exactly 64 bits.
const _: () = assert!(std::mem::size_of::<u64>() == 8, "u64 must be 8 bytes");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(2), 2);
        assert_eq!(factorial(3), 6);
        assert_eq!(factorial(4), 24);
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(6), 720);
    }

    #[test]
    fn test_choose() {
        assert_eq!(choose(6, 0), 1);
        assert_eq!(choose(6, 1), 6);
        assert_eq!(choose(6, 2), 15);
        assert_eq!(choose(6, 3), 20);
        assert_eq!(choose(5, 2), 10);
        assert_eq!(choose(4, 1), 4);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // Validates compile-time constant
    fn test_ncolors_in_valid_range() {
        assert!(
            NCOLORS >= 3 && NCOLORS <= 6,
            "NCOLORS must be 3, 4, 5, or 6"
        );
    }

    #[test]
    fn test_nfaces() {
        assert_eq!(NFACES, 1 << NCOLORS);
        match NCOLORS {
            3 => assert_eq!(NFACES, 8),
            4 => assert_eq!(NFACES, 16),
            5 => assert_eq!(NFACES, 32),
            6 => assert_eq!(NFACES, 64),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_ncycles() {
        match NCOLORS {
            3 => assert_eq!(NCYCLES, 2),   // C(3,0)*2! = 1*2 = 2
            4 => assert_eq!(NCYCLES, 14),  // C(4,0)*3! + C(4,1)*2! = 1*6 + 4*2 = 14
            5 => assert_eq!(NCYCLES, 74), // C(5,0)*4! + C(5,1)*3! + C(5,2)*2! = 1*24 + 5*6 + 10*2 = 74
            6 => assert_eq!(NCYCLES, 394), // C(6,0)*5! + C(6,1)*4! + C(6,2)*3! + C(6,3)*2! = 1*120 + 6*24 + 15*6 + 20*2 = 394
            _ => unreachable!(),
        }
    }

    #[test]
    #[allow(clippy::assertions_on_constants)] // Validates compile-time constant
    fn test_cycleset_length() {
        // Should have enough u64s to hold NCYCLES bits
        assert!(CYCLESET_LENGTH * 64 >= NCYCLES);
        // Should not have more than 63 unused bits
        assert!(CYCLESET_LENGTH * 64 - NCYCLES < 64);
    }

    #[test]
    fn test_npoints() {
        let expected = (1 << (NCOLORS - 2)) * NCOLORS * (NCOLORS - 1);
        assert_eq!(NPOINTS, expected);

        match NCOLORS {
            3 => assert_eq!(NPOINTS, 2 * 3 * 2),  // = 12
            4 => assert_eq!(NPOINTS, 4 * 4 * 3),  // = 48
            5 => assert_eq!(NPOINTS, 8 * 5 * 4),  // = 160
            6 => assert_eq!(NPOINTS, 16 * 6 * 5), // = 480
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_total_central_neighbor_degree() {
        let expected = 2 * choose(NCOLORS, NCOLORS - 1) + choose(NCOLORS, NCOLORS - 2);
        assert_eq!(TOTAL_CENTRAL_NEIGHBOR_DEGREE, expected);

        match NCOLORS {
            3 => assert_eq!(TOTAL_CENTRAL_NEIGHBOR_DEGREE, 9),  // 2*C(3,2) + C(3,1) = 2*3 + 3 = 9
            4 => assert_eq!(TOTAL_CENTRAL_NEIGHBOR_DEGREE, 14), // 2*C(4,3) + C(4,2) = 2*4 + 6 = 14
            5 => assert_eq!(TOTAL_CENTRAL_NEIGHBOR_DEGREE, 20), // 2*C(5,4) + C(5,3) = 2*5 + 10 = 20
            6 => assert_eq!(TOTAL_CENTRAL_NEIGHBOR_DEGREE, 27), // 2*C(6,5) + C(6,4) = 2*6 + 15 = 27
            _ => unreachable!(),
        }
    }
}
