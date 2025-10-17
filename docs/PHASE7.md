# Phase 7: VennPredicate - Main Venn Diagram Search

**Status**: In Progress (PR #1 & #2 complete - Dynamic Face State, VennPredicate Skeleton, Constraint Propagation)

**This is a living document**: Committed in first PR, updated throughout, removed in last PR.

## Overview

Phase 7 implements the main non-deterministic search for Venn diagram facial cycles. This is the **most critical phase** of the entire implementation - it's where we actually search for valid Venn diagrams.

**Key Challenge**: The raw search space is approximately **10^150** configurations. Constraint propagation is **absolutely required** to prune this to a tractable search space.

## Architecture

The VennPredicate searches for facial cycle assignments across all 64 faces (NCOLORS=6). For each face, it:
1. Chooses a facial cycle from the remaining possible cycles
2. Propagates constraints to neighboring faces (eliminates impossible cycles)
3. Selects the next face with the fewest remaining options (fail-fast heuristic)
4. Backtracks when a face has zero possible cycles remaining

**Integration with existing system**:
- **MEMO data** (Phase 6): Provides precomputed constraint lookup tables
- **Trail system** (Phase 1): Handles O(1) state restoration on backtrack
- **SearchEngine** (Phase 3): Manages predicate execution and backtracking
- **InnerFacePredicate** (Phase 5): Provides degree signatures as input

## Expected Results (from C reference tests)

Test expectations from `c-reference/test/test_venn*.c`:

- **NCOLORS=3** (`test_venn3.c`): 2 solutions total
  - Choosing one face forces all 7 other faces (highly constrained)

- **NCOLORS=4** (`test_venn4.c`): 24 total solutions across various degree signatures
  - Example: signature [6,3,3,3,0,0] has 8 solutions

- **NCOLORS=5** (`test_venn5.c`): 152 solutions for degree signature [0,0,0,0,0]
  - Tests incremental solution building

- **NCOLORS=6** (`test_venn6.c`): 233 solutions for degree signature [0,0,0,0,0,0]
  - Primary target configuration (unconstrained inner face degrees)
  - The complete search goal

**Test strategy**: Start with NCOLORS=3 (simplest, highly constrained), validate against known solutions, then scale up to N=4, N=5, and finally N=6.

## Implementation Steps (Work Order)

### PR #1: Dynamic Face State & VennPredicate Skeleton ✅ COMPLETE

**Goal**: Establish mutable face state and predicate integration with engine.

- [x] **Step 1**: Dynamic face state structures (~150 lines)
  - `DynamicFaces` struct with per-face mutable state
  - Per-face possible cycles (CycleSet, tracked on trail)
  - Integration with Trail for backtracking
  - Initialization from FacesMemo

- [x] **Step 2**: VennPredicate skeleton (~100 lines)
  - Predicate struct with basic state
  - Integration with SearchContext
  - Stub try_pred/retry_pred implementations
  - Basic tests (predicate creation, state initialization)

**Completed**: PR #1 implemented full trail support including:
- Option<u64> encoding (0 = None, n+1 = Some(n)) for trail tracking
- Optimized word-level trailing for CycleSet (only trails modified words)
- NCOLORS-aware tests (verified across NCOLORS=3,4,5,6)
- VennPredicate skeleton with fail-fast heuristic
- 10 new tests in venn.rs

**Actual size**: ~400 lines (including comprehensive trail support), 10 tests

**Test results**: All 160 tests passing across all NCOLORS values (3, 4, 5, 6)

---

### PR #2: Constraint Propagation (CRITICAL) ✅ COMPLETE

**Goal**: Implement the constraint propagation mechanism that prunes the 10^150 search space.

**⚠️ CRITICAL**: This is the **prerequisite for tractable search**. Without constraint propagation, the search space is impossibly large (~10^150 configurations).

- [x] **Step 3**: Constraint propagation engine (~400 lines)
  - Implement cycle elimination based on MEMO tables
  - Edge adjacency constraints (cycle_pairs lookup) - **Deferred to PR #3** (requires vertex/edge tracking)
  - Color omission constraints (cycles_omitting_one_color)
  - Color pair omission constraints (cycles_omitting_color_pair, upper triangle only)
  - Cascading propagation algorithm (singleton auto-assignment with recursion)
  - Failure detection (face with zero possible cycles)
  - Integration with Trail (record all eliminations)

**Completed**: PR #2 implemented core constraint propagation:
- New module: `src/propagation/mod.rs` (382 lines)
- PropagationFailure error enum with Display impl
- `propagate_cycle_choice()` - Main entry point called from VennPredicate::retry_pred
- `restrict_face_cycles()` - Workhorse with singleton auto-assignment and cascading
  - **KEY**: When face reduces to exactly 1 possible cycle, auto-assigns and recursively propagates
  - This cascading effect is critical for search tractability
- `propagate_non_adjacent_faces()` - Uses cycles_omitting_one_color from MEMO
- `propagate_non_vertex_adjacent_faces()` - Uses cycles_omitting_color_pair (upper triangle)
- `propagate_edge_adjacency()` - Stub with TODO (deferred to PR #3)
- `CycleSet::from_words()` - Convert MEMO lookup results (raw u64 arrays) to CycleSets
- Depth tracking (max 128) for recursion safety
- Integration into VennPredicate::retry_pred

**Actual size**: ~400 lines (370 in propagation module, 30 in supporting changes), unit tests in mod

**Test results**: All 140 tests passing across all NCOLORS values (3, 4, 5, 6)

---

### PR #3 (renumbered to #11): Integration Testing & Validation

**Status**: NEXT - Testing the search implementation

**What's Already Complete** (from PR #9-10):
- ✅ Face selection with fail-fast heuristic (`choose_next_face()`)
- ✅ Cycle assignment and choice iteration (`retry_pred()`)
- ✅ Constraint propagation with cascading singleton auto-assignment
- ✅ Failure handling (returns `Failure` on empty possible_cycles)
- ✅ Backtracking via engine + trail system
- ✅ Success detection (`try_pred()` returns `Success` when all faces assigned)

**Goal**: Test whether the search actually works end-to-end.

**Priority 1: Integration Testing**
- [ ] Write integration test for NCOLORS=3 (expect 2 solutions)
- [ ] Run actual VennPredicate searches end-to-end
- [ ] Verify solution counts match C reference (`test_venn3.c`)
- [ ] Validate solution structure (all faces assigned, constraints satisfied)

**Priority 2: Scale to NCOLORS=4-6**
- [ ] Test NCOLORS=4 (expect 24 solutions across various signatures)
- [ ] Test NCOLORS=5 (expect 152 solutions for signature [0,0,0,0,0])
- [ ] Test NCOLORS=6 (expect 233 solutions for signature [0,0,0,0,0,0])

**Optional: Edge Adjacency (if needed)**
- [ ] Implement vertex/edge tracking data structures
- [ ] Implement `propagate_edge_adjacency()` using cycle_pairs lookup
- [ ] Measure if edge adjacency significantly improves constraint pruning

**Note**: Edge adjacency may not be needed if current propagation is sufficient. Test first, then decide!

**Test expectations**:
- NCOLORS=3: Find both 2 solutions
- NCOLORS=4: Find all 24 solutions across various degree signatures
- NCOLORS=5: Find all 152 solutions for signature [0,0,0,0,0]
- NCOLORS=6: Find all 233 solutions for signature [0,0,0,0,0,0]

**Estimated size**: ~200-300 lines of test code

---

### PR #4 (renumbered to #12): Performance Validation & Optimization

**Goal**: Optimize performance and validate at scale.

- [ ] Performance validation & optimization
  - Measure search performance vs. C implementation
  - Profile hot paths (constraint propagation, face selection)
  - Optimize if needed (goal: within 0.5-2x of C performance)
  - Document performance characteristics

**Test expectations**:
- Performance within 0.5-2x of C implementation (~5 seconds on similar hardware for NCOLORS=6)

**Success criteria**: All tests passing, performance acceptable, ready for Phase 8 (CornersPredicate, LogPredicate, etc.)

**Estimated size**: ~100-200 lines (optimization + profiling)

---

## Summary

**Implementation Status**:
- ✅ Core search implementation ~800 lines (PR #9-10)
- ⏸️ Testing & validation ~300-400 lines (PR #11-12)
- **Total**: ~1200 lines across 4 PRs

**Key Insight**: Original PR #11-12 (Face Selection & Backtracking) were mostly implemented in PR #9-10 alongside the VennPredicate skeleton. This means we're further along than originally planned.

**Critical enabler**: PR #10 (Constraint Propagation) with cascading singleton auto-assignment - this prunes the ~10^150 search space to tractable size.

**Validation strategy**: Incremental testing from NCOLORS=3 → 4 → 5 → 6, validating against C reference test expectations at each step.

**Dependencies**:
- ✅ Phase 1 (Trail system) - complete
- ✅ Phase 2 (Geometric types) - complete
- ✅ Phase 3 (SearchEngine) - complete
- ✅ Phase 5 (InnerFacePredicate) - complete
- ✅ Phase 6 (MEMO data) - complete

## Detailed Plans

Before each PR, this section will be updated with detailed implementation plans for that PR's steps:

### PR #1 Detailed Plan

*To be added before starting PR #1*

---

## Progress Tracking

- [x] PR #9: Dynamic Face State & VennPredicate Skeleton ✅ **COMPLETE**
  - Files: `src/state/faces.rs`, `src/predicates/venn.rs`, `src/context/mod.rs`, `src/geometry/cycle_set.rs`
  - Tests: 160 total passing (10 new in venn.rs)
  - Trail support: Full implementation with Option<u64> encoding and optimized CycleSet trailing
  - NCOLORS support: Verified across 3, 4, 5, 6
  - **Also included**: Face selection heuristic, cycle assignment, backtracking integration
- [x] PR #10: Constraint Propagation (CRITICAL) ✅ **COMPLETE**
  - Files: `src/propagation/mod.rs` (new), `src/lib.rs`, `src/predicates/venn.rs`, `src/geometry/cycle_set.rs`
  - Tests: 140 total passing (3 new unit tests in propagation/mod.rs)
  - Cascading propagation: Singleton auto-assignment with recursive constraint propagation
  - MEMO integration: Uses cycles_omitting_one_color and cycles_omitting_color_pair lookup tables
  - Edge adjacency: Stub implementation (optional future work)
- [ ] ~~PR #11: Face Selection & Cycle Assignment~~ (mostly done in PR #9-10)
- [ ] ~~PR #12: Backtracking & Search Completion~~ (mostly done in PR #9-10)
- [ ] PR #11 (renumbered): Integration Testing & Validation 🔜 **NEXT**
- [ ] PR #12 (renumbered): Performance Validation & Optimization

**Phase 7 Status**: In Progress (~80% implementation complete, testing remains)
