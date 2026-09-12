// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Search context combining MEMO and DYNAMIC state.
//!
//! The SearchContext is the core data structure that combines:
//! - Tier 1 (MEMO): Immutable precomputed data
//! - Tier 2 (DYNAMIC): Mutable search state with trail-based backtracking
//!
//! This design enables parallelization by allowing multiple independent SearchContext
//! instances to operate on the same MEMO data.

pub mod dynamic;
pub mod memoized;

// Re-export for convenience
pub use dynamic::DynamicState;
pub use memoized::MemoizedData;

use crate::geometry::constants::NCOLORS;
use crate::geometry::{CycleId, CycleSet};
use crate::state::Statistics;
use crate::trail::{Trail, TrailedState};
use std::fs::File;
use std::io::BufWriter;
use std::mem::size_of;

/// Search context combining MEMO and DYNAMIC state.
///
/// This is the main data structure passed through the search algorithm.
/// Each SearchContext can operate independently, enabling parallelization.
///
/// The dynamic owner contains both state and its indexed undo log. Moving a
/// context preserves every undo target; immutable state access cannot resize or
/// replace its storage. Rewind costs O(k) for k recorded writes.
///
/// ```
/// use venn_search::SearchContext;
/// let mut ctx = SearchContext::new();
/// let checkpoint = ctx.checkpoint();
/// ctx.set_face_degree(0, 4);
/// let mut moved = Box::new(ctx);
/// moved.rewind_to(checkpoint);
/// assert_eq!(moved.get_face_degree(0), 0);
/// ```
///
/// The two halves cannot be swapped independently:
/// ```compile_fail
/// use venn_search::SearchContext;
/// let mut a = SearchContext::new();
/// let mut b = SearchContext::new();
/// std::mem::swap(&mut a.dynamic.state, &mut b.dynamic.state);
/// ```
/// ```compile_fail
/// use venn_search::SearchContext;
/// let mut ctx = SearchContext::new();
/// ctx.parts_mut().1.trail = venn_search::SearchContext::new().parts_mut().1.trail;
/// ```
///
/// Neither access path allows unrestricted mutable storage:
/// ```compile_fail
/// use venn_search::SearchContext;
/// let mut ctx = SearchContext::new();
/// ctx.state().faces.faces.clear();
/// ```
/// ```compile_fail
/// use venn_search::SearchContext;
/// let mut ctx = SearchContext::new();
/// ctx.parts_mut().1.state().vertex_processed.resize(0, 0);
/// ```
#[derive(Debug)]
pub struct SearchContext {
    /// Immutable precomputed data
    pub memo: MemoizedData,
    dynamic: TrailedState,
    output: Option<Box<BufWriter<File>>>,

    pub statistics: Statistics,
}

impl SearchContext {
    /// Create a new search context with initialized MEMO data.
    pub fn new() -> Self {
        let memo = MemoizedData::new();
        let dynamic = TrailedState::new(&memo);
        Self {
            memo,
            dynamic,
            output: None,
            statistics: Statistics::new(),
        }
    }

    /// Create a search context with existing MEMO data.
    ///
    /// This is useful for parallel searches that share the same MEMO data.
    pub fn with_memo(memo: MemoizedData) -> Self {
        let dynamic = TrailedState::new(&memo);
        Self {
            memo,
            dynamic,
            output: None,
            statistics: Statistics::new(),
        }
    }

    /// Get the size of the MEMO data structure itself (stack allocation).
    ///
    /// This does NOT include heap-allocated data. For full size estimation,
    /// see `estimate_memo_heap_size()`.
    pub fn memo_size_bytes() -> usize {
        size_of::<MemoizedData>()
    }

    /// Estimate the total heap size of MEMO data.
    ///
    /// This includes:
    /// - Cycles Vec allocation
    /// - Face Vec allocation
    /// - Vertex Box allocation
    /// - Any other heap-allocated MEMO structures
    pub fn estimate_memo_heap_size(&self) -> usize {
        let mut total = 0;

        // Cycles Vec: capacity * size_of<Cycle>
        total += self.memo.cycles.len() * size_of::<crate::geometry::Cycle>();

        // Faces Vec: capacity * size_of<Face>
        total += self.memo.faces.faces.capacity() * size_of::<crate::geometry::Face>();

        // Face degree array (on stack, counted in memo_size_bytes)
        // Not included in heap size

        // Vertices Box: full 3D array
        use crate::geometry::constants::{NCOLORS, NFACES};
        total += size_of::<Option<crate::geometry::Vertex>>() * NCOLORS * NCOLORS * NFACES;

        total
    }

    /// Immutable current search state.
    pub fn state(&self) -> &DynamicState {
        self.dynamic.state()
    }

    /// Inspect log length, capacity and entry layout.
    pub fn trail(&self) -> &Trail {
        self.dynamic.trail()
    }

    /// Borrow immutable MEMO and the inseparable mutable state/log owner.
    pub fn parts_mut(&mut self) -> (&MemoizedData, &mut TrailedState) {
        (&self.memo, &mut self.dynamic)
    }

    /// Current owner-local log position; see `TrailedState::checkpoint`.
    pub fn checkpoint(&self) -> usize {
        self.dynamic.checkpoint()
    }

    /// Restore this owner's writes; see `TrailedState::rewind_to` for checkpoint rules.
    pub fn rewind_to(&mut self, checkpoint: usize) {
        self.dynamic.rewind_to(checkpoint);
    }

    /// Prevent rewind past the current log position.
    pub fn freeze(&mut self) {
        self.dynamic.freeze();
    }

    /// Reinitialize state, log and output together. Existing checkpoints are invalidated.
    /// MEMO and statistics retain their existing values.
    pub fn reset_state(&mut self) {
        self.dynamic = TrailedState::new(&self.memo);
        self.output = None;
    }

    /// Replace the untrailed output stream for an OpenClose lifecycle.
    pub fn replace_output(
        &mut self,
        output: Option<Box<BufWriter<File>>>,
    ) -> Option<Box<BufWriter<File>>> {
        std::mem::replace(&mut self.output, output)
    }

    /// Borrow output mutably alongside immutable data used to format it.
    pub fn output_parts(
        &mut self,
    ) -> (
        &MemoizedData,
        &DynamicState,
        &Statistics,
        Option<&mut BufWriter<File>>,
    ) {
        (
            &self.memo,
            self.dynamic.state(),
            &self.statistics,
            self.output.as_deref_mut(),
        )
    }

    /// Set a face degree with trail recording. Panics if round >= NCOLORS.
    pub fn set_face_degree(&mut self, round: usize, degree: u64) {
        self.dynamic.set_face_degree(round, degree);
    }

    /// Get the current face degrees array.
    pub fn get_face_degrees(&self) -> &[u64; NCOLORS] {
        &self.state().current_face_degrees
    }

    /// Get one degree. Panics if round >= NCOLORS.
    pub fn get_face_degree(&self, round: usize) -> u64 {
        self.state().current_face_degrees[round]
    }

    /// Trail a cycle reset for predicate entry.
    pub fn reset_face_cycle(&mut self, face_id: usize) {
        self.dynamic.reset_face_cycle(face_id);
    }

    /// Trail a checked forced assignment.
    pub fn set_face_cycle(&mut self, face_id: usize, cycle_id: CycleId) {
        self.dynamic.set_face_cycle(face_id, cycle_id);
    }

    /// Advance the retry cursor without trailing it; it survives choice rewind.
    pub fn set_retry_cursor_untrailed(&mut self, face_id: usize, cycle: Option<CycleId>) {
        self.dynamic.set_retry_cursor_untrailed(face_id, cycle);
    }

    /// Trail changed cycle words and the cached count together.
    pub fn set_face_possible_cycles(&mut self, face_id: usize, new_cycles: CycleSet) {
        self.dynamic.set_face_possible_cycles(face_id, new_cycles);
    }

    /// Get a face's possible cycles.
    pub fn get_face_possible_cycles(&self, face_id: usize) -> &CycleSet {
        &self.state().faces.faces[face_id].possible_cycles
    }

    /// Get a face's cached cycle count.
    pub fn get_face_cycle_count(&self, face_id: usize) -> u64 {
        self.state().faces.faces[face_id].cycle_count
    }
}

impl Default for SearchContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_context_new() {
        let ctx = SearchContext::new();
        assert_eq!(ctx.trail().len(), 0);
        // Dynamic state should be initialized
        assert!(!ctx.state().faces.faces.is_empty());
    }

    #[test]
    fn test_independent_contexts() {
        // Create two independent contexts
        let ctx1 = SearchContext::new();
        let ctx2 = SearchContext::new();

        // Both contexts have independent state
        assert_eq!(ctx1.trail().len(), 0);
        assert_eq!(ctx2.trail().len(), 0);
        assert!(!ctx1.state().faces.faces.is_empty());
        assert!(!ctx2.state().faces.faces.is_empty());
    }

    #[test]
    fn test_with_memo() {
        let memo = MemoizedData::new();
        let ctx1 = SearchContext::with_memo(memo.clone());
        let ctx2 = SearchContext::with_memo(memo.clone());

        // Both contexts have independent trails
        assert_eq!(ctx1.trail().len(), 0);
        assert_eq!(ctx2.trail().len(), 0);
    }

    #[test]
    fn test_memo_size_logging() {
        let ctx = SearchContext::new();
        let stack_size = SearchContext::memo_size_bytes();
        let heap_size = ctx.estimate_memo_heap_size();
        let total = stack_size + heap_size;

        println!("MemoizedData stack size: {} bytes", stack_size);
        println!("MemoizedData heap size: {} bytes", heap_size);
        println!(
            "MemoizedData total size: {} bytes ({:.2} KB)",
            total,
            total as f64 / 1024.0
        );

        // Verify size is reasonable (should be under 1MB for NCOLORS=6)
        assert!(total < 1024 * 1024, "MEMO data should be under 1MB");
    }
}
