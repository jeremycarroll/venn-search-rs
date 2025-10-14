// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Color type for edge labels.
//!
//! In Venn diagrams, each curve has a unique color (label).
//! For 6-Venn diagrams, colors are 0..5 corresponding to curves a, b, c, d, e, f.
//!
//! This module will be fully implemented during Phase 2.

/// Number of colors (curves) in the Venn diagram.
///
/// For now this is a constant, but could become a const generic in the future.
pub const NCOLORS: usize = 6;

/// A color (edge label) in the range 0..NCOLORS.
///
/// This is a newtype wrapper to provide type safety and prevent mixing
/// colors with other integer values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Color(u8);

impl Color {
    /// Create a new color, panicking if out of range.
    ///
    /// # Panics
    ///
    /// Panics if `value >= NCOLORS`.
    pub fn new(value: u8) -> Self {
        assert!((value as usize) < NCOLORS, "Color out of range: {}", value);
        Self(value)
    }

    /// Try to create a new color, returning None if out of range.
    pub fn try_new(value: u8) -> Option<Self> {
        if (value as usize) < NCOLORS {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Get the underlying value.
    pub fn value(self) -> u8 {
        self.0
    }

    /// Get the color as a usize (for array indexing).
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

// More functionality to be added during Phase 2:
// - ColorSet (bitset of colors representing faces)
// - Iterator over all colors
// - Conversion to/from char ('a'..'f')
// - etc.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_new() {
        let c = Color::new(0);
        assert_eq!(c.value(), 0);

        let c = Color::new(5);
        assert_eq!(c.value(), 5);
    }

    #[test]
    #[should_panic(expected = "Color out of range")]
    fn test_color_out_of_range() {
        Color::new(6);
    }

    #[test]
    fn test_color_try_new() {
        assert!(Color::try_new(0).is_some());
        assert!(Color::try_new(5).is_some());
        assert!(Color::try_new(6).is_none());
    }

    #[test]
    fn test_color_as_usize() {
        let c = Color::new(3);
        assert_eq!(c.as_usize(), 3);
    }
}
