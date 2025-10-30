# Test Suite Documentation

This document describes the test suite for the Rust implementation.

## Overview

The Rust test suite validates the Venn diagram search implementation for N=3, 4, 5, and 6 colors. Tests use conditional compilation with feature flags to build for different values of NCOLORS.

## Test File Organization

| Test File | Purpose | Feature Flag |
|-----------|---------|--------------|
| `tests/trail_integration_test.rs` | Trail system and backtracking | (all) |
| `tests/engine_integration_test.rs` | Non-deterministic engine | (all) |
| `tests/venn3_test.rs` | 3-Venn diagram tests | `ncolors_3` |
| `tests/venn5_test.rs` | 5-Venn diagram tests | `ncolors_5` |
| `tests/venn6_test.rs` | 6-Venn diagram tests | `ncolors_6` (default) |
| `tests/known_solution_test.rs` | Carroll 2000 solution verification | `ncolors_6` |
| `tests/phase5_integration_test.rs` | Constraint propagation tests | (varies) |
| `tests/venn_integration_test.rs` | Full search integration | (varies) |
| `tests/common/mod.rs` | Shared test utilities | (all) |

## Running Tests

```bash
# Run all tests (default N=6)
cargo test

# Run tests for specific NCOLORS
cargo test --features ncolors_3
cargo test --features ncolors_5
cargo test --features ncolors_6  # default

# Run with output visible
cargo test -- --nocapture

# Run specific test
cargo test test_all
```

## Key Test Categories

### 1. Trail System Tests (`trail_integration_test.rs`)

Tests the trail-based backtracking system:
- Basic trail operations (push, pop, checkpoint, rewind)
- State restoration on backtrack
- Nested trail scopes

**Status**: Complete

### 2. Engine Tests (`engine_integration_test.rs`)

Tests the non-deterministic search engine:
- Predicate execution (try_pred, retry_pred)
- Backtracking through predicate stack
- Success/failure propagation
- OpenClose lifecycle hooks

**Status**: Complete

### 3. Venn Diagram Search Tests

#### N=3: `venn3_test.rs`

Tests the simplest case - single 3-Venn diagram with 2 labellings:

- **`test_venn3`**: Finds 2 solutions (ABC and ACB labellings)
  - Expected: 2 solutions
  - Status: ✅ Passing

See `images/` directory for visual diagrams of 3-Venn diagram configurations.

#### N=5: `venn5_test.rs`

Tests 5-Venn diagrams including isomorphism class verification:

- **`test_venn5_abcde`**: Tests degree signature (5,5,5,4,4)
  - Verifies 23 canonical solutions
  - Tests isomorphism elimination under D₅ symmetry
  - Status: ✅ Passing

- **Test coverage**: Basic search functionality, canonicality checks
- **Future**: Could add detailed isomorphism class analysis (testing all 10 D₅ symmetry operations explicitly)

#### N=6: `venn6_test.rs`

Tests 6-Venn diagrams (the main target case):

- **`test_664443`**: Degree signature [6,6,4,4,4,3] → 5 solutions
- **`test_664434`**: Degree signature [6,6,4,4,3,4] → 2 solutions
- **`test_655443`**: Degree signature [6,5,5,4,4,3] → 6 solutions
- **`test_all`**: Full search
  - Finds all 39 inner face degree signatures
  - Finds all 233 canonical solutions
  - Verifies canonicality filtering (eliminates ~14 equivocal duplicates)
  - Performance: ~3.5 seconds in release mode
  - Status: ✅ Passing

**Test utilities**:
- `FixedInnerFacePredicate`: Test helper to lock inner face degrees (in `tests/common/mod.rs`)
- `PrintSolutionCountPerInnerFace`: Logs solutions per degree signature

### 4. Known Solution Test (`known_solution_test.rs`)

Verifies the Carroll 2000 published solution:
- Tests deterministic propagation from known facial cycles
- Validates constraint propagation correctness
- Status: ✅ Passing

The test uses facial cycles from the published solution to verify that constraint propagation correctly derives the complete solution.

### 5. Constraint Propagation Tests (`phase5_integration_test.rs`)

Tests individual constraint propagation mechanisms:
- Edge adjacency constraints
- Non-adjacency constraints
- Vertex adjacency constraints
- Face degree restrictions

**Status**: Complete - tests various propagation scenarios

### 6. Integration Tests (`venn_integration_test.rs`)

End-to-end tests of the full search pipeline:
- Initialize → InnerFace → Venn → Fail
- Statistics collection
- Solution counting

**Status**: Complete

## Test Coverage Summary

### What's Well Tested ✅

- Trail system and backtracking
- Search engine mechanics
- Full search for N=3, 5, 6
- Solution counts match expected results
- Canonicality filtering for N=6
- Constraint propagation
- Carroll 2000 solution verification

### Test Documentation Gaps ⚠️

1. **Detailed vertex/edge tests**
   - Tests could validate specific vertex configurations at each face type
   - Tests could verify edge orientation (primary/secondary, clockwise/counterclockwise)
   - Currently tests validate via solution counts, not detailed internal state
   - **Impact**: Medium - internal correctness is validated indirectly via solution counts

2. **Isomorphism class analysis**
   - Tests could verify all D₅ group actions explicitly (10 symmetries: 5 rotations × 2 reflections)
   - Tests could validate signature maximization across all labellings
   - Currently tests verify canonicality works via correct solution counts
   - **Impact**: Low - canonicality checks work correctly as verified by solution counts

3. **Visual documentation**
   - Test documentation could include more visual diagrams showing:
     - Face arrangements for each test case
     - Vertex orientations (primary/secondary)
     - Edge directions and clockwise/counterclockwise traversal
   - Would aid understanding of complex test cases
   - **Impact**: High for understanding

4. **4-Venn diagram tests**
   - Could add dedicated venn4_test.rs with monotonicity verification
   - **Impact**: Low - 4-Venn is mainly for testing, not production target

## Adding New Tests

### Template for Degree Signature Test

```rust
#[test]
fn test_DEGREE_SIG() {
    // Test specific degree signature
    run_test(
        [d1, d2, d3, d4, d5, d6],  // neighbor degrees
        true,                       // expect_to_start
        N,                          // expected_canonical
        M                           // expected_equivocal
    );
}
```

### Template for Full Search Test

```rust
#[test]
fn test_custom_search() {
    let mut ctx = SearchContext::new();

    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(InnerFacePredicate))
        .add(Box::new(VennPredicate::new()))
        .add(Statistics::counting_predicate(
            Counters::VennSolutions,
            None,
        ))
        .terminal(Box::new(FailPredicate))
        .build();

    engine.search(&mut ctx);

    assert_eq!(ctx.statistics.get(Counters::VennSolutions), EXPECTED);
}
```

## Performance Benchmarks

Test performance in release mode (`cargo test --release`):

| Test | Time | Notes |
|------|------|-------|
| `test_all` (N=6) | ~3.5s | Full search: 39 inner faces, 233 solutions |
| `test_venn5_abcde` (N=5) | <1s | 23 solutions |
| `test_venn3` (N=3) | <0.1s | 2 solutions |

## Future Test Enhancements

Based on docs/CLEANUP.md analysis:

1. **Property-based tests** (using proptest)
   - Geometric invariants (face degree sums, edge counts)
   - Cycle validity properties
   - Symmetry properties

2. **Detailed state validation tests**
   - Port vertex configuration tests from C
   - Add edge orientation validation
   - Test face adjacency relationships

3. **Visual test documentation**
   - Add diagrams for key Rust test cases
   - Document expected internal states

4. **Stress tests**
   - Very deep search trees
   - Memory usage during backtracking

## References

- [TESTS-C.md](TESTS-C.md) - C test suite with visual diagrams
- [DESIGN.md](DESIGN.md) - Architecture and engine design
- [RESULTS.md](RESULTS.md) - Expected solution counts
- [Carroll 2000] - Original algorithm description
