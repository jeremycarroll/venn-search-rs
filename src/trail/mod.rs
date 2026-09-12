// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Indexed undo log and its paired state owner.
//!
//! No entry retains an address: moving the owner leaves every logical target valid.
//! Recording/checkpoint acquisition is O(1); rewind is O(k) for k undone entries.
//! Only this module can record or replay entries, always against the same state.

use crate::context::{DynamicState, MemoizedData};
use crate::geometry::constants::{CYCLESET_LENGTH, NCOLORS, NCYCLES, NFACES, NPOINTS};
use crate::geometry::{CurveLink, CycleId, CycleSet};
use crate::state::faces::encode_optional_index;
use crate::state::DynamicEdge;

/// Closed inventory of undo targets. Indices are checked before narrowing to u16.
#[derive(Debug, Clone, Copy)]
enum Target {
    FaceDegree(u16),
    CurrentCycle(u16),
    PossibleCycleWord(u16, u16),
    CycleCount(u16),
    EdgeConnection(u16, u16),
    DualFace(u16, bool),
    CrossingCount(u16, u16),
    VertexProcessed(u16),
    EdgeColorCount(u16, u16),
    ColorChecked(u16),
}

#[derive(Debug, Clone, Copy)]
struct TrailEntry {
    target: Target,
    old_value: u64,
}

/// Read-only log metadata. Mutation and replay belong to [`TrailedState`].
#[derive(Debug)]
pub struct Trail {
    entries: Vec<TrailEntry>,
    frozen_checkpoint: Option<usize>,
}

impl Trail {
    /// Maximum entries; overflow panics before the affected write.
    pub const MAX_SIZE: usize = 16_384;

    fn new() -> Self {
        Self {
            entries: Vec::with_capacity(Self::MAX_SIZE),
            frozen_checkpoint: None,
        }
    }

    fn ensure_room(&self, additional: usize) {
        assert!(
            additional <= Self::MAX_SIZE - self.entries.len(),
            "Trail overflow: exceeded {} entries",
            Self::MAX_SIZE
        );
    }

    /// Number of recorded writes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are no recorded writes.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Reserved entry capacity.
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    /// Size of one indexed undo entry, including alignment.
    pub fn entry_size_bytes() -> usize {
        std::mem::size_of::<TrailEntry>()
    }
}

/// Owns state and its log as an inseparable pair.
///
/// Safe clients may move or replace the entire owner. They cannot replace one
/// half, resize owned storage, or replay another log against it. Propagation
/// receives this owner and immutable MEMO through `SearchContext::parts_mut`.
#[derive(Debug)]
pub struct TrailedState {
    state: DynamicState,
    trail: Trail,
}

impl TrailedState {
    /// Initialize state and an empty log together.
    pub fn new(memo: &MemoizedData) -> Self {
        Self {
            state: DynamicState::new(memo),
            trail: Trail::new(),
        }
    }

    /// Immutable view of current state; no mutable storage escape is provided.
    pub fn state(&self) -> &DynamicState {
        &self.state
    }

    /// Inspect the log without access to recording or replay.
    pub fn trail(&self) -> &Trail {
        &self.trail
    }

    /// Record a log position in O(1).
    ///
    /// Positions are local to this owner and its current initialization. Use only
    /// positions from this owner that have not been discarded by rewind/reset.
    pub fn checkpoint(&self) -> usize {
        self.trail.len()
    }

    /// Restore writes in reverse order, stopping at the frozen boundary.
    ///
    /// The checkpoint must originate from this owner, without an intervening reset,
    /// and be no greater than the current log length. Rewind costs O(k) undone writes.
    pub fn rewind_to(&mut self, checkpoint: usize) {
        assert!(
            checkpoint <= self.trail.len(),
            "Checkpoint beyond trail end"
        );
        let target = checkpoint.max(self.trail.frozen_checkpoint.unwrap_or(0));
        while self.trail.len() > target {
            let entry = self.trail.entries.pop().unwrap();
            *Self::slot(&mut self.state, entry.target) = entry.old_value;
        }
    }

    /// Prevent subsequent rewinds from passing the current position.
    pub fn freeze(&mut self) {
        self.trail.frozen_checkpoint = Some(self.trail.len());
    }

    fn index(index: usize, limit: usize) -> u16 {
        assert!(index < limit, "Index {} out of range 0..{}", index, limit);
        u16::try_from(index).expect("Undo index exceeds u16")
    }

    /// All untracked slot access stays inside the owner, including reverse replay.
    fn slot(state: &mut DynamicState, target: Target) -> &mut u64 {
        match target {
            Target::FaceDegree(round) => &mut state.current_face_degrees[round as usize],
            Target::CurrentCycle(face) => {
                &mut state.faces.faces[face as usize].current_cycle_encoded
            }
            Target::PossibleCycleWord(face, word) => {
                &mut state.faces.faces[face as usize].possible_cycles.words_mut()[word as usize]
            }
            Target::CycleCount(face) => &mut state.faces.faces[face as usize].cycle_count,
            Target::EdgeConnection(face, color) => {
                &mut state.faces.faces[face as usize].edge_dynamic[color as usize].to_encoded
            }
            Target::DualFace(face, previous) => {
                let face = &mut state.faces.faces[face as usize];
                if previous {
                    &mut face.previous_face_id_encoded
                } else {
                    &mut face.next_face_id_encoded
                }
            }
            Target::CrossingCount(i, j) => state.crossing_counts.get_mut(i as usize, j as usize),
            Target::VertexProcessed(vertex) => &mut state.vertex_processed[vertex as usize],
            Target::EdgeColorCount(direction, color) => {
                &mut state.edge_color_counts[direction as usize][color as usize]
            }
            Target::ColorChecked(color) => &mut state.colors_checked[color as usize],
        }
    }

    fn record_and_set(&mut self, target: Target, value: u64) {
        self.trail.ensure_room(1);
        let slot = Self::slot(&mut self.state, target);
        self.trail.entries.push(TrailEntry {
            target,
            old_value: *slot,
        });
        *slot = value;
    }

    /// Set a degree, recording even repeated writes (degrees are unrestricted u64s).
    pub fn set_face_degree(&mut self, round: usize, degree: u64) {
        let round = Self::index(round, NCOLORS);
        self.record_and_set(Target::FaceDegree(round), degree);
    }

    /// Trail a current-cycle reset, including the previous retry cursor.
    pub fn reset_face_cycle(&mut self, face: usize) {
        let face = Self::index(face, NFACES);
        self.record_and_set(Target::CurrentCycle(face), 0);
    }

    /// Trail a forced cycle assignment after validating its configured domain.
    pub fn set_face_cycle(&mut self, face: usize, cycle: CycleId) {
        let face = Self::index(face, NFACES);
        let encoded = encode_optional_index(Some(cycle), NCYCLES);
        self.record_and_set(Target::CurrentCycle(face), encoded);
    }

    /// Advance the retry cursor WITHOUT recording it, so it survives choice rewind.
    pub fn set_retry_cursor_untrailed(&mut self, face: usize, cycle: Option<CycleId>) {
        let face = Self::index(face, NFACES);
        let encoded = encode_optional_index(cycle, NCYCLES);
        self.state.faces.faces[face as usize].current_cycle_encoded = encoded;
    }

    /// Update changed words and their cached count together in O(w) inspected words.
    /// Capacity is checked for the whole update before any state is changed.
    pub fn set_face_possible_cycles(&mut self, face: usize, cycles: CycleSet) {
        let face = Self::index(face, NFACES);
        let old = self.state.faces.faces[face as usize].possible_cycles;
        let count = cycles.len() as u64;
        let count_changed = self.state.faces.faces[face as usize].cycle_count != count;
        let changed_words = old
            .words()
            .iter()
            .zip(cycles.words())
            .filter(|(a, b)| a != b)
            .count();
        self.trail
            .ensure_room(changed_words + usize::from(count_changed));
        for word in 0..CYCLESET_LENGTH {
            if old.words()[word] != cycles.words()[word] {
                self.record_and_set(
                    Target::PossibleCycleWord(face, word as u16),
                    cycles.words()[word],
                );
            }
        }
        if count_changed {
            self.record_and_set(Target::CycleCount(face), count);
        }
    }

    /// Set an edge's checked optional connection, recording its previous encoding.
    pub fn set_edge_connection(&mut self, face: usize, color: usize, link: Option<CurveLink>) {
        let face = Self::index(face, NFACES);
        let color = Self::index(color, NCOLORS);
        let encoded = DynamicEdge::encode_to(link);
        self.record_and_set(Target::EdgeConnection(face, color), encoded);
    }

    /// Set the next dual face (None = 0, Some(id) = id + 1).
    pub fn set_next_face(&mut self, face: usize, next: Option<usize>) {
        self.set_dual_face(face, next, false);
    }

    /// Set the previous dual face, with the same checked optional-ID encoding.
    pub fn set_previous_face(&mut self, face: usize, previous: Option<usize>) {
        self.set_dual_face(face, previous, true);
    }

    fn set_dual_face(&mut self, face: usize, next: Option<usize>, previous: bool) {
        let face = Self::index(face, NFACES);
        let encoded = encode_optional_index(next.map(|id| id as u64), NFACES);
        self.record_and_set(Target::DualFace(face, previous), encoded);
    }

    /// Set an ordered color-pair count; i < j < NCOLORS is required in release too.
    pub fn set_crossing_count(&mut self, i: usize, j: usize, count: u64) {
        assert!(i < j, "CrossingCounts only valid for i < j");
        let i = Self::index(i, NCOLORS);
        let j = Self::index(j, NCOLORS);
        self.record_and_set(Target::CrossingCount(i, j), count);
    }

    /// Record first processing of a configured vertex; repeated processing is a no-op.
    pub fn mark_vertex_processed(&mut self, vertex: usize) {
        let vertex = Self::index(vertex, NPOINTS);
        if self.state.vertex_processed[vertex as usize] == 0 {
            self.record_and_set(Target::VertexProcessed(vertex), 1);
        }
    }

    /// Increment a direction/color edge count, recording the old count first.
    pub fn increment_edge_color_count(&mut self, direction: usize, color: usize) {
        let direction = Self::index(direction, 2);
        let color = Self::index(color, NCOLORS);
        let count = self.state.edge_color_counts[direction as usize][color as usize]
            .checked_add(1)
            .expect("Edge color count overflow");
        self.record_and_set(Target::EdgeColorCount(direction, color), count);
    }

    /// Trail the completion flag for a configured color.
    pub fn mark_color_checked(&mut self, color: usize) {
        let color = Self::index(color, NCOLORS);
        self.record_and_set(Target::ColorChecked(color), 1);
    }

    /// Reset the temporary propagation accumulator (untrailed).
    pub fn clear_completed_colors(&mut self) {
        self.state.colors_completed_this_call = 0;
    }

    /// Add one configured color to the temporary propagation accumulator (untrailed).
    pub fn add_completed_color(&mut self, color: usize) {
        let color = Self::index(color, NCOLORS);
        self.state.colors_completed_this_call |= 1u64 << color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn overflow_keeps_last_value_and_repeated_writes_rewind() {
        let memo = MemoizedData::new();
        let mut owner = TrailedState::new(&memo);
        for _ in 0..Trail::MAX_SIZE {
            owner.set_face_degree(0, 42);
        }
        assert_eq!(owner.trail().len(), Trail::MAX_SIZE);
        assert_eq!(owner.trail().capacity(), Trail::MAX_SIZE);
        assert!(catch_unwind(AssertUnwindSafe(|| owner.set_face_degree(0, 99))).is_err());
        assert_eq!(owner.state().current_face_degrees[0], 42);
        owner.rewind_to(0);
        assert_eq!(owner.state().current_face_degrees[0], 0);
    }

    #[test]
    fn cycle_update_overflow_keeps_words_and_cached_count_together() {
        let memo = MemoizedData::new();
        let mut owner = TrailedState::new(&memo);
        for _ in 0..Trail::MAX_SIZE - 1 {
            owner.set_face_degree(0, 42);
        }
        let before = owner.state().faces.faces[0].possible_cycles;
        let count = owner.state().faces.faces[0].cycle_count;
        assert!(catch_unwind(AssertUnwindSafe(|| {
            owner.set_face_possible_cycles(0, CycleSet::empty())
        }))
        .is_err());
        assert_eq!(owner.trail().len(), Trail::MAX_SIZE - 1);
        assert_eq!(owner.state().faces.faces[0].possible_cycles, before);
        assert_eq!(owner.state().faces.faces[0].cycle_count, count);
    }
}
