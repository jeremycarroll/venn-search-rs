// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Color type for edge labels.
//!
//! In Venn diagrams, each curve has a unique color (label).
//! Colors are represented as integers 0..NCOLORS-1, with char representations 'a'..'f'.
//!
//! # Examples
//!
//! ```
//! use venn_search::geometry::{Color, NCOLORS};
//!
//! // Create colors by index
//! let red = Color::new(0);
//! assert_eq!(red.to_char(), 'a');
//!
//! // Parse from char
//! let blue = Color::from_char('b').unwrap();
//! assert_eq!(blue.value(), 1);
//!
//! // Iterate over all colors
//! let colors: Vec<char> = Color::all().map(|c| c.to_char()).collect();
//! assert_eq!(colors.len(), NCOLORS);
//! ```

use crate::geometry::constants::NCOLORS;
use std::fmt;

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

    /// Convert color to its character representation ('a', 'b', 'c', ...).
    ///
    /// Color 0 → 'a', Color 1 → 'b', etc.
    ///
    /// # Example
    /// ```
    /// use venn_search::geometry::Color;
    /// assert_eq!(Color::new(0).to_char(), 'a');
    /// assert_eq!(Color::new(5).to_char(), 'f');
    /// ```
    pub fn to_char(self) -> char {
        (b'a' + self.0) as char
    }

    /// Parse a color from its character representation.
    ///
    /// Accepts 'a'..'f' (or up to NCOLORS).
    ///
    /// # Example
    /// ```
    /// use venn_search::geometry::Color;
    /// assert_eq!(Color::from_char('a'), Some(Color::new(0)));
    /// assert_eq!(Color::from_char('c'), Some(Color::new(2)));
    /// assert_eq!(Color::from_char('z'), None);
    /// ```
    pub fn from_char(c: char) -> Option<Self> {
        if c >= 'a' && c < (b'a' + NCOLORS as u8) as char {
            Some(Self((c as u8) - b'a'))
        } else {
            None
        }
    }

    /// Iterator over all valid colors (0..NCOLORS).
    ///
    /// # Example
    /// ```
    /// use venn_search::geometry::{Color, NCOLORS};
    /// let colors: Vec<_> = Color::all().collect();
    /// assert_eq!(colors.len(), NCOLORS);
    /// assert_eq!(colors[0].value(), 0);
    /// ```
    pub fn all() -> impl Iterator<Item = Color> {
        (0..NCOLORS as u8).map(Color)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

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

    #[test]
    fn test_to_char() {
        assert_eq!(Color::new(0).to_char(), 'a');
        assert_eq!(Color::new(1).to_char(), 'b');
        assert_eq!(Color::new(2).to_char(), 'c');

        // Test based on NCOLORS
        match NCOLORS {
            3 => {
                assert_eq!(Color::new(2).to_char(), 'c');
            }
            4 => {
                assert_eq!(Color::new(3).to_char(), 'd');
            }
            5 => {
                assert_eq!(Color::new(4).to_char(), 'e');
            }
            6 => {
                assert_eq!(Color::new(5).to_char(), 'f');
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_from_char() {
        assert_eq!(Color::from_char('a'), Some(Color::new(0)));
        assert_eq!(Color::from_char('b'), Some(Color::new(1)));
        assert_eq!(Color::from_char('c'), Some(Color::new(2)));

        // Invalid chars
        assert_eq!(Color::from_char('z'), None);
        assert_eq!(Color::from_char('A'), None);
        assert_eq!(Color::from_char('0'), None);

        // Test boundary based on NCOLORS
        match NCOLORS {
            3 => {
                assert!(Color::from_char('c').is_some());
                assert!(Color::from_char('d').is_none());
            }
            4 => {
                assert!(Color::from_char('d').is_some());
                assert!(Color::from_char('e').is_none());
            }
            5 => {
                assert!(Color::from_char('e').is_some());
                assert!(Color::from_char('f').is_none());
            }
            6 => {
                assert!(Color::from_char('f').is_some());
                assert!(Color::from_char('g').is_none());
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_from_char_to_char_round_trip() {
        for i in 0..NCOLORS as u8 {
            let color = Color::new(i);
            let ch = color.to_char();
            assert_eq!(Color::from_char(ch), Some(color));
        }
    }

    #[test]
    fn test_all_iterator() {
        let colors: Vec<_> = Color::all().collect();
        assert_eq!(colors.len(), NCOLORS);

        for (i, color) in colors.iter().enumerate() {
            assert_eq!(color.value(), i as u8);
        }
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Color::new(0)), "a");
        assert_eq!(format!("{}", Color::new(1)), "b");
        assert_eq!(format!("{}", Color::new(2)), "c");
    }

    #[test]
    fn test_ordering() {
        assert!(Color::new(0) < Color::new(1));
        assert!(Color::new(1) < Color::new(2));
        assert!(Color::new(2) > Color::new(0));
    }
}
