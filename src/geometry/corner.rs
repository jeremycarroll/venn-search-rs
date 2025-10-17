// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Corner detection and crossing count tracking for triangle Venn diagrams.
//!
//! This module implements the corner detection algorithm from Carroll 2000,
//! which determines the minimum number of corners required on each curve to
//! realize the Venn diagram with triangles.
//!
//! # Triangle Constraint
//!
//! For a Venn diagram to be drawable with triangles, each pair of colors
//! can cross at most 6 times. This constraint is enforced during the search
//! by tracking crossing counts as facial cycles are assigned.

use crate::geometry::constants::NCOLORS;
use crate::geometry::{Color, ColorSet};

/// Maximum crossings allowed between any pair of curves (triangle constraint).
///
/// For a 6-Venn diagram drawable with triangles, each pair of curves can
/// cross at most 6 times.
pub const MAX_CROSSINGS_PER_PAIR: usize = 6;

/// Tracking state for corner detection walk around a curve.
///
/// Used to implement the Carroll 2000 corner detection algorithm.
#[derive(Debug, Clone)]
pub struct CornerWalkState {
    /// Colors of curves that are currently outside (we're inside them).
    pub out: ColorSet,

    /// Colors of curves we've recently crossed from inside to outside.
    pub passed: ColorSet,

    /// Vertices where corners must be placed (indices into vertex array).
    pub corner_vertices: Vec<usize>,
}

impl CornerWalkState {
    /// Create a new corner walk state with empty sets.
    pub fn new() -> Self {
        Self {
            out: ColorSet::empty(),
            passed: ColorSet::empty(),
            corner_vertices: Vec::new(),
        }
    }

    /// Process a vertex during the walk around a curve.
    ///
    /// # Arguments
    ///
    /// * `other_color` - The other color crossing at this vertex
    /// * `vertex_id` - ID of the vertex being processed
    ///
    /// # Algorithm
    ///
    /// If other_color is in Out:
    /// - Remove other_color from Out
    /// - If other_color is in Passed: clear Passed and add vertex to results
    /// Otherwise:
    /// - Add other_color to both Out and Passed
    pub fn process_vertex(&mut self, other_color: Color, vertex_id: usize) {
        if self.out.contains(other_color) {
            // Crossing from outside to inside
            self.out.remove(other_color);

            if self.passed.contains(other_color) {
                // Found a corner location
                self.corner_vertices.push(vertex_id);
                self.passed = ColorSet::empty();
            }
        } else {
            // Crossing from inside to outside
            self.out.insert(other_color);
            self.passed.insert(other_color);
        }
    }

    /// Get the number of corners detected on this walk.
    pub fn corner_count(&self) -> usize {
        self.corner_vertices.len()
    }
}

impl Default for CornerWalkState {
    fn default() -> Self {
        Self::new()
    }
}

/// Crossing counts between pairs of colors (curves).
///
/// Tracks how many times each pair of colors crosses in the current solution.
/// This is used to enforce the triangle constraint (max 6 crossings per pair).
///
/// # Memory Layout
///
/// Stored as a symmetric matrix using the upper triangle only (i < j).
/// For NCOLORS=6, this is 15 pairs: (0,1), (0,2), ..., (4,5).
///
/// # Trail Tracking
///
/// All modifications to crossing counts must be trail-tracked for backtracking.
#[derive(Debug, Clone)]
pub struct CrossingCounts {
    /// Crossing counts indexed by color pair (upper triangle only).
    ///
    /// Access via `get(i, j)` and `set(i, j, count)` which enforce i < j.
    /// Stored as [NCOLORS][NCOLORS] for simple indexing, lower triangle unused.
    counts: [[u8; NCOLORS]; NCOLORS],
}

impl CrossingCounts {
    /// Create a new crossing counts structure with all counts at zero.
    pub fn new() -> Self {
        Self {
            counts: [[0u8; NCOLORS]; NCOLORS],
        }
    }

    /// Get the crossing count for a color pair.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if i >= j (must use upper triangle).
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> u8 {
        debug_assert!(
            i < j,
            "CrossingCounts only valid for i < j (upper triangle), got i={}, j={}",
            i,
            j
        );
        self.counts[i][j]
    }

    /// Get a mutable pointer to a crossing count for trail tracking.
    ///
    /// # Safety
    ///
    /// Caller must use trail.record_and_set() to modify the value.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if i >= j (must use upper triangle).
    #[inline]
    pub fn get_mut_ptr(&mut self, i: usize, j: usize) -> *mut u8 {
        debug_assert!(
            i < j,
            "CrossingCounts only valid for i < j (upper triangle), got i={}, j={}",
            i,
            j
        );
        &mut self.counts[i][j] as *mut u8
    }

    /// Check if a crossing count exceeds the maximum allowed.
    #[inline]
    pub fn exceeds_max(&self, i: usize, j: usize) -> bool {
        self.get(i, j) as usize > MAX_CROSSINGS_PER_PAIR
    }

    /// Get all crossing counts for debugging.
    pub fn all_counts(&self) -> Vec<((usize, usize), u8)> {
        let mut result = Vec::new();
        for i in 0..NCOLORS {
            for j in (i + 1)..NCOLORS {
                result.push(((i, j), self.counts[i][j]));
            }
        }
        result
    }
}

impl Default for CrossingCounts {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corner_walk_state_creation() {
        let state = CornerWalkState::new();
        assert!(state.out.is_empty());
        assert!(state.passed.is_empty());
        assert_eq!(state.corner_count(), 0);
    }

    #[test]
    fn test_corner_walk_crossing_out_to_in() {
        let mut state = CornerWalkState::new();
        let color_a = Color::new(0);

        // First crossing: inside to outside
        state.process_vertex(color_a, 10);
        assert!(state.out.contains(color_a));
        assert!(state.passed.contains(color_a));
        assert_eq!(state.corner_count(), 0);

        // Second crossing: outside to inside
        state.process_vertex(color_a, 20);
        assert!(!state.out.contains(color_a));
        assert_eq!(state.corner_count(), 1);
        assert_eq!(state.corner_vertices[0], 20);
        assert!(state.passed.is_empty());
    }

    #[test]
    fn test_crossing_counts_creation() {
        let counts = CrossingCounts::new();

        // All counts should be zero
        for i in 0..NCOLORS {
            for j in (i + 1)..NCOLORS {
                assert_eq!(counts.get(i, j), 0);
            }
        }
    }

    #[test]
    fn test_crossing_counts_get() {
        let counts = CrossingCounts::new();
        assert_eq!(counts.get(0, 1), 0);
        assert_eq!(counts.get(0, 2), 0);

        // Test another pair (valid for all NCOLORS >= 3)
        if NCOLORS >= 3 {
            assert_eq!(counts.get(1, 2), 0);
        }
    }

    #[test]
    #[should_panic(expected = "CrossingCounts only valid for i < j")]
    #[cfg(debug_assertions)]
    fn test_crossing_counts_panics_on_lower_triangle() {
        let counts = CrossingCounts::new();
        let _ = counts.get(3, 1); // i > j - should panic
    }

    #[test]
    #[should_panic(expected = "CrossingCounts only valid for i < j")]
    #[cfg(debug_assertions)]
    fn test_crossing_counts_panics_on_diagonal() {
        let counts = CrossingCounts::new();
        let _ = counts.get(2, 2); // i == j - should panic
    }

    #[test]
    fn test_crossing_counts_all_counts() {
        let counts = CrossingCounts::new();
        let all = counts.all_counts();

        // Should have NCOLORS * (NCOLORS - 1) / 2 pairs
        let expected_pairs = NCOLORS * (NCOLORS - 1) / 2;
        assert_eq!(all.len(), expected_pairs);

        // All should be zero
        for (_, count) in all {
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn test_max_crossings_constant() {
        // Verify the constant is set correctly for triangles
        assert_eq!(MAX_CROSSINGS_PER_PAIR, 6);
    }
}
