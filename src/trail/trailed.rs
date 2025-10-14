// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Trailed values that automatically record changes for backtracking.

use super::Trail;
use std::marker::PhantomData;

/// A value that automatically records changes to the trail.
///
/// `Trailed<T>` wraps a value and ensures that any modification is recorded
/// in the trail for later backtracking. This provides type-safe tracked state.
///
/// # Type Requirements
///
/// `T` must implement `Copy` because:
/// - Old values are stored as raw u64 in the trail
/// - We need bitwise conversion for trail storage
/// - Only primitive types and small Copy types are supported
///
/// # Memory Model
///
/// - DYNAMIC: Trailed values are mutable state (Tier 2) that changes during search
/// - Each value has a unique ID for trail tracking
///
/// # Example
///
/// ```ignore
/// let mut trail = Trail::new();
/// let mut registry = TrailedRegistry::new();
/// let mut value = registry.register(&mut trail, 42u32);
///
/// trail.checkpoint();
/// value.set(&mut trail, 100);
/// assert_eq!(value.get(), 100);
///
/// trail.rewind();
/// assert_eq!(value.get(), 42); // Value restored
/// ```
#[derive(Debug)]
pub struct Trailed<T: Copy> {
    /// The current value
    value: T,
    /// Unique identifier for trail tracking
    id: usize,
    /// Phantom data to ensure proper variance
    _phantom: PhantomData<T>,
}

impl<T: Copy> Trailed<T> {
    /// Create a new trailed value with the given ID.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `id` is unique within the context.
    /// Use `TrailedRegistry` to safely manage IDs.
    pub(crate) fn new(id: usize, initial_value: T) -> Self {
        Self {
            value: initial_value,
            id,
            _phantom: PhantomData,
        }
    }

    /// Get the current value.
    pub fn get(&self) -> T {
        self.value
    }

    /// Get the unique ID for this trailed value.
    #[allow(dead_code)]
    pub(crate) fn id(&self) -> usize {
        self.id
    }
}

// Specialized implementations for types that fit in u64

impl Trailed<u8> {
    /// Set the value, recording the old value in the trail.
    pub fn set(&mut self, trail: &mut Trail, new_value: u8) {
        let old_value = self.value as u64;
        trail.record_change(self.id, old_value);
        self.value = new_value;
    }
}

impl Trailed<u16> {
    /// Set the value, recording the old value in the trail.
    pub fn set(&mut self, trail: &mut Trail, new_value: u16) {
        let old_value = self.value as u64;
        trail.record_change(self.id, old_value);
        self.value = new_value;
    }
}

impl Trailed<u32> {
    /// Set the value, recording the old value in the trail.
    pub fn set(&mut self, trail: &mut Trail, new_value: u32) {
        let old_value = self.value as u64;
        trail.record_change(self.id, old_value);
        self.value = new_value;
    }
}

impl Trailed<u64> {
    /// Set the value, recording the old value in the trail.
    pub fn set(&mut self, trail: &mut Trail, new_value: u64) {
        let old_value = self.value;
        trail.record_change(self.id, old_value);
        self.value = new_value;
    }

    /// Set the value only if it differs from current value.
    ///
    /// Returns true if the value was changed, false if it was already correct.
    /// This corresponds to `trailMaybeSetInt` in the C implementation.
    pub fn maybe_set(&mut self, trail: &mut Trail, new_value: u64) -> bool {
        if self.value != new_value {
            self.set(trail, new_value);
            true
        } else {
            false
        }
    }
}

impl Trailed<bool> {
    /// Set the value, recording the old value in the trail.
    pub fn set(&mut self, trail: &mut Trail, new_value: bool) {
        let old_value = self.value as u64;
        trail.record_change(self.id, old_value);
        self.value = new_value;
    }
}

impl Trailed<usize> {
    /// Set the value, recording the old value in the trail.
    pub fn set(&mut self, trail: &mut Trail, new_value: usize) {
        let old_value = self.value as u64;
        trail.record_change(self.id, old_value);
        self.value = new_value;
    }
}

/// Registry for managing unique IDs for Trailed values.
///
/// This ensures that each Trailed value gets a unique ID for trail tracking.
#[derive(Debug)]
pub struct TrailedRegistry {
    next_id: usize,
}

impl TrailedRegistry {
    /// Create a new registry starting at ID 0.
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    /// Register a new trailed value with an initial value.
    ///
    /// Returns a Trailed<T> with a unique ID.
    pub fn register<T: Copy>(&mut self, initial_value: T) -> Trailed<T> {
        let id = self.next_id;
        self.next_id += 1;
        Trailed::new(id, initial_value)
    }

    /// Get the next available ID (useful for debugging).
    pub fn next_id(&self) -> usize {
        self.next_id
    }
}

impl Default for TrailedRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for restoring trailed values from the trail.
///
/// This is used internally by the trail system to restore values during backtracking.
pub trait TrailRestore {
    /// Restore a value from a u64 stored in the trail.
    fn restore(&mut self, old_value: u64);
}

impl TrailRestore for Trailed<u8> {
    fn restore(&mut self, old_value: u64) {
        self.value = old_value as u8;
    }
}

impl TrailRestore for Trailed<u16> {
    fn restore(&mut self, old_value: u64) {
        self.value = old_value as u16;
    }
}

impl TrailRestore for Trailed<u32> {
    fn restore(&mut self, old_value: u64) {
        self.value = old_value as u32;
    }
}

impl TrailRestore for Trailed<u64> {
    fn restore(&mut self, old_value: u64) {
        self.value = old_value;
    }
}

impl TrailRestore for Trailed<bool> {
    fn restore(&mut self, old_value: u64) {
        self.value = old_value != 0;
    }
}

impl TrailRestore for Trailed<usize> {
    fn restore(&mut self, old_value: u64) {
        self.value = old_value as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trailed_u64_basic() {
        let mut trail = Trail::new();
        let mut registry = TrailedRegistry::new();
        let mut value = registry.register(42u64);

        assert_eq!(value.get(), 42);
        assert_eq!(value.id(), 0);

        value.set(&mut trail, 100);
        assert_eq!(value.get(), 100);
        assert_eq!(trail.len(), 1);
    }

    #[test]
    fn test_trailed_maybe_set() {
        let mut trail = Trail::new();
        let mut registry = TrailedRegistry::new();
        let mut value = registry.register(42u64);

        // Setting same value doesn't record in trail
        assert!(!value.maybe_set(&mut trail, 42));
        assert_eq!(trail.len(), 0);

        // Setting different value records in trail
        assert!(value.maybe_set(&mut trail, 100));
        assert_eq!(trail.len(), 1);
        assert_eq!(value.get(), 100);
    }

    #[test]
    fn test_trailed_different_types() {
        let mut registry = TrailedRegistry::new();

        let v1 = registry.register(42u8);
        let v2 = registry.register(100u16);
        let v3 = registry.register(1000u32);
        let v4 = registry.register(10000u64);
        let v5 = registry.register(true);

        assert_eq!(v1.id(), 0);
        assert_eq!(v2.id(), 1);
        assert_eq!(v3.id(), 2);
        assert_eq!(v4.id(), 3);
        assert_eq!(v5.id(), 4);
    }

    #[test]
    fn test_trail_restore_u64() {
        let mut value = Trailed::new(0, 100u64);
        assert_eq!(value.get(), 100);

        value.restore(42);
        assert_eq!(value.get(), 42);
    }

    #[test]
    fn test_trail_restore_bool() {
        let mut value = Trailed::new(0, true);
        assert_eq!(value.get(), true);

        value.restore(0);
        assert_eq!(value.get(), false);

        value.restore(1);
        assert_eq!(value.get(), true);
    }

    #[test]
    fn test_registry_unique_ids() {
        let mut registry = TrailedRegistry::new();

        let v1 = registry.register(1u64);
        let v2 = registry.register(2u64);
        let v3 = registry.register(3u64);

        assert_eq!(v1.id(), 0);
        assert_eq!(v2.id(), 1);
        assert_eq!(v3.id(), 2);
        assert_eq!(registry.next_id(), 3);
    }
}
