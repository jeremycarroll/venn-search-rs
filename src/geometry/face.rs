// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Face type for representing regions in Venn diagrams.
//!
//! A face is a region enclosed by edges of different colors. Each face is
//! defined by a set of colors (representing which curves bound it) and has
//! a facial cycle describing the order of colors around its boundary.
//!
//! # Phase 2 Skeleton
//!
//! This is a skeleton implementation providing the basic type structure.
//! Full initialization with edge arrays and cycle constraints will be
//! implemented in Phase 3 as part of the Initialize predicate.

use crate::geometry::{Color, ColorSet, CycleId, CycleSet};

/// Unique identifier for a face.
///
/// For NCOLORS=6, there are 64 faces (2^6 = NFACES).
/// Face IDs correspond to subsets of colors represented as bitmasks.
pub type FaceId = usize;

/// A face (region) in the Venn diagram.
///
/// Each face is defined by its color set (which curves bound it) and has
/// an associated facial cycle describing the cyclic order of colors around
/// its boundary.
///
/// # Facial Cycles
///
/// A face's facial cycle must be one of the possible cycles that:
/// - Contains exactly the colors in the face's color set
/// - Has length >= 3 (faces must be bounded by at least 3 curves)
/// - Is valid according to the diagram's constraints
///
/// The search algorithm narrows down the possible cycles for each face
/// until each face has exactly one cycle assigned.
#[derive(Debug, Clone)]
pub struct Face {
    /// Unique identifier for this face (corresponds to its color set as a bitmask).
    pub id: FaceId,

    /// The set of colors that bound this face.
    ///
    /// For example, face {a,b,c} is bounded by curves a, b, and c.
    pub colors: ColorSet,

    /// Number of possible cycles for this face.
    ///
    /// Starts as the count of all cycles with this face's color set.
    /// Gets reduced during search until it reaches 1 (unique cycle assigned).
    pub cycle_count: usize,

    /// Set of possible cycles for this face.
    ///
    /// Starts with all cycles matching this face's color set.
    /// Gets filtered during search based on constraints.
    pub possible_cycles: CycleSet,
}

impl Face {
    /// Create a new face (used during initialization).
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier (typically the face's color set as a bitmask)
    /// * `colors` - Set of colors bounding this face
    /// * `possible_cycles` - Initial set of possible facial cycles
    ///
    /// # Note
    ///
    /// During initialization, all cycles with the same color set as the face
    /// are added to `possible_cycles`. The search algorithm then narrows this down.
    pub fn new(id: FaceId, colors: ColorSet, possible_cycles: CycleSet) -> Self {
        let cycle_count = possible_cycles.len();

        Self {
            id,
            colors,
            cycle_count,
            possible_cycles,
        }
    }

    /// Get the number of colors bounding this face.
    pub fn num_colors(&self) -> usize {
        self.colors.len()
    }

    /// Check if a color bounds this face.
    pub fn has_color(&self, color: Color) -> bool {
        self.colors.contains(color)
    }

    /// Check if this face has a unique cycle assigned.
    ///
    /// Returns true if exactly one possible cycle remains.
    pub fn has_unique_cycle(&self) -> bool {
        self.cycle_count == 1
    }

    /// Get the assigned cycle ID if unique.
    ///
    /// Returns Some(cycle_id) if exactly one cycle remains, None otherwise.
    pub fn unique_cycle(&self) -> Option<CycleId> {
        if self.cycle_count == 1 {
            self.possible_cycles.iter().next()
        } else {
            None
        }
    }

    /// Check if a cycle is possible for this face.
    pub fn is_cycle_possible(&self, cycle_id: CycleId) -> bool {
        self.possible_cycles.contains(cycle_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face_creation() {
        let colors = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let mut possible = CycleSet::empty();
        possible.insert(0);
        possible.insert(1);

        let face = Face::new(7, colors, possible);

        assert_eq!(face.id, 7);
        assert_eq!(face.colors, colors);
        assert_eq!(face.cycle_count, 2);
        assert!(!face.has_unique_cycle());
    }

    #[test]
    fn test_num_colors() {
        let colors = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let face = Face::new(0, colors, CycleSet::empty());

        assert_eq!(face.num_colors(), 3);
    }

    #[test]
    fn test_has_color() {
        let colors = ColorSet::from_colors(&[Color::new(1), Color::new(2)]);
        let face = Face::new(0, colors, CycleSet::empty());

        assert!(!face.has_color(Color::new(0)));
        assert!(face.has_color(Color::new(1)));
        assert!(face.has_color(Color::new(2)));
    }

    #[test]
    #[cfg(ncolors_min_4)]
    fn test_unique_cycle() {
        let colors = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);

        // Face with multiple possible cycles
        let mut possible_multi = CycleSet::empty();
        possible_multi.insert(5);
        possible_multi.insert(10);
        let face_multi = Face::new(0, colors, possible_multi);

        assert!(!face_multi.has_unique_cycle());
        assert_eq!(face_multi.unique_cycle(), None);

        // Face with unique cycle
        let mut possible_single = CycleSet::empty();
        possible_single.insert(7);
        let face_single = Face::new(0, colors, possible_single);

        assert!(face_single.has_unique_cycle());
        assert_eq!(face_single.unique_cycle(), Some(7));
    }

    #[test]
    #[cfg(ncolors_min_4)]
    fn test_is_cycle_possible() {
        let colors = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let mut possible = CycleSet::empty();
        possible.insert(3);
        possible.insert(7);

        let face = Face::new(0, colors, possible);

        assert!(face.is_cycle_possible(3));
        assert!(!face.is_cycle_possible(5));
        assert!(face.is_cycle_possible(7));
    }

    #[test]
    fn test_empty_possible_cycles() {
        let colors = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let face = Face::new(0, colors, CycleSet::empty());

        assert_eq!(face.cycle_count, 0);
        assert!(!face.has_unique_cycle());
        assert_eq!(face.unique_cycle(), None);
    }
}
