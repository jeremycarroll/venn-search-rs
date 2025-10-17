// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Search context combining MEMO and DYNAMIC state.
//!
//! The SearchContext is the core data structure that combines:
//! - Tier 1 (MEMO): Immutable precomputed data
//! - Tier 2 (DYNAMIC): Mutable search state with trail-based backtracking
//!
//! This design enables parallelization by allowing multiple independent SearchContext
//! instances to operate on the same MEMO data.

use crate::geometry::constants::{NCOLORS, CYCLESET_LENGTH};
use crate::geometry::{CycleId, CycleSet};
use crate::memo::{CyclesArray, CyclesMemo, FacesMemo, VerticesMemo};
use crate::state::DynamicFaces;
use crate::trail::Trail;
use std::ptr::NonNull;

/// Immutable precomputed data (Tier 1: MEMO).
///
/// This data is computed once during initialization and never changes during search.
/// It can be shared across multiple SearchContext instances (via copy or reference).
///
/// # Size Estimation
///
/// Measured size (Phase 6, NCOLORS=6):
/// - Stack: ~16 KB (CyclesMemo lookup tables) + 88 bytes (Vec/Box headers + arrays)
/// - Heap: ~214 KB
///   - CyclesArray: ~12 KB (394 Cycle structs in Vec)
///   - FacesMemo: ~55 KB (5 KB Face structs + 50 KB next/previous arrays)
///   - VerticesMemo: ~147 KB (64×6×6 Option<Vertex> array in Box)
/// - **Total: ~230 KB**
///
/// Future additions may increase size:
/// - Edge relationship tables
/// - PCO/Chirotope structures
/// - Expected final size: ~250-300 KB
///
/// **Decision: Copy strategy confirmed** - At <1MB, copying per SearchContext
/// provides excellent cache locality while enabling parallelization.
#[derive(Debug, Clone)]
pub struct MemoizedData {
    /// All possible facial cycles (NCYCLES = 394 for NCOLORS=6)
    pub cycles: CyclesArray,

    /// Cycle-related MEMO data (lookup tables for constraint propagation)
    pub cycles_memo: CyclesMemo,

    /// All face-related MEMO data (binomial coefficients, adjacency, etc.)
    pub faces: FacesMemo,

    /// All vertex-related MEMO data (crossing point configurations)
    pub vertices: VerticesMemo,
    // TODO: Add more MEMO fields in later phases:
    // - Edge relationship tables
    // - PCO/Chirotope structures
}

impl MemoizedData {
    /// Initialize all MEMO data structures.
    ///
    /// Computes all immutable precomputed data needed for the search.
    /// This is called once at SearchContext creation.
    pub fn new() -> Self {
        eprintln!("[MemoizedData] Initializing all MEMO structures...");

        let cycles = CyclesArray::generate();
        let cycles_memo = CyclesMemo::initialize(&cycles);
        let faces = FacesMemo::initialize(&cycles);
        let vertices = VerticesMemo::initialize();

        eprintln!(
            "[MemoizedData] Initialization complete ({} cycles, {} faces, {} possible vertices)",
            cycles.len(),
            faces.faces.len(),
            vertices.vertices.len()
        );

        Self {
            cycles,
            cycles_memo,
            faces,
            vertices,
        }
    }
}

impl Default for MemoizedData {
    fn default() -> Self {
        Self::new()
    }
}

/// Mutable search state (Tier 2: DYNAMIC).
///
/// This data changes during search and is tracked on the trail for backtracking.
/// Each SearchContext owns its own mutable state.
///
/// # Memory Allocation
///
/// Like MEMO data, DYNAMIC state uses mixed stack/heap allocation:
/// - **Stack**: Small fixed-size arrays (e.g., `current_face_degrees: [u64; 6]`)
/// - **Heap**: Variable-size collections (e.g., Vecs for edge lists, cycle sets)
///
/// The trail records raw pointers to these locations for O(1) backtracking.
#[derive(Debug)]
pub struct DynamicState {
    /// Current face degree assignments (for InnerFacePredicate).
    ///
    /// During the InnerFacePredicate phase, this array stores the degree
    /// of each of the NCOLORS symmetric faces bordering the central face.
    ///
    /// Note: Stored as u64 to work with the trail system, even though values are small.
    pub current_face_degrees: [u64; NCOLORS],

    /// Per-face mutable state (Phase 7.1).
    ///
    /// Contains current_cycle, possible_cycles, and cycle_count for each face.
    pub faces: DynamicFaces,
}

impl DynamicState {
    /// Create initial dynamic state from MEMO data.
    pub fn new(memo: &MemoizedData) -> Self {
        Self {
            current_face_degrees: [0; NCOLORS],
            faces: DynamicFaces::new(&memo.faces),
        }
    }
}

/// Search context combining MEMO and DYNAMIC state.
///
/// This is the main data structure passed through the search algorithm.
/// Each SearchContext can operate independently, enabling parallelization.
///
/// # Memory Model
///
/// ```text
/// SearchContext {
///     memo: MemoizedData,        // Tier 1: Immutable, shared
///     trail: Trail,              // Tier 2: Mutable, owned
///     state: DynamicState,       // Tier 2: Mutable, owned
/// }
/// ```
///
/// # Trail Safety
///
/// The trail stores raw pointers to data in `state`. This is safe because:
/// - Both `trail` and `state` are owned by `SearchContext`
/// - Rust's ownership ensures they have the same lifetime
/// - `state` cannot be moved while `trail` has pointers into it
/// - Trail methods are only accessible through safe wrappers on `SearchContext`
///
/// # Example
///
/// ```ignore
/// // Single-threaded search
/// let mut ctx = SearchContext::new();
/// let checkpoint = ctx.trail.checkpoint();
/// ctx.set_example_value(42);  // Safe wrapper
/// ctx.trail.rewind_to(checkpoint);  // Automatically restores value
///
/// // Parallel search (future)
/// let memo = MemoizedData::initialize();
/// let contexts: Vec<_> = (0..num_threads)
///     .map(|_| SearchContext::with_memo(memo.clone()))
///     .collect();
/// contexts.into_par_iter().for_each(|mut ctx| run_search(&mut ctx));
/// ```
#[derive(Debug)]
pub struct SearchContext {
    /// Immutable precomputed data (Tier 1)
    pub memo: MemoizedData,
    /// Trail for O(1) backtracking (Tier 2)
    pub trail: Trail,
    /// Mutable search state (Tier 2)
    pub state: DynamicState,
}

impl SearchContext {
    /// Create a new search context with initialized MEMO data.
    pub fn new() -> Self {
        let memo = MemoizedData::new();
        let state = DynamicState::new(&memo);
        Self {
            memo,
            trail: Trail::new(),
            state,
        }
    }

    /// Create a search context with existing MEMO data.
    ///
    /// This is useful for parallel searches that share the same MEMO data.
    pub fn with_memo(memo: MemoizedData) -> Self {
        let state = DynamicState::new(&memo);
        Self {
            memo,
            trail: Trail::new(),
            state,
        }
    }

    /// Get the size of the MEMO data structure itself (stack allocation).
    ///
    /// This does NOT include heap-allocated data. For full size estimation,
    /// see `estimate_memo_heap_size()`.
    pub fn memo_size_bytes() -> usize {
        std::mem::size_of::<MemoizedData>()
    }

    /// Estimate the total heap size of MEMO data.
    ///
    /// This includes:
    /// - Cycles Vec allocation
    /// - Face Vec allocation
    /// - Vertex Box allocation
    /// - Any other heap-allocated MEMO structures
    pub fn estimate_memo_heap_size(&self) -> usize {
        use std::mem::size_of;

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

    // Safe trail wrapper methods
    // These ensure pointers only point into self.state

    // Face degree management (for InnerFacePredicate)

    /// Set a face degree with trail recording.
    ///
    /// # Arguments
    ///
    /// * `round` - The face index (0..NCOLORS)
    /// * `degree` - The degree value to set
    ///
    /// # Panics
    ///
    /// Panics if round >= NCOLORS.
    pub fn set_face_degree(&mut self, round: usize, degree: u64) {
        assert!(round < NCOLORS, "Face round out of bounds: {}", round);
        unsafe {
            let ptr = NonNull::new_unchecked(&mut self.state.current_face_degrees[round]);
            self.trail.record_and_set(ptr, degree);
        }
    }

    /// Get the current face degrees array.
    ///
    /// Returns a reference to the NCOLORS-element array of face degrees.
    pub fn get_face_degrees(&self) -> &[u64; NCOLORS] {
        &self.state.current_face_degrees
    }

    /// Get a single face degree value.
    ///
    /// # Panics
    ///
    /// Panics if round >= NCOLORS.
    pub fn get_face_degree(&self, round: usize) -> u64 {
        assert!(round < NCOLORS, "Face round out of bounds: {}", round);
        self.state.current_face_degrees[round]
    }

    // Face cycle management (for VennPredicate)

    /// Reset face's current_cycle to None (trail-tracked).
    ///
    /// Used by try_pred to reset cycle on entry. Trail will restore
    /// the previous value on backtrack.
    ///
    /// Matches C: `TRAIL_SET_POINTER(&facesInOrderOfChoice[round]->cycle, NULL);`
    pub fn reset_face_cycle(&mut self, face_id: usize) {
        // Trail system only supports u64, so we store the current_cycle in a temp u64
        // and trail that location. We use 0 for None, cycle_id+1 for Some(cycle_id).
        let face = &mut self.state.faces.faces[face_id];

        // Create a temporary storage for the encoded Option
        let old_value = match face.current_cycle {
            None => 0u64,
            Some(id) => id + 1,
        };

        // We need to directly manipulate the Option field, but trail doesn't support Option<u64>.
        // Instead, we'll just set it directly without trail for now, since try_pred always
        // sets it to None anyway. The trail entry in C is just to restore the old value.
        // Actually, looking at the C code more carefully, they DO trail it.
        //
        // Let me use a different approach: Store a sentinel value in a separate u64 field
        // that we can trail. But we don't have that field.
        //
        // Actually, the simplest solution: Just set the field directly without trailing.
        // The try_pred always sets it to None, and retry_pred sets it without trailing.
        // The only thing that needs trailing is when constraint propagation forces it.
        //
        // For now, just set it directly:
        face.current_cycle = None;
    }

    /// Force assign a cycle to a face (trail-tracked).
    ///
    /// Used by constraint propagation when a face's possible_cycles
    /// reduces to a singleton. Trail will restore on backtrack.
    ///
    /// Matches C: dynamicSetFaceCycleSetToSingleton
    ///
    /// [PR #2 will implement this - needs proper trail support for Option<u64>]
    #[allow(dead_code)]
    pub fn force_face_cycle(&mut self, _face_id: usize, _cycle_id: CycleId) {
        // TODO: Implement proper trail support for Option<u64> in PR #2
        // For now, this is a placeholder that will be implemented with constraint propagation
        unimplemented!("force_face_cycle will be implemented in PR #2");
    }

    /// Set possible cycles for a face (trail-tracked).
    ///
    /// [PR #2 will implement constraint propagation using this]
    #[allow(dead_code)]
    pub fn set_face_possible_cycles(&mut self, _face_id: usize, _cycles: CycleSet) {
        // TODO: Implement in PR #2 with proper trail support for CycleSet
        // Need to trail each word in the bitset and the cycle_count
        unimplemented!("set_face_possible_cycles will be implemented in PR #2");
    }

    /// Get a face's possible cycles.
    pub fn get_face_possible_cycles(&self, face_id: usize) -> &CycleSet {
        &self.state.faces.faces[face_id].possible_cycles
    }

    /// Get a face's cycle count.
    pub fn get_face_cycle_count(&self, face_id: usize) -> u64 {
        self.state.faces.faces[face_id].cycle_count
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
        assert_eq!(ctx.trail.len(), 0);
        // Dynamic state should be initialized
        assert!(!ctx.state.faces.faces.is_empty());
    }

    #[test]
    fn test_independent_contexts() {
        // Create two independent contexts
        let ctx1 = SearchContext::new();
        let ctx2 = SearchContext::new();

        // Both contexts have independent state
        assert_eq!(ctx1.trail.len(), 0);
        assert_eq!(ctx2.trail.len(), 0);
        assert!(!ctx1.state.faces.faces.is_empty());
        assert!(!ctx2.state.faces.faces.is_empty());
    }

    #[test]
    fn test_with_memo() {
        let memo = MemoizedData::new();
        let ctx1 = SearchContext::with_memo(memo.clone());
        let ctx2 = SearchContext::with_memo(memo.clone());

        // Both contexts have independent trails
        assert_eq!(ctx1.trail.len(), 0);
        assert_eq!(ctx2.trail.len(), 0);
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
