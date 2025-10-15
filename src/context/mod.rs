// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Search context combining MEMO and DYNAMIC state.
//!
//! The SearchContext is the core data structure that combines:
//! - Tier 1 (MEMO): Immutable precomputed data
//! - Tier 2 (DYNAMIC): Mutable search state with trail-based backtracking
//!
//! This design enables parallelization by allowing multiple independent SearchContext
//! instances to operate on the same MEMO data.

use crate::geometry::constants::NCOLORS;
use crate::memo::{CyclesArray, FacesMemo, VerticesMemo};
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
/// - Stack: 88 bytes (Vec/Box headers + face_degree_by_color_count array)
/// - Heap: ~152 KB
///   - FacesMemo: ~5 KB (64 Face structs in Vec)
///   - VerticesMemo: ~147 KB (64×6×6 Option<Vertex> array in Box)
/// - **Total: ~149 KB**
///
/// Future additions may increase size:
/// - Cycle constraint lookup tables
/// - Edge relationship tables
/// - PCO/Chirotope structures
/// - Expected final size: ~200-300 KB
///
/// **Decision: Copy strategy confirmed** - At <1MB, copying per SearchContext
/// provides excellent cache locality while enabling parallelization.
#[derive(Debug, Clone)]
pub struct MemoizedData {
    /// All possible facial cycles (NCYCLES = 394 for NCOLORS=6)
    pub cycles: CyclesArray,

    /// All face-related MEMO data (binomial coefficients, adjacency, etc.)
    pub faces: FacesMemo,

    /// All vertex-related MEMO data (crossing point configurations)
    pub vertices: VerticesMemo,
    // TODO: Add more MEMO fields in later phases:
    // - Cycle constraint lookup tables
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
    // TODO: Add more DYNAMIC fields during Phase 6-7:
    // - Faces state (current facial cycle assignments)
    // - EdgeColorCount (crossing counts)
    // - Other mutable search state
    /// Current face degree assignments (for InnerFacePredicate).
    ///
    /// During the InnerFacePredicate phase, this array stores the degree
    /// of each of the NCOLORS symmetric faces bordering the central face.
    ///
    /// Note: Stored as u64 to work with the trail system, even though values are small.
    pub current_face_degrees: [u64; NCOLORS],

    // Example placeholders for demonstration (will be removed):
    pub example_value: u64,
    pub example_array: Vec<u64>,
}

impl DynamicState {
    /// Create initial dynamic state.
    pub fn new() -> Self {
        Self {
            current_face_degrees: [0; NCOLORS],
            example_value: 0,
            example_array: vec![0; 10],
        }
    }
}

impl Default for DynamicState {
    fn default() -> Self {
        Self::new()
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
        Self {
            memo: MemoizedData::new(),
            trail: Trail::new(),
            state: DynamicState::new(),
        }
    }

    /// Create a search context with existing MEMO data.
    ///
    /// This is useful for parallel searches that share the same MEMO data.
    pub fn with_memo(memo: MemoizedData) -> Self {
        Self {
            memo,
            trail: Trail::new(),
            state: DynamicState::new(),
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

    /// Set the example value with trail recording.
    ///
    /// # Example
    /// ```ignore
    /// let checkpoint = ctx.trail.checkpoint();
    /// ctx.set_example_value(42);
    /// ctx.trail.rewind_to(checkpoint);  // Value automatically restored
    /// ```
    pub fn set_example_value(&mut self, value: u64) {
        unsafe {
            let ptr = NonNull::new_unchecked(&mut self.state.example_value);
            self.trail.record_and_set(ptr, value);
        }
    }

    /// Set an array element with trail recording.
    ///
    /// # Panics
    ///
    /// Panics if index is out of bounds.
    pub fn set_array_element(&mut self, index: usize, value: u64) {
        unsafe {
            let ptr = NonNull::new_unchecked(&mut self.state.example_array[index]);
            self.trail.record_and_set(ptr, value);
        }
    }

    /// Conditionally set a value (only if different).
    ///
    /// Returns true if the value was changed, false otherwise.
    pub fn maybe_set_example_value(&mut self, value: u64) -> bool {
        unsafe {
            let ptr = NonNull::new_unchecked(&mut self.state.example_value);
            self.trail.maybe_record_and_set(ptr, value)
        }
    }

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

    // TODO: Add more specific trail methods as needed:
    // - set_face_cycle(&mut self, face_id: usize, cycle: u64)
    // - set_edge_count(&mut self, color_pair: usize, count: u64)
    // - set_bitmap(&mut self, bitmap: u64)
    // etc.
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
        assert_eq!(ctx.state.example_value, 0);
    }

    #[test]
    fn test_set_and_restore() {
        let mut ctx = SearchContext::new();

        let checkpoint = ctx.trail.checkpoint();
        ctx.set_example_value(42);

        assert_eq!(ctx.state.example_value, 42);
        assert_eq!(ctx.trail.len(), 1);

        ctx.trail.rewind_to(checkpoint);
        assert_eq!(ctx.state.example_value, 0); // Restored!
    }

    #[test]
    fn test_array_elements() {
        let mut ctx = SearchContext::new();

        let checkpoint = ctx.trail.checkpoint();
        ctx.set_array_element(3, 100);
        ctx.set_array_element(7, 200);

        assert_eq!(ctx.state.example_array[3], 100);
        assert_eq!(ctx.state.example_array[7], 200);

        ctx.trail.rewind_to(checkpoint);
        assert_eq!(ctx.state.example_array[3], 0); // Restored!
        assert_eq!(ctx.state.example_array[7], 0); // Restored!
    }

    #[test]
    fn test_independent_contexts() {
        // Create two independent contexts
        let mut ctx1 = SearchContext::new();
        let ctx2 = SearchContext::new();

        ctx1.trail.checkpoint();
        ctx1.set_example_value(100);

        // ctx2 should be completely unaffected
        assert_eq!(ctx1.state.example_value, 100);
        assert_eq!(ctx2.state.example_value, 0);
        assert_eq!(ctx1.trail.len(), 1);
        assert_eq!(ctx2.trail.len(), 0);
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
    fn test_maybe_set() {
        let mut ctx = SearchContext::new();
        ctx.trail.checkpoint();

        // Setting same value doesn't record
        assert!(!ctx.maybe_set_example_value(0));
        assert_eq!(ctx.trail.len(), 0);

        // Setting different value records
        assert!(ctx.maybe_set_example_value(42));
        assert_eq!(ctx.trail.len(), 1);
        assert_eq!(ctx.state.example_value, 42);
    }

    #[test]
    fn test_nested_operations() {
        let mut ctx = SearchContext::new();

        let cp1 = ctx.trail.checkpoint();
        ctx.set_example_value(10);

        let cp2 = ctx.trail.checkpoint();
        ctx.set_example_value(20);
        ctx.set_array_element(0, 100);

        assert_eq!(ctx.state.example_value, 20);
        assert_eq!(ctx.state.example_array[0], 100);

        // Rewind to cp2
        ctx.trail.rewind_to(cp2);
        assert_eq!(ctx.state.example_value, 10);
        assert_eq!(ctx.state.example_array[0], 0);

        // Rewind to cp1
        ctx.trail.rewind_to(cp1);
        assert_eq!(ctx.state.example_value, 0);
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
