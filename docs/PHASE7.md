# Phase 7: VennPredicate - Main Venn Diagram Search

**Status**: In Progress (PR #1 complete - Dynamic Face State & VennPredicate Skeleton)

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

- **NCOLORS=5** (`test_venn5.c`): 152 solutions for degree signature [0,0,0,0,0,0]
  - Tests incremental solution building

- **NCOLORS=6** (`test_venn6.c`): 233 solutions for degree signature [5,5,5,4,4,4]
  - Primary target configuration
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

### PR #2: Constraint Propagation (CRITICAL)

**Goal**: Implement the constraint propagation mechanism that prunes the 10^150 search space.

**⚠️ CRITICAL**: This is the **prerequisite for tractable search**. Without constraint propagation, the search space is impossibly large (~10^150 configurations).

- [ ] **Step 3**: Constraint propagation engine (~400 lines)
  - Implement cycle elimination based on MEMO tables
  - Edge adjacency constraints (cycle_pairs lookup)
  - Vertex adjacency constraints (cycle_triples lookup)
  - Color omission constraints (cycles_omitting_one_color)
  - Propagation queue/worklist algorithm
  - Failure detection (face with zero possible cycles)
  - Integration with Trail (record all eliminations)

**Test expectations**:
- Unit tests for constraint propagation on isolated faces
- Tests from `test_known_solution.c` (incremental solution building)
- Verify constraint propagation correctly eliminates invalid cycles
- Verify failure detection when face has no remaining cycles

**Estimated size**: ~400 lines, 15-20 tests

---

### PR #3: Face Selection & Cycle Assignment

**Goal**: Implement the core search loop - choosing faces and assigning cycles.

- [ ] **Step 4**: Face selection heuristics (~150 lines)
  - Find unassigned face with fewest possible cycles (fail-fast)
  - Break ties consistently (deterministic search order)
  - Track assigned vs. unassigned faces

- [ ] **Step 5**: Cycle assignment & choice point creation (~200 lines)
  - Assign cycle to selected face
  - Create choice point for trying alternative cycles
  - Trigger constraint propagation after assignment
  - Handle immediate failure (backtrack if propagation fails)

**Test expectations**:
- Tests from `test_venn3.c` (simple 8-face diagrams)
- Verify face selection chooses minimum-option face
- Verify cycle assignment triggers propagation
- Verify immediate backtrack on propagation failure

**Estimated size**: ~350 lines, 12-15 tests

---

### PR #4: Backtracking & Search Completion

**Goal**: Complete the search loop with proper backtracking and success detection.

- [ ] **Step 6**: Backtracking logic (~150 lines)
  - Implement retry_pred (try next cycle choice)
  - Handle exhausted choices (return Failure, pop to previous choice point)
  - Trail-based state restoration (automatic via existing trail system)

- [ ] **Step 7**: Search termination (~100 lines)
  - Detect success (all faces assigned)
  - Validate solution completeness
  - Return Success to engine

**Test expectations**:
- Complete `test_venn3.c` tests (find both 2 solutions)
- Tests from `test_venn4.c` (24 solutions across various signatures)
- Verify backtracking restores state correctly
- Verify all solutions found match expected count

**Estimated size**: ~250 lines, 10-12 tests

---

### PR #5: Full Validation & Performance

**Goal**: Scale to full NCOLORS=6 and validate against all known results.

- [ ] **Step 8**: NCOLORS=5 and NCOLORS=6 tests (~200 lines test code)
  - Implement tests from `test_venn5.c` (152 solutions)
  - Implement tests from `test_venn6.c` (233 solutions for [5,5,5,4,4,4])
  - Validate solution counts match C implementation
  - Validate solution structure (all faces assigned, constraints satisfied)

- [ ] **Step 9**: Performance validation & optimization
  - Measure search performance vs. C implementation
  - Profile hot paths (constraint propagation, face selection)
  - Optimize if needed (goal: within 0.5-2x of C performance)
  - Document performance characteristics

**Test expectations**:
- **NCOLORS=5**: Find all 152 solutions for signature [0,0,0,0,0,0]
- **NCOLORS=6**: Find all 233 solutions for signature [5,5,5,4,4,4]
- Performance within 0.5-2x of C implementation (~5 seconds on similar hardware)

**Success criteria**: All tests passing, performance acceptable, ready for Phase 8 (CornersPredicate, LogPredicate, etc.)

**Estimated size**: ~300 lines (tests + optimization), 8-12 tests

---

## Summary

**Total estimated scope**:
- ~1600 lines of implementation code
- ~50-60 tests across 5 PRs
- ~9 distinct implementation steps

**Critical path**: PR #2 (Constraint Propagation) is the key enabler - without it, search is intractable.

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

- [x] PR #1: Dynamic Face State & VennPredicate Skeleton ✅ **COMPLETE**
  - Files: `src/state/faces.rs`, `src/predicates/venn.rs`, `src/context/mod.rs`, `src/geometry/cycle_set.rs`
  - Tests: 160 total passing (10 new in venn.rs)
  - Trail support: Full implementation with Option<u64> encoding and optimized CycleSet trailing
  - NCOLORS support: Verified across 3, 4, 5, 6
- [ ] PR #2: Constraint Propagation (CRITICAL) 🔜 **NEXT**
- [ ] PR #3: Face Selection & Cycle Assignment
- [ ] PR #4: Backtracking & Search Completion
- [ ] PR #5: Full Validation & Performance

**Phase 7 Status**: In Progress (1/5 PRs complete - 20%)
