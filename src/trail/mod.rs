// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Trail-based backtracking system for the Venn search engine.
//!
//! This module provides O(1) backtracking by recording state changes in a trail.
//! When backtracking occurs, the trail is rewound to restore previous state.
//!
//! The implementation closely matches the C trail system in c-reference/engine.c (lines 21-227).
//!
//! # Design
//!
//! - Trail entries are 128 bits: non-null pointer (64-bit) + old value (64-bit)
//! - Only supports `u64` values (no u8/u16/u32 overhead)
//! - Automatic restoration on rewind (walks trail backwards, writes old values)
//! - Pointers must point to data owned by SearchContext (lifetime safety)

use std::ptr::NonNull;

/// A single entry in the trail, recording one state change.
///
/// Structure: 128-bit entry with pointer (64-bit) + value (64-bit)
/// ```ignore
/// struct TrailEntry {
///     ptr: NonNull<u64>  // 8 bytes, non-null pointer
///     old_value: u64     // 8 bytes, previous value
/// }
/// ```
///
/// We use `NonNull<u64>` instead of `*mut u64` to guarantee the pointer is never null,
/// eliminating an entire class of potential bugs.
#[derive(Debug, Clone, Copy)]
struct TrailEntry {
    /// Non-null pointer to the u64 value that was changed
    ptr: NonNull<u64>,
    /// The old value before the change
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
/// Uses a Vec for dynamic storage with automatic restoration on rewind.
///
/// # Safety
///
/// Trail operations are `unsafe` because they store raw pointers. The safety invariant
/// is that all pointers must point to data owned by the SearchContext that owns this trail.
/// This is enforced by making trail operations private and only exposing them through
/// safe wrapper methods on SearchContext.
#[derive(Debug)]
pub struct Trail {
    /// All trail entries recorded so far
    entries: Vec<TrailEntry>,
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
            frozen_checkpoint: None,
        }
    }

    /// Record a checkpoint for later backtracking.
    ///
    /// Returns the checkpoint index. The caller is responsible for storing
    /// this index and passing it to `rewind_to()` when backtracking.
    pub fn checkpoint(&mut self) -> usize {
        self.entries.len()
    }

    /// Rewind the trail to a specific checkpoint index.
    ///
    /// Restores all values that were changed after the checkpoint.
    /// The caller must provide a valid checkpoint index (from a previous `checkpoint()` call).
    ///
    /// # Arguments
    ///
    /// * `target_idx` - The checkpoint index to rewind to
    pub fn rewind_to(&mut self, target_idx: usize) {
        // Don't rewind past frozen checkpoint
        let target = if let Some(frozen) = self.frozen_checkpoint {
            target_idx.max(frozen)
        } else {
            target_idx
        };

        // Restore all values back to target
        while self.entries.len() > target {
            let entry = self.entries.pop().unwrap();
            unsafe {
                // SAFETY: Pointer was valid when recorded, and data is owned by
                // SearchContext which owns this trail, so pointer is still valid.
                // NonNull guarantees it's not null.
                *entry.ptr.as_ptr() = entry.old_value;
            }
        }
    }

    /// Freeze the trail at the current position.
    ///
    /// After freezing, no backtracking past this point is allowed.
    pub fn freeze(&mut self) {
        self.frozen_checkpoint = Some(self.entries.len());
    }

    /// Record a state change and update the value (internal use only).
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - `ptr` points to a valid `u64` owned by the SearchContext that owns this trail
    /// - `ptr` remains valid for the lifetime of the trail
    /// - No other mutable references to `*ptr` exist during this call
    ///
    /// # Arguments
    ///
    /// * `ptr` - Non-null pointer to the u64 value to modify
    /// * `new_value` - The new value to set
    ///
    /// # Panics
    ///
    /// Panics if the trail exceeds MAX_SIZE (design constraint - search is too deep).
    pub(crate) unsafe fn record_and_set(&mut self, ptr: NonNull<u64>, new_value: u64) {
        if self.entries.len() >= Self::MAX_SIZE {
            panic!("Trail overflow: exceeded {} entries", Self::MAX_SIZE);
        }

        // Read old value and record it
        let old_value = *ptr.as_ptr();
        self.entries.push(TrailEntry { ptr, old_value });

        // Write new value
        *ptr.as_ptr() = new_value;
    }

    /// Conditionally record and set a value (like C's trailMaybeSetInt).
    ///
    /// Returns true if the value was changed, false if it was already correct.
    ///
    /// # Safety
    ///
    /// Same safety requirements as `record_and_set`.
    #[allow(dead_code)]
    pub(crate) unsafe fn maybe_record_and_set(
        &mut self,
        ptr: NonNull<u64>,
        new_value: u64,
    ) -> bool {
        if *ptr.as_ptr() != new_value {
            self.record_and_set(ptr, new_value);
            true
        } else {
            false
        }
    }

    /// Get the current number of entries in the trail.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the trail is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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
        assert!(trail.is_empty());
    }

    #[test]
    fn test_record_and_restore() {
        let mut trail = Trail::new();
        let mut value1 = 10u64;
        let mut value2 = 20u64;

        // Record initial state
        let checkpoint = trail.checkpoint();

        // Make changes
        unsafe {
            trail.record_and_set(NonNull::new_unchecked(&mut value1), 100);
            trail.record_and_set(NonNull::new_unchecked(&mut value2), 200);
        }

        assert_eq!(value1, 100);
        assert_eq!(value2, 200);
        assert_eq!(trail.len(), 2);

        // Rewind restores old values
        trail.rewind_to(checkpoint);
        assert_eq!(value1, 10); // Restored!
        assert_eq!(value2, 20); // Restored!
        assert_eq!(trail.len(), 0);
    }

    #[test]
    fn test_nested_checkpoints() {
        let mut trail = Trail::new();
        let mut value = 10u64;

        unsafe {
            trail.record_and_set(NonNull::new_unchecked(&mut value), 20);
        }
        let cp1 = trail.checkpoint();

        unsafe {
            trail.record_and_set(NonNull::new_unchecked(&mut value), 30);
        }
        let cp2 = trail.checkpoint();

        unsafe {
            trail.record_and_set(NonNull::new_unchecked(&mut value), 40);
        }
        assert_eq!(value, 40);
        assert_eq!(trail.len(), 3);

        // Rewind to cp2
        trail.rewind_to(cp2);
        assert_eq!(value, 30); // Restored to cp2
        assert_eq!(trail.len(), 2);

        // Rewind to cp1
        trail.rewind_to(cp1);
        assert_eq!(value, 20); // Restored to cp1
        assert_eq!(trail.len(), 1);
    }

    #[test]
    fn test_freeze() {
        let mut trail = Trail::new();
        let mut value = 10u64;

        unsafe {
            trail.record_and_set(NonNull::new_unchecked(&mut value), 20);
        }
        let cp1 = trail.checkpoint();

        unsafe {
            trail.record_and_set(NonNull::new_unchecked(&mut value), 30);
        }
        trail.freeze(); // Freeze at position 2

        let cp2 = trail.checkpoint();
        unsafe {
            trail.record_and_set(NonNull::new_unchecked(&mut value), 40);
        }

        // Can rewind to cp2 (recent changes)
        trail.rewind_to(cp2);
        assert_eq!(value, 30);
        assert_eq!(trail.len(), 2);

        // Cannot rewind past freeze point
        trail.rewind_to(cp1);
        assert_eq!(value, 30); // Still 30, not 20 (blocked by freeze)
        assert_eq!(trail.len(), 2);
    }

    #[test]
    fn test_maybe_record_and_set() {
        let mut trail = Trail::new();
        let mut value = 42u64;

        let checkpoint = trail.checkpoint();

        // Setting same value doesn't record in trail
        let changed = unsafe { trail.maybe_record_and_set(NonNull::new_unchecked(&mut value), 42) };
        assert!(!changed);
        assert_eq!(trail.len(), 0);

        // Setting different value records in trail
        let changed =
            unsafe { trail.maybe_record_and_set(NonNull::new_unchecked(&mut value), 100) };
        assert!(changed);
        assert_eq!(trail.len(), 1);
        assert_eq!(value, 100);

        // Rewind restores
        trail.rewind_to(checkpoint);
        assert_eq!(value, 42);
    }

    #[test]
    fn test_array_elements() {
        let mut trail = Trail::new();
        let mut array = [0u64, 10, 20, 30, 40];

        let checkpoint = trail.checkpoint();

        // Trail changes to array elements
        unsafe {
            trail.record_and_set(NonNull::new_unchecked(&mut array[1]), 100);
            trail.record_and_set(NonNull::new_unchecked(&mut array[3]), 300);
        }

        assert_eq!(array[1], 100);
        assert_eq!(array[3], 300);

        // Rewind restores array elements
        trail.rewind_to(checkpoint);
        assert_eq!(array[1], 10);
        assert_eq!(array[3], 30);
    }

    #[test]
    #[should_panic(expected = "Trail overflow")]
    fn test_trail_overflow() {
        let mut trail = Trail::new();
        let mut value = 0u64;

        // Try to exceed MAX_SIZE
        for i in 0..Trail::MAX_SIZE + 1 {
            unsafe {
                trail.record_and_set(NonNull::new_unchecked(&mut value), i as u64);
            }
        }
    }
}
