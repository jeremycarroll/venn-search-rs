// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! ColorSet type for representing sets of colors as bitsets.
//!
//! A ColorSet is a compact representation of a set of colors using a bitset,
//! where bit i represents the presence of color i.
//!
//! # Examples
//!
//! ```
//! use venn_search::geometry::{Color, ColorSet};
//!
//! // Create a color set
//! let mut set = ColorSet::empty();
//! set.insert(Color::new(0));  // 'a'
//! set.insert(Color::new(1));  // 'b'
//! set.insert(Color::new(2));  // 'c'
//!
//! assert_eq!(set.len(), 3);
//! assert_eq!(format!("{}", set), "|abc|");
//!
//! // Iterate over colors in the set
//! let colors: Vec<char> = set.iter().map(|c| c.to_char()).collect();
//! assert_eq!(colors, vec!['a', 'b', 'c']);
//! ```

use crate::geometry::{constants::NCOLORS, Color};
use std::fmt;

/// A set of colors represented as a bitset.
///
/// Bit i (counting from LSB) is set if color i is in the set.
/// This provides O(1) insert, remove, and contains operations.
///
/// Uses u64 for compatibility with the trail system (which only supports u64 values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorSet(u64);

impl ColorSet {
    /// Create an empty color set.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create a color set containing all valid colors (0..NCOLORS).
    pub fn full() -> Self {
        Self((1 << NCOLORS) - 1)
    }

    /// Create a color set from a slice of colors.
    pub fn from_colors(colors: &[Color]) -> Self {
        let mut set = Self::empty();
        for &color in colors {
            set.insert(color);
        }
        set
    }

    /// Create a color set from a raw bit value.
    ///
    /// # Arguments
    ///
    /// * `bits` - The raw bitset value (u64)
    ///
    /// # Note
    ///
    /// This is useful when converting face IDs (which are bitmasks) to ColorSets.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Check if the set contains a specific color.
    pub fn contains(self, color: Color) -> bool {
        (self.0 >> color.value()) & 1 != 0
    }

    /// Insert a color into the set.
    pub fn insert(&mut self, color: Color) {
        self.0 |= 1 << color.value();
    }

    /// Remove a color from the set.
    pub fn remove(&mut self, color: Color) {
        self.0 &= !(1 << color.value());
    }

    /// Get the number of colors in the set (population count).
    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Check if the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Get the underlying bitset value (u64 for trail compatibility).
    pub fn bits(self) -> u64 {
        self.0
    }

    /// Iterate over all colors in the set.
    ///
    /// Colors are yielded in ascending order (0, 1, 2, ...).
    pub fn iter(self) -> impl Iterator<Item = Color> {
        ColorSetIter {
            bits: self.0,
            index: 0,
        }
    }
}

/// Iterator over colors in a ColorSet.
struct ColorSetIter {
    bits: u64,
    index: u8,
}

impl Iterator for ColorSetIter {
    type Item = Color;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < NCOLORS as u8 {
            let idx = self.index;
            self.index += 1;

            if (self.bits >> idx) & 1 != 0 {
                return Some(Color::new(idx));
            }
        }
        None
    }
}

impl fmt::Display for ColorSet {
    /// Format a color set as "|abc|".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "|")?;
        for color in self.iter() {
            write!(f, "{}", color.to_char())?;
        }
        write!(f, "|")
    }
}

impl From<&[Color]> for ColorSet {
    fn from(colors: &[Color]) -> Self {
        Self::from_colors(colors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let set = ColorSet::empty();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.bits(), 0);
    }

    #[test]
    fn test_full() {
        let set = ColorSet::full();
        assert!(!set.is_empty());
        assert_eq!(set.len(), NCOLORS);

        for i in 0..NCOLORS as u8 {
            assert!(set.contains(Color::new(i)));
        }
    }

    #[test]
    fn test_insert_contains() {
        let mut set = ColorSet::empty();
        assert!(!set.contains(Color::new(0)));

        set.insert(Color::new(0));
        assert!(set.contains(Color::new(0)));
        assert_eq!(set.len(), 1);

        set.insert(Color::new(2));
        assert!(set.contains(Color::new(0)));
        assert!(set.contains(Color::new(2)));
        assert!(!set.contains(Color::new(1)));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut set = ColorSet::full();
        assert_eq!(set.len(), NCOLORS);

        set.remove(Color::new(0));
        assert!(!set.contains(Color::new(0)));
        assert_eq!(set.len(), NCOLORS - 1);

        set.remove(Color::new(0)); // Remove again - should be idempotent
        assert_eq!(set.len(), NCOLORS - 1);
    }

    #[test]
    fn test_from_colors() {
        // Use colors valid for all NCOLORS >= 3
        let colors = vec![Color::new(0), Color::new(1), Color::new(2)];
        let set = ColorSet::from_colors(&colors);

        assert!(set.contains(Color::new(0)));
        assert!(set.contains(Color::new(1)));
        assert!(set.contains(Color::new(2)));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_iter() {
        let mut set = ColorSet::empty();
        set.insert(Color::new(0));
        set.insert(Color::new(1));
        set.insert(Color::new(2));

        let colors: Vec<_> = set.iter().collect();
        assert_eq!(colors.len(), 3);
        assert_eq!(colors[0], Color::new(0));
        assert_eq!(colors[1], Color::new(1));
        assert_eq!(colors[2], Color::new(2));
    }

    #[test]
    fn test_display() {
        let mut set = ColorSet::empty();
        assert_eq!(format!("{}", set), "||");

        set.insert(Color::new(0));
        set.insert(Color::new(1));
        set.insert(Color::new(2));
        assert_eq!(format!("{}", set), "|abc|");
    }

    #[test]
    fn test_display_full_set() {
        let set = ColorSet::full();
        let display = format!("{}", set);

        match NCOLORS {
            3 => assert_eq!(display, "|abc|"),
            4 => assert_eq!(display, "|abcd|"),
            5 => assert_eq!(display, "|abcde|"),
            6 => assert_eq!(display, "|abcdef|"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_equality() {
        let set1 = ColorSet::from_colors(&[Color::new(0), Color::new(2)]);
        let set2 = ColorSet::from_colors(&[Color::new(2), Color::new(0)]);
        assert_eq!(set1, set2);

        let set3 = ColorSet::from_colors(&[Color::new(0), Color::new(1)]);
        assert_ne!(set1, set3);
    }

    #[test]
    fn test_from_slice() {
        // Use colors valid for all NCOLORS >= 3
        let colors = [Color::new(0), Color::new(1), Color::new(2)];
        let set: ColorSet = (&colors[..]).into();

        assert!(set.contains(Color::new(0)));
        assert!(set.contains(Color::new(1)));
        assert!(set.contains(Color::new(2)));
        assert_eq!(set.len(), 3);
    }
}
