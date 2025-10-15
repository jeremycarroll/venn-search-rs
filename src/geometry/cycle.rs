// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Cycle type for facial cycles in Venn diagrams.
//!
//! A facial cycle represents a cyclic sequence of colors that bound a face.
//! For example, "(abc)" represents a face bounded by curves a, b, and c in that order.
//!
//! Cycles are represented as sequences with rotation equivalence: (abc) = (bca) = (cab).
//! We canonicalize cycles by always starting with the smallest color.
//!
//! # Examples
//!
//! ```
//! use venn_search::geometry::{Cycle, Color};
//!
//! // Create a cycle from colors
//! let colors = vec![Color::new(0), Color::new(1), Color::new(2)];
//! let cycle = Cycle::new(&colors);
//!
//! assert_eq!(cycle.len(), 3);
//! assert!(cycle.contains_sequence(Color::new(0), Color::new(1)));
//! assert_eq!(format!("{}", cycle), "(abc)");
//! ```
//!
use crate::geometry::{constants::NCOLORS, Color, ColorSet};
use std::fmt;

/// A cycle ID (index into the global Cycles array).
///
/// Each possible facial cycle is assigned a unique ID from 0..NCYCLES-1.
pub type CycleId = u64;

/// A facial cycle representing a cyclic sequence of colors.
///
/// Cycles represent the boundary of a face as a sequence of colors (curve labels).
/// They are stored in canonical form (starting with the smallest color).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    /// Number of colors in this cycle (3 to NCOLORS)
    length: u8,
    /// The colors in cyclic order (canonical: starts with smallest)
    colors: [Color; 6], // Fixed size array, only use first `length` elements
    /// Bitset of which colors are present
    colorset: ColorSet,
}

impl Cycle {
    /// Create a new cycle from a slice of colors.
    ///
    /// The cycle will be canonicalized (rotated to start with smallest color).
    ///
    /// # Panics
    ///
    /// Panics if the slice has fewer than 3 colors or more than NCOLORS colors.
    pub fn new(colors: &[Color]) -> Self {
        assert!(
            colors.len() >= 3 && colors.len() <= NCOLORS,
            "Cycle must have 3..={} colors, got {}",
            NCOLORS,
            colors.len()
        );

        let mut cycle_colors = [Color::new(0); 6];
        let length = colors.len() as u8;

        // Find the minimum color and its position
        let (min_pos, _min_color) = colors.iter().enumerate().min_by_key(|(_, &c)| c).unwrap();

        // Copy colors starting from min position (canonical rotation)
        for i in 0..colors.len() {
            cycle_colors[i] = colors[(min_pos + i) % colors.len()];
        }

        // Build colorset
        let colorset = ColorSet::from_colors(colors);

        Self {
            length,
            colors: cycle_colors,
            colorset,
        }
    }

    /// Get the number of colors in this cycle.
    pub fn len(&self) -> usize {
        self.length as usize
    }

    /// Check if this cycle is empty (should never happen, but included for completeness).
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Get the colors in this cycle as a slice.
    pub fn colors(&self) -> &[Color] {
        &self.colors[..self.length as usize]
    }

    /// Get the colorset (bitset of which colors are present).
    pub fn colorset(&self) -> ColorSet {
        self.colorset
    }

    /// Check if this cycle contains the sequence color_a followed by color_b.
    ///
    /// Returns true if there's an edge from color_a to color_b in the cycle.
    pub fn contains_sequence(&self, color_a: Color, color_b: Color) -> bool {
        for i in 0..self.len() {
            let next_i = (i + 1) % self.len();
            if self.colors[i] == color_a && self.colors[next_i] == color_b {
                return true;
            }
        }
        false
    }

    /// Check if this cycle contains the triple (a, b, c) in order.
    ///
    /// Returns true if there's a vertex where colors a, b, c meet in that order.
    pub fn contains_triple(&self, a: Color, b: Color, c: Color) -> bool {
        for i in 0..self.len() {
            let i1 = (i + 1) % self.len();
            let i2 = (i + 2) % self.len();
            if self.colors[i] == a && self.colors[i1] == b && self.colors[i2] == c {
                return true;
            }
        }
        false
    }

    /// Find the index of a color in this cycle.
    ///
    /// Returns None if the color is not in the cycle.
    pub fn index_of(&self, color: Color) -> Option<usize> {
        self.colors[..self.len()].iter().position(|&c| c == color)
    }

    /// Reverse the direction of this cycle.
    ///
    /// Returns a new cycle with colors in reverse order (still canonical).
    /// For example, (abc) reversed is (acb) (keeping 'a' first, reversing rest).
    pub fn reverse(&self) -> Self {
        let mut reversed_colors = Vec::with_capacity(self.len());

        // Keep first color, reverse the rest
        reversed_colors.push(self.colors[0]);
        for i in (1..self.len()).rev() {
            reversed_colors.push(self.colors[i]);
        }

        Self::new(&reversed_colors)
    }
}

impl fmt::Display for Cycle {
    /// Format cycle as "(abc)" where a, b, c are color characters.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for i in 0..self.len() {
            write!(f, "{}", self.colors[i].to_char())?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_new() {
        let colors = vec![Color::new(0), Color::new(1), Color::new(2)];
        let cycle = Cycle::new(&colors);

        assert_eq!(cycle.len(), 3);
        assert_eq!(
            cycle.colors(),
            &[Color::new(0), Color::new(1), Color::new(2)]
        );
    }

    #[test]
    fn test_cycle_canonicalization() {
        // (bca) should be canonicalized to (abc)
        let colors1 = vec![Color::new(1), Color::new(2), Color::new(0)];
        let cycle1 = Cycle::new(&colors1);

        let colors2 = vec![Color::new(0), Color::new(1), Color::new(2)];
        let cycle2 = Cycle::new(&colors2);

        assert_eq!(cycle1, cycle2);
        assert_eq!(format!("{}", cycle1), "(abc)");
    }

    #[test]
    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))] // Requires NCOLORS >= 5 (uses color 4)
    fn test_contains_sequence() {
        let colors = vec![Color::new(0), Color::new(2), Color::new(4)];
        let cycle = Cycle::new(&colors);

        assert!(cycle.contains_sequence(Color::new(0), Color::new(2)));
        assert!(cycle.contains_sequence(Color::new(2), Color::new(4)));
        assert!(cycle.contains_sequence(Color::new(4), Color::new(0))); // Wraps around

        assert!(!cycle.contains_sequence(Color::new(0), Color::new(4)));
        assert!(!cycle.contains_sequence(Color::new(2), Color::new(0)));
    }

    #[test]
    #[cfg(not(any(feature = "ncolors_3")))] // Requires NCOLORS >= 4
    fn test_contains_triple() {
        let colors = vec![Color::new(0), Color::new(1), Color::new(2), Color::new(3)];
        let cycle = Cycle::new(&colors);

        assert!(cycle.contains_triple(Color::new(0), Color::new(1), Color::new(2)));
        assert!(cycle.contains_triple(Color::new(1), Color::new(2), Color::new(3)));
        assert!(cycle.contains_triple(Color::new(3), Color::new(0), Color::new(1))); // Wraps

        assert!(!cycle.contains_triple(Color::new(0), Color::new(2), Color::new(3)));
    }

    #[test]
    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))] // Requires NCOLORS >= 5
    fn test_index_of() {
        let colors = vec![Color::new(0), Color::new(2), Color::new(4)];
        let cycle = Cycle::new(&colors);

        assert_eq!(cycle.index_of(Color::new(0)), Some(0));
        assert_eq!(cycle.index_of(Color::new(2)), Some(1));
        assert_eq!(cycle.index_of(Color::new(4)), Some(2));
        assert_eq!(cycle.index_of(Color::new(1)), None);
    }

    #[test]
    fn test_reverse() {
        let colors = vec![Color::new(0), Color::new(1), Color::new(2)];
        let cycle = Cycle::new(&colors);
        let reversed = cycle.reverse();

        // (abc) reversed is (acb) - keep first, reverse rest
        assert_eq!(format!("{}", reversed), "(acb)");

        // Reverse twice should give original
        let double_reversed = reversed.reverse();
        assert_eq!(cycle, double_reversed);
    }

    #[test]
    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))] // Requires NCOLORS >= 5
    fn test_colorset() {
        let colors = vec![Color::new(0), Color::new(2), Color::new(4)];
        let cycle = Cycle::new(&colors);
        let colorset = cycle.colorset();

        assert!(colorset.contains(Color::new(0)));
        assert!(!colorset.contains(Color::new(1)));
        assert!(colorset.contains(Color::new(2)));
        assert!(!colorset.contains(Color::new(3)));
        assert!(colorset.contains(Color::new(4)));
    }

    #[test]
    fn test_display() {
        let colors = vec![Color::new(0), Color::new(1), Color::new(2)];
        let cycle = Cycle::new(&colors);
        assert_eq!(format!("{}", cycle), "(abc)");
    }

    #[test]
    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5")))] // Requires NCOLORS = 6
    fn test_display_with_gaps() {
        let colors = vec![Color::new(1), Color::new(3), Color::new(5)];
        let cycle = Cycle::new(&colors);
        assert_eq!(format!("{}", cycle), "(bdf)");
    }

    #[test]
    #[should_panic(expected = "Cycle must have")]
    fn test_cycle_too_short() {
        let colors = vec![Color::new(0), Color::new(1)];
        Cycle::new(&colors);
    }

    #[test]
    fn test_cycle_different_lengths() {
        // Test cycles of length 3, 4, 5, 6
        for len in 3..=NCOLORS {
            let colors: Vec<_> = (0..len as u8).map(Color::new).collect();
            let cycle = Cycle::new(&colors);
            assert_eq!(cycle.len(), len);
        }
    }
}
