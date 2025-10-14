// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! CycleSet type for representing sets of cycles as bitsets.
//!
//! A CycleSet is a compact representation of a set of cycle IDs using a bitset,
//! where bit i represents the presence of cycle i. Since NCYCLES can be up to 394
//! (for NCOLORS=6), we need multiple u64 words to represent all possible cycles.
//!
//! # Examples
//!
//! ```
//! use venn_search::geometry::{CycleSet, CycleId};
//!
//! // Create an empty cycle set
//! let mut set = CycleSet::empty();
//! set.insert(0);  // Add cycle with ID 0
//! set.insert(5);  // Add cycle with ID 5
//!
//! assert_eq!(set.len(), 2);
//! assert!(set.contains(0));
//! assert!(set.contains(5));
//! assert!(!set.contains(3));
//! ```
//!
//! # Note
//!
//! Full cycle enumeration (computing all 394 cycles for NCOLORS=6) will be
//! implemented in Phase 3. This module provides the container type for sets
//! of cycle IDs.

use crate::geometry::{constants::*, CycleId};
use std::fmt;

/// A set of cycles represented as a bitset.
///
/// Uses an array of u64 words to represent up to NCYCLES cycle IDs.
/// Bit i (across all words) is set if cycle i is in the set.
///
/// For NCOLORS=6 (NCYCLES=394), this uses 7 u64 words (448 bits total, 394 used).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CycleSet([u64; CYCLESET_LENGTH]);

impl CycleSet {
    /// Create an empty cycle set.
    pub const fn empty() -> Self {
        Self([0; CYCLESET_LENGTH])
    }

    /// Create a cycle set containing all valid cycles (0..NCYCLES).
    pub fn full() -> Self {
        let mut words = [0u64; CYCLESET_LENGTH];

        // Fill complete words
        let complete_words = NCYCLES / 64;
        for i in 0..complete_words {
            words[i] = u64::MAX;
        }

        // Fill partial last word if needed
        let remaining_bits = NCYCLES % 64;
        if remaining_bits > 0 {
            words[complete_words] = (1u64 << remaining_bits) - 1;
        }

        Self(words)
    }

    /// Check if the set contains a specific cycle.
    ///
    /// # Panics
    ///
    /// Panics if `id >= NCYCLES`.
    pub fn contains(&self, id: CycleId) -> bool {
        assert!(
            (id as usize) < NCYCLES,
            "CycleId out of range: {} >= {}",
            id,
            NCYCLES
        );
        let word_idx = (id / 64) as usize;
        let bit_idx = id % 64;
        (self.0[word_idx] >> bit_idx) & 1 != 0
    }

    /// Insert a cycle into the set.
    ///
    /// # Panics
    ///
    /// Panics if `id >= NCYCLES`.
    pub fn insert(&mut self, id: CycleId) {
        assert!(
            (id as usize) < NCYCLES,
            "CycleId out of range: {} >= {}",
            id,
            NCYCLES
        );
        let word_idx = (id / 64) as usize;
        let bit_idx = id % 64;
        self.0[word_idx] |= 1u64 << bit_idx;
    }

    /// Remove a cycle from the set.
    ///
    /// # Panics
    ///
    /// Panics if `id >= NCYCLES`.
    pub fn remove(&mut self, id: CycleId) {
        assert!(
            (id as usize) < NCYCLES,
            "CycleId out of range: {} >= {}",
            id,
            NCYCLES
        );
        let word_idx = (id / 64) as usize;
        let bit_idx = id % 64;
        self.0[word_idx] &= !(1u64 << bit_idx);
    }

    /// Get the number of cycles in the set (population count).
    pub fn len(&self) -> usize {
        self.0.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&w| w == 0)
    }

    /// Get a reference to the underlying bitset words.
    pub fn words(&self) -> &[u64; CYCLESET_LENGTH] {
        &self.0
    }

    /// Iterate over all cycle IDs in the set.
    ///
    /// Cycle IDs are yielded in ascending order (0, 1, 2, ...).
    pub fn iter(&self) -> impl Iterator<Item = CycleId> + '_ {
        CycleSetIter {
            words: &self.0,
            word_idx: 0,
            bit_idx: 0,
            remaining: NCYCLES,
        }
    }

    /// Compute the union of two cycle sets.
    pub fn union(&self, other: &Self) -> Self {
        let mut result = [0u64; CYCLESET_LENGTH];
        for i in 0..CYCLESET_LENGTH {
            result[i] = self.0[i] | other.0[i];
        }
        Self(result)
    }

    /// Compute the intersection of two cycle sets.
    pub fn intersection(&self, other: &Self) -> Self {
        let mut result = [0u64; CYCLESET_LENGTH];
        for i in 0..CYCLESET_LENGTH {
            result[i] = self.0[i] & other.0[i];
        }
        Self(result)
    }

    /// Compute the difference of two cycle sets (self - other).
    pub fn difference(&self, other: &Self) -> Self {
        let mut result = [0u64; CYCLESET_LENGTH];
        for i in 0..CYCLESET_LENGTH {
            result[i] = self.0[i] & !other.0[i];
        }
        Self(result)
    }
}

/// Iterator over cycle IDs in a CycleSet.
struct CycleSetIter<'a> {
    words: &'a [u64; CYCLESET_LENGTH],
    word_idx: usize,
    bit_idx: u64,
    remaining: usize, // Total cycle IDs to check (NCYCLES)
}

impl<'a> Iterator for CycleSetIter<'a> {
    type Item = CycleId;

    fn next(&mut self) -> Option<Self::Item> {
        while self.word_idx < CYCLESET_LENGTH {
            let cycle_id = self.word_idx as u64 * 64 + self.bit_idx;

            // Stop if we've checked all valid cycle IDs
            if cycle_id >= self.remaining as u64 {
                return None;
            }

            let bit_set = (self.words[self.word_idx] >> self.bit_idx) & 1 != 0;

            // Advance to next bit
            self.bit_idx += 1;
            if self.bit_idx >= 64 {
                self.bit_idx = 0;
                self.word_idx += 1;
            }

            if bit_set {
                return Some(cycle_id);
            }
        }
        None
    }
}

impl fmt::Display for CycleSet {
    /// Format a cycle set as "{0, 5, 12, ...}".
    ///
    /// Note: This shows cycle IDs, not the actual cycle contents.
    /// Full cycle data will be available in Phase 3.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for id in self.iter() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", id)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl From<&[CycleId]> for CycleSet {
    fn from(ids: &[CycleId]) -> Self {
        let mut set = Self::empty();
        for &id in ids {
            set.insert(id);
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let set = CycleSet::empty();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_full() {
        let set = CycleSet::full();
        assert!(!set.is_empty());
        assert_eq!(set.len(), NCYCLES);

        // Check all valid IDs are present
        for i in 0..NCYCLES as u64 {
            assert!(set.contains(i), "Missing cycle ID {}", i);
        }
    }

    #[test]
    fn test_insert_contains() {
        let mut set = CycleSet::empty();
        assert!(!set.contains(0));
        assert!(!set.contains(1));

        set.insert(0);
        assert!(set.contains(0));
        assert!(!set.contains(1));
        assert_eq!(set.len(), 1);

        set.insert(1);
        assert!(set.contains(0));
        assert!(set.contains(1));
        assert_eq!(set.len(), 2);

        // Insert duplicate - should be idempotent
        set.insert(1);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut set = CycleSet::full();
        let initial_len = set.len();

        set.remove(0);
        assert!(!set.contains(0));
        assert_eq!(set.len(), initial_len - 1);

        // Remove again - should be idempotent
        set.remove(0);
        assert_eq!(set.len(), initial_len - 1);
    }

    #[test]
    fn test_iter() {
        let mut set = CycleSet::empty();
        set.insert(0);
        set.insert(1);
        if NCYCLES > 2 {
            set.insert(2);
        }

        let ids: Vec<_> = set.iter().collect();
        assert_eq!(ids[0], 0);
        assert_eq!(ids[1], 1);
        if NCYCLES > 2 {
            assert_eq!(ids[2], 2);
            assert_eq!(ids.len(), 3);
        } else {
            assert_eq!(ids.len(), 2);
        }
    }

    #[test]
    fn test_iter_empty() {
        let set = CycleSet::empty();
        let ids: Vec<_> = set.iter().collect();
        assert_eq!(ids.len(), 0);
    }

    #[test]
    fn test_iter_full() {
        let set = CycleSet::full();
        let ids: Vec<_> = set.iter().collect();
        assert_eq!(ids.len(), NCYCLES);

        // Check IDs are in order
        for (i, &id) in ids.iter().enumerate() {
            assert_eq!(id, i as u64);
        }
    }

    #[test]
    fn test_union() {
        let mut set1 = CycleSet::empty();
        set1.insert(0);
        if NCYCLES > 1 {
            set1.insert(1);
        }

        let mut set2 = CycleSet::empty();
        set2.insert(0);

        let union = set1.union(&set2);
        assert!(union.contains(0));
        if NCYCLES > 1 {
            assert!(union.contains(1));
            assert_eq!(union.len(), 2);
        } else {
            assert_eq!(union.len(), 1);
        }
    }

    #[test]
    fn test_intersection() {
        let mut set1 = CycleSet::empty();
        set1.insert(0);
        if NCYCLES > 1 {
            set1.insert(1);
        }

        let mut set2 = CycleSet::empty();
        set2.insert(0);

        let intersection = set1.intersection(&set2);
        assert!(intersection.contains(0));
        if NCYCLES > 1 {
            assert!(!intersection.contains(1));
        }
        assert_eq!(intersection.len(), 1);
    }

    #[test]
    fn test_difference() {
        let mut set1 = CycleSet::empty();
        set1.insert(0);
        if NCYCLES > 1 {
            set1.insert(1);
        }

        let mut set2 = CycleSet::empty();
        set2.insert(0);

        let diff = set1.difference(&set2);
        assert!(!diff.contains(0));
        if NCYCLES > 1 {
            assert!(diff.contains(1));
            assert_eq!(diff.len(), 1);
        } else {
            assert_eq!(diff.len(), 0);
        }
    }

    #[test]
    fn test_display() {
        let mut set = CycleSet::empty();
        assert_eq!(format!("{}", set), "{}");

        set.insert(0);
        if NCYCLES > 1 {
            set.insert(1);
            assert_eq!(format!("{}", set), "{0, 1}");
        } else {
            assert_eq!(format!("{}", set), "{0}");
        }
    }

    #[test]
    fn test_from_slice() {
        let ids = if NCYCLES >= 2 {
            vec![0u64, 1]
        } else {
            vec![0u64]
        };
        let set: CycleSet = (&ids[..]).into();

        assert!(set.contains(0));
        assert_eq!(set.len(), ids.len());
    }

    #[test]
    fn test_equality() {
        let mut set1 = CycleSet::empty();
        set1.insert(0);

        let mut set2 = CycleSet::empty();
        set2.insert(0);

        assert_eq!(set1, set2);

        if NCYCLES > 1 {
            set2.insert(1);
            assert_ne!(set1, set2);
        }
    }

    #[test]
    #[should_panic(expected = "CycleId out of range")]
    fn test_contains_out_of_range() {
        let set = CycleSet::empty();
        set.contains(NCYCLES as u64);
    }

    #[test]
    #[should_panic(expected = "CycleId out of range")]
    fn test_insert_out_of_range() {
        let mut set = CycleSet::empty();
        set.insert(NCYCLES as u64);
    }

    #[test]
    fn test_cycleset_length_correct() {
        // Verify CYCLESET_LENGTH has enough u64s for NCYCLES bits
        assert!(CYCLESET_LENGTH * 64 >= NCYCLES);
        // And not too many (less than 64 unused bits)
        assert!(CYCLESET_LENGTH * 64 - NCYCLES < 64);

        // Verify specific values
        match NCOLORS {
            3 => assert_eq!(CYCLESET_LENGTH, 1),   // 2 cycles need 1 u64
            4 => assert_eq!(CYCLESET_LENGTH, 1),   // 14 cycles need 1 u64
            5 => assert_eq!(CYCLESET_LENGTH, 2),   // 74 cycles need 2 u64s
            6 => assert_eq!(CYCLESET_LENGTH, 7),   // 394 cycles need 7 u64s
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_multi_word_operations() {
        // Test that operations work correctly across word boundaries
        // (only relevant for NCOLORS >= 5, where NCYCLES > 64)
        if NCYCLES > 64 {
            let mut set = CycleSet::empty();

            // Insert IDs in different words
            set.insert(0);      // Word 0, first bit
            set.insert(63);     // Word 0, last bit
            set.insert(64);     // Word 1, first bit

            // Only insert higher IDs if they're valid
            let mut expected = vec![0, 63, 64];
            if (NCYCLES as u64) > 70 {
                set.insert(70);
                expected.push(70);
            }

            assert!(set.contains(0));
            assert!(set.contains(63));
            assert!(set.contains(64));
            assert_eq!(set.len(), expected.len());

            // Test iteration across words
            let ids: Vec<_> = set.iter().collect();
            assert_eq!(ids, expected);
        }
    }
}
