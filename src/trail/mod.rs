// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Trail-based backtracking system for the Venn search engine.
//!
//! This module provides O(1) backtracking by recording state changes in a trail.
//! When backtracking occurs, the trail is rewound to restore previous state.
//!
//! The implementation is based on the C trail system in c-reference/engine.c (lines 21-227).

pub mod trailed;

pub use trailed::{Trailed, TrailedRegistry};

/// A single entry in the trail, recording one state change.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Fields will be used when we implement automatic restoration
struct TrailEntry {
    /// Unique identifier for the value being tracked
    id: usize,
    /// The old value before the change (stored as u64)
    old_value: u64,
}

/// The trail system for O(1) backtracking.
///
/// The trail records all state changes so they can be efficiently undone during backtracking.
/// This is the foundation of the non-deterministic search engine.
///
/// # Memory Model
///
/// - DYNAMIC: The trail is mutable state (Tier 2) that changes during search
/// - Each SearchContext owns its own trail for independent parallel searches
///
/// # Implementation Notes
///
/// The C implementation uses a global static array with pointer arithmetic.
/// This Rust implementation uses a Vec with index-based operations for safety.
#[derive(Debug)]
pub struct Trail {
    /// All trail entries recorded so far
    entries: Vec<TrailEntry>,
    /// Stack of checkpoint indices for nested backtracking
    checkpoints: Vec<usize>,
    /// Optional frozen checkpoint that prevents further backtracking
    frozen_checkpoint: Option<usize>,
}

impl Trail {
    /// Maximum trail size (matches C TRAIL_SIZE = 16384)
    const MAX_SIZE: usize = 16384;

    /// Create a new empty trail.
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(Self::MAX_SIZE),
            checkpoints: Vec::with_capacity(64), // Reasonable depth estimate
            frozen_checkpoint: None,
        }
    }

    /// Record a checkpoint for later backtracking.
    ///
    /// Returns the checkpoint index.
    pub fn checkpoint(&mut self) -> usize {
        let checkpoint = self.entries.len();
        self.checkpoints.push(checkpoint);
        checkpoint
    }

    /// Rewind the trail to the most recent checkpoint.
    ///
    /// Returns true if there was a checkpoint to rewind to, false otherwise.
    pub fn rewind(&mut self) -> bool {
        if let Some(checkpoint_idx) = self.checkpoints.pop() {
            // Don't rewind past frozen checkpoint
            if let Some(frozen) = self.frozen_checkpoint {
                if checkpoint_idx < frozen {
                    self.checkpoints.push(checkpoint_idx);
                    return false;
                }
            }

            // Truncate to checkpoint (entries beyond checkpoint are discarded)
            self.entries.truncate(checkpoint_idx);
            true
        } else {
            false
        }
    }

    /// Freeze the trail at the current position.
    ///
    /// After freezing, no backtracking past this point is allowed.
    /// This corresponds to `trailFreeze()` in the C implementation.
    pub fn freeze(&mut self) {
        self.frozen_checkpoint = Some(self.entries.len());
    }

    /// Record a state change in the trail (internal use only).
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the value being changed
    /// * `old_value` - The value before the change (as u64)
    ///
    /// # Panics
    ///
    /// Panics if the trail exceeds MAX_SIZE (indicates a bug in the search algorithm).
    pub(crate) fn record_change(&mut self, id: usize, old_value: u64) {
        if self.entries.len() >= Self::MAX_SIZE {
            panic!("Trail overflow: exceeded {} entries", Self::MAX_SIZE);
        }

        self.entries.push(TrailEntry { id, old_value });
    }

    /// Get an iterator over trail entries since the last checkpoint.
    ///
    /// This is useful for debugging and testing.
    #[allow(dead_code)]
    #[allow(private_interfaces)]
    pub(crate) fn entries_since_checkpoint(&self) -> impl Iterator<Item = &TrailEntry> {
        let start = self.checkpoints.last().copied().unwrap_or(0);
        self.entries[start..].iter()
    }

    /// Get the current number of entries in the trail.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the trail is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of active checkpoints.
    pub fn checkpoint_depth(&self) -> usize {
        self.checkpoints.len()
    }
}

impl Default for Trail {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trail_new() {
        let trail = Trail::new();
        assert_eq!(trail.len(), 0);
        assert_eq!(trail.checkpoint_depth(), 0);
        assert!(trail.is_empty());
    }

    #[test]
    fn test_checkpoint_and_rewind() {
        let mut trail = Trail::new();

        // Record some changes
        trail.record_change(1, 10);
        trail.record_change(2, 20);
        assert_eq!(trail.len(), 2);

        // Create checkpoint
        let checkpoint = trail.checkpoint();
        assert_eq!(checkpoint, 2);
        assert_eq!(trail.checkpoint_depth(), 1);

        // Record more changes
        trail.record_change(3, 30);
        trail.record_change(4, 40);
        assert_eq!(trail.len(), 4);

        // Rewind to checkpoint
        assert!(trail.rewind());
        assert_eq!(trail.len(), 2);
        assert_eq!(trail.checkpoint_depth(), 0);
    }

    #[test]
    fn test_nested_checkpoints() {
        let mut trail = Trail::new();

        trail.record_change(1, 10);
        let _cp1 = trail.checkpoint();

        trail.record_change(2, 20);
        let _cp2 = trail.checkpoint();

        trail.record_change(3, 30);
        assert_eq!(trail.len(), 3);
        assert_eq!(trail.checkpoint_depth(), 2);

        // Rewind inner checkpoint
        assert!(trail.rewind());
        assert_eq!(trail.len(), 2);
        assert_eq!(trail.checkpoint_depth(), 1);

        // Rewind outer checkpoint
        assert!(trail.rewind());
        assert_eq!(trail.len(), 1);
        assert_eq!(trail.checkpoint_depth(), 0);
    }

    #[test]
    fn test_rewind_empty() {
        let mut trail = Trail::new();
        assert!(!trail.rewind()); // No checkpoint to rewind
    }

    #[test]
    fn test_freeze() {
        let mut trail = Trail::new();

        trail.record_change(1, 10);
        trail.checkpoint();

        trail.record_change(2, 20);
        trail.freeze(); // Freeze at position 2

        trail.checkpoint();
        trail.record_change(3, 30);

        // Can rewind recent changes
        assert!(trail.rewind());
        assert_eq!(trail.len(), 2);

        // Cannot rewind past freeze point
        assert!(!trail.rewind());
        assert_eq!(trail.len(), 2);
    }

    #[test]
    fn test_entries_since_checkpoint() {
        let mut trail = Trail::new();

        trail.record_change(1, 10);
        trail.checkpoint();
        trail.record_change(2, 20);
        trail.record_change(3, 30);

        let entries: Vec<_> = trail.entries_since_checkpoint().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, 2);
        assert_eq!(entries[1].id, 3);
    }

    #[test]
    #[should_panic(expected = "Trail overflow")]
    fn test_trail_overflow() {
        let mut trail = Trail::new();

        // Try to exceed MAX_SIZE
        for i in 0..Trail::MAX_SIZE + 1 {
            trail.record_change(i, 0);
        }
    }
}
