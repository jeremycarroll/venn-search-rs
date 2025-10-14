# CLAUDE.md

This file provides guidance to Claude Code when working with this Rust rewrite of the venntriangles C codebase.

## Quick Start

**Note**: Most commands won't work until implementation is complete. Currently only basic Cargo commands work.

```bash
# Build the project (currently just builds "Hello, world!")
cargo build --release

# Run tests (none exist yet)
cargo test

# Check code compiles
cargo check

# Run clippy for linting
cargo clippy

# Format code
cargo fmt

# Commands below will work once implementation is complete:
# cargo run --release -- -f ../results
# cargo run --release -- -f ../results -d 664443
# cargo run --release -- -f ../results -n 1 -j 1
```

## Migration Status

This is a Rust rewrite of the C implementation at ../venntriangles (tag: v1.1-pco).

**Migration approach**: Incremental port with architecture improvements, leveraging Rust's type system while preserving the proven search algorithm.

### Current Status (as of October 2025)

**Completed:**
- ✅ Project structure and Cargo setup
- ✅ C reference files copied (25 .c files, 22 .h files in `c-reference/`)
- ✅ Initial documentation (README, CLAUDE.md, LICENSE)
- ✅ Git repository initialized

**In Progress:**
- 🚧 No implementation started yet - `src/main.rs` contains "Hello, world!" stub

**Not Started:**
- ⬜ Trail system implementation
- ⬜ Geometric types (Color, Cycle, Edge, Vertex, Face)
- ⬜ Search engine framework
- ⬜ Predicates (Initialize, InnerFace, Venn, Corners, Save)
- ⬜ Alternating operators (PCO, Chirotope)
- ⬜ GraphML output
- ⬜ CLI argument parsing
- ⬜ Test suite migration
- ⬜ Performance benchmarking

**Next Immediate Steps:**
1. Implement trail system (foundation for everything)
2. Port basic geometric types
3. Set up test infrastructure with first C test cases

## Reference C Implementation

The original C implementation is at: `/Users/jcarroll/venn/venntriangles`

Key commits to reference:
- Tag `v1.0`: Original working Venn 6-triangle search
- Tag `v1.1-pco`: Adds generalized alternating ternary operators (PCO and Chirotopes)

See `c-reference/` directory for copied C source files if needed.

## Architecture Overview

This program searches for monotone simple 6-Venn diagrams drawable with six triangles, as described in Carroll 2000. The search is divided into three main phases:

1. Finding maximal sequences of 6 integers making a 5-face degree signature
2. Finding 64 facial cycles defining a Venn diagram with this signature
3. Finding an edge-to-corner mapping where every pair of lines crosses at most once

Output is in GraphML format defining a planar graph with 18 pseudoline segments in six sets of three.

## Core Design Principles

### 1. Trail-Based Backtracking (CRITICAL)

The trail system is the core efficiency mechanism - **do not remove or simplify away**:

- **Purpose**: Efficient O(1) backtracking by recording state changes
- **C Implementation**: Array of trail entries with pointer-based rewind
- **Rust Implementation**: Should use `Vec<TrailEntry>` with index-based rewind, wrapped in type-safe API

**Suggested Rust approach**:
```rust
struct Trail {
    entries: Vec<TrailEntry>,
    checkpoint: usize,
}

struct Trailed<T> {
    value: T,
    // Reference to trail - exact mechanism TBD (Rc<RefCell<Trail>> or unsafe pointer)
}

impl<T: Copy> Trailed<T> {
    fn set(&mut self, new_value: T) {
        // Automatically record old value in trail before updating
    }
}
```

**Key insight**: The trail makes backtracking O(1) instead of O(n) and automatically handles all state restoration.

#### Trail System Design Decisions

**Open question: How to share Trail reference across Trailed values?**

Three main approaches to consider:

1. **`Rc<RefCell<Trail>>` (Safe, ergonomic)**
   - ✅ Safe Rust, no unsafe code
   - ✅ Easy to use, automatic reference counting
   - ✅ Good for initial implementation
   - ❌ Runtime overhead from RefCell borrow checking
   - ❌ Small allocation overhead
   - **Recommendation**: Start here, optimize later if needed

2. **`*mut Trail` with unsafe (Performance)**
   - ✅ Zero runtime overhead
   - ✅ Matches C implementation's approach
   - ❌ Requires unsafe code
   - ❌ Must carefully prove lifetime invariants
   - ❌ More complex to maintain
   - **Recommendation**: Only after profiling shows RefCell is a bottleneck

3. **Arena-based with indices (Middle ground)**
   - ✅ Safe Rust with minimal overhead
   - ✅ Good cache locality
   - ❌ More complex API (index instead of reference)
   - ❌ Requires arena allocator
   - **Recommendation**: Consider if RefCell overhead is measurable but unsafe isn't justified

**Suggested implementation path:**
1. Start with `Rc<RefCell<Trail>>` for correctness
2. Add comprehensive tests and benchmarks
3. Profile with real workloads
4. Optimize to unsafe/arena only if measurements justify it

### 2. Non-Deterministic Search Engine

Implements backtracking search via predicates:
- Each predicate has `try` (first attempt) and `retry` (backtracking) methods
- Engine maintains a stack of predicate states
- Trail system handles all state restoration on backtrack

**Rust mapping**:
- Trait-based predicates with `try_predicate` and `retry_predicate` methods
- Type-safe predicate results using enums
- Generic engine that works with any predicate sequence

### 3. Alternating Ternary Operators (Advanced)

The `pco` branch generalized two concepts:
- **Partial Cyclic Orders (PCO)**: Used for line crossing constraints
- **Chirotopes**: Used for oriented matroid testing (research/future use)

Both use alternating ternary relations with closure algorithms.

**Rust mapping**: Use traits to abstract the closure step:
```rust
trait AlternatingPredicate {
    fn closure_step(&self, i: usize, j: usize, k: usize, l: usize) -> bool;
}
```

### 4. Type-Safe Geometric Primitives

The C code uses careful conventions for:
- Colors (0..NCOLORS-1)
- Edges (directed, with reversal)
- Vertices (oriented meeting points)
- Faces (regions bounded by cycles)
- Cycles (sequences of edge colors)

**Rust advantages**:
- Use newtypes and enums for type safety
- Use `Option<T>` instead of null pointers
- Use slices and iterators instead of pointer arithmetic
- Compile-time bounds checking via const generics where possible

Example:
```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Color(u8);

#[derive(Copy, Clone, Debug)]
struct Edge {
    color: Color,
    face: FaceId,
    reversed: EdgeId,
}
```

## Module Structure (Planned)

```
src/
├── main.rs              - Entry point, CLI argument parsing
├── lib.rs               - Library root
├── trail/               - Trail-based backtracking system
│   ├── mod.rs
│   └── trailed.rs       - Trailed<T> wrapper type
├── engine/              - Non-deterministic search engine
│   ├── mod.rs
│   ├── predicate.rs     - Predicate trait and types
│   └── stack.rs         - Search stack management
├── geometry/            - Core geometric types
│   ├── mod.rs
│   ├── color.rs
│   ├── cycle.rs
│   ├── edge.rs
│   ├── vertex.rs
│   └── face.rs
├── alternating/         - Alternating ternary operators
│   ├── mod.rs
│   ├── pco.rs          - Partial cyclic orders
│   └── chirotope.rs    - Chirotope support
├── predicates/          - Search predicates
│   ├── mod.rs
│   ├── initialize.rs
│   ├── innerface.rs
│   ├── venn.rs
│   ├── corners.rs
│   └── save.rs
├── triangles.rs         - Triangle/line intersection logic
├── graphml.rs           - GraphML output generation
└── statistics.rs        - Search statistics tracking
```

## Migration Order (Suggested)

1. **Trail system** - Foundation for everything else
2. **Basic geometric types** - Color, Cycle, Edge
3. **Engine framework** - Predicate trait, search stack, basic predicates
4. **Face and Vertex** - More complex geometric types
5. **Initialize predicate** - First real predicate
6. **InnerFace predicate** - Builds face structure
7. **Venn predicate** - Validates Venn property
8. **Corners predicate** - Triangle/line mapping
9. **Alternating predicates** - PCO/Chirotope (advanced)
10. **GraphML output** - Results serialization
11. **Statistics and CLI** - Polish

## First Steps for Migration

### Phase 0: Setup and Preparation (Start Here!)

Before writing any Rust code:

1. **Study the C implementation**
   ```bash
   # Read the key files to understand the algorithm
   cd ../venntriangles
   less trail.h trail.c        # Trail system
   less engine.h engine.c      # Search engine
   less color.h cycle.h        # Basic types
   less venn.c                 # Main search
   ```

2. **Run C implementation to understand behavior**
   ```bash
   cd ../venntriangles
   make
   ./venn -f results -n 1      # Find one solution
   # Examine the output to understand expected behavior
   ```

3. **Extract test cases from C tests**
   ```bash
   # Look at test files in C implementation
   ls ../venntriangles/tests/
   # Identify which tests to port first
   ```

### Phase 1: Trail System (Week 1-2)

**Goal**: Implement and thoroughly test the trail system in isolation.

```bash
# Create module structure
mkdir -p src/trail
touch src/trail/mod.rs
touch src/trail/trailed.rs
touch src/lib.rs
```

**Implementation checklist:**
- [ ] `Trail` struct with `Vec<TrailEntry>`
- [ ] `checkpoint()` and `rewind()` methods
- [ ] `Trailed<T>` wrapper with automatic trail recording
- [ ] Unit tests for trail operations
- [ ] Benchmark trail performance vs. C implementation

**Reference files**: `c-reference/trail.h`, `c-reference/trail.c`

### Phase 2: Basic Types (Week 2-3)

**Goal**: Port Color, Cycle, and basic constants.

```bash
mkdir -p src/geometry
touch src/geometry/mod.rs
touch src/geometry/color.rs
touch src/geometry/cycle.rs
```

**Implementation checklist:**
- [ ] `Color` newtype with bounds checking
- [ ] `Cycle` type (sequence of colors)
- [ ] NCOLORS constant or const generic parameter
- [ ] Basic operations (equality, ordering, iteration)
- [ ] Unit tests from C test suite

**Reference files**: `c-reference/color.h`, `c-reference/cycle.h`

### Phase 3: Engine Framework (Week 3-4)

**Goal**: Implement the non-deterministic search engine.

```bash
mkdir -p src/engine
touch src/engine/mod.rs
touch src/engine/predicate.rs
touch src/engine/stack.rs
```

**Implementation checklist:**
- [ ] `Predicate` trait with `try_pred` and `retry_pred`
- [ ] `SearchEngine` that manages predicate stack
- [ ] Integration with Trail system
- [ ] Simple test predicate (e.g., "find integers 1-10")
- [ ] Verify backtracking works correctly

**Reference files**: `c-reference/engine.h`, `c-reference/engine.c`

### Getting Unstuck

If you encounter issues:

1. **Compare with C code**: The C implementation is proven and correct
2. **Add debug logging**: Use `tracing` or `println!` to understand state
3. **Write smaller tests**: Isolate the problematic component
4. **Check CLAUDE.md**: Review design decisions and recommendations
5. **Profile performance**: Use `cargo flamegraph` to find bottlenecks

### Success Criteria for Each Phase

**Phase 1 (Trail)**: Can checkpoint, modify values, and rewind correctly
**Phase 2 (Types)**: Can represent colors, cycles with type safety
**Phase 3 (Engine)**: Can run simple search with backtracking

## Key Differences from C

### Memory Management
- **C**: Manual malloc/free with careful ownership tracking
- **Rust**: Automatic via ownership system, no manual freeing needed

### Trail System
- **C**: Pointer-based with raw pointer arithmetic
- **Rust**: Index-based with safe Vec operations, wrapped in type-safe API

### Null Handling
- **C**: NULL pointers with careful checking
- **Rust**: Option<T> with compiler-enforced checking

### Array Bounds
- **C**: Manual bounds checking, easy to make mistakes
- **Rust**: Automatic bounds checking, panics on overflow

### Constants
- **C**: Compile-time NCOLORS via -DNCOLORS=6
- **Rust**: Const generics where possible, or runtime configuration

## Dependencies and Tooling

### Recommended Crates

**CLI and Configuration:**
- `clap` (v4.x) - Command line argument parsing with derive macros
- `anyhow` - Ergonomic error handling

**Serialization/Output:**
- `quick-xml` or `xml-rs` - GraphML output generation
- `serde` - If we need config file support

**Testing:**
- Built-in `cargo test` framework
- `proptest` - Property-based testing for geometric invariants
- `criterion` - Benchmarking framework for performance comparison

**Optional/Advanced:**
- `rayon` - If we want to parallelize independent searches later
- `tracing` - Structured logging/diagnostics (better than println!)
- `smallvec` - Stack-allocated vectors for small arrays

### Development Tools

```bash
# Essential
cargo install cargo-edit      # cargo add/rm commands
cargo install cargo-watch     # Auto-rebuild on changes
rustup component add clippy   # Linting
rustup component add rustfmt  # Formatting

# Recommended
cargo install cargo-criterion # Better criterion integration
cargo install cargo-flamegraph # Performance profiling
cargo install cargo-expand    # Macro expansion debugging
```

### Editor/IDE Setup

The project works with:
- **RustRover** (current setup) - Full IDE experience
- **VS Code** + rust-analyzer - Lightweight alternative
- **CLion** + Rust plugin - Similar to RustRover
- **Vim/Emacs** + rust-analyzer - For traditionalists

## Testing Strategy

The C implementation has extensive tests:
- PCO tests for NCOLORS=2,4,5
- Chirotope tests with known configurations
- Venn tests for NCOLORS=3,4,5,6
- Known solution validation tests

**Rust testing**:
- Port existing test cases as unit tests
- Add property-based tests where appropriate
- Ensure tests validate against known solutions
- Test both the trail system and engine independently

### Setting Up the Test Infrastructure

#### Unit Tests (Start Here)

Place unit tests in the same file as the implementation:

```rust
// src/trail/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trail_checkpoint_rewind() {
        // Test implementation
    }
}
```

#### Integration Tests

Create integration tests in `tests/` directory:

```bash
# Create test files
touch tests/trail_tests.rs
touch tests/engine_tests.rs
touch tests/venn_tests.rs
```

Structure integration tests to mirror C test suite:

```rust
// tests/trail_tests.rs
use venn_search::trail::{Trail, Trailed};

#[test]
fn test_trail_basic_operations() {
    // Port from C test suite
}
```

#### Property-Based Tests

Use `proptest` for geometric invariants:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_cycle_reversal_involution(colors in prop::collection::vec(0u8..6, 3..6)) {
        // Property: reverse(reverse(cycle)) == cycle
        let cycle = Cycle::from_colors(&colors);
        assert_eq!(cycle.reverse().reverse(), cycle);
    }
}
```

#### Test Organization Recommendations

```
tests/
├── common/
│   └── mod.rs           # Shared test utilities
├── trail_tests.rs       # Trail system tests
├── engine_tests.rs      # Search engine tests
├── geometry_tests.rs    # Color, Cycle, Edge, etc.
├── venn_n3_tests.rs    # 3-Venn known solutions
├── venn_n4_tests.rs    # 4-Venn known solutions
├── venn_n5_tests.rs    # 5-Venn known solutions
└── venn_n6_tests.rs    # 6-Venn known solutions
```

#### Porting C Tests

When porting tests from C:

1. **Identify the test**: Find in `../venntriangles/tests/`
2. **Understand the assertion**: What invariant is being checked?
3. **Port to Rust idiomatically**: Use Result, Option, iterators
4. **Document the original**: Add comment with C test file reference

Example:
```rust
/// Port of test_pco_n4() from c-reference/tests/pco_tests.c
#[test]
fn test_pco_n4() {
    // Test implementation
}
```

#### Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test trail_tests

# Run tests with output
cargo test -- --nocapture

# Run tests in parallel (default)
cargo test -- --test-threads=4

# Run single test
cargo test test_trail_checkpoint_rewind
```

## Performance Considerations

The C implementation is highly optimized:
- Trail-based backtracking for O(1) state restoration
- Careful memory layout for cache efficiency
- Minimal allocations during search

**Rust goals**:
- Match or exceed C performance
- Profile and optimize hot paths
- Use inline annotations where beneficial
- Consider unsafe blocks for critical paths (only after profiling)

### Establishing Performance Baseline

Before optimizing, establish baseline performance from the C implementation:

#### 1. Benchmark C Implementation

```bash
cd ../venntriangles
make clean && make CFLAGS="-O3 -march=native"

# Measure time for standard searches
time ./venn -f results -n 1 -j 1        # Find one solution
time ./venn -f results -d 664443 -n 1   # Specific degree sequence
time ./venn -f results -n 10            # Find 10 solutions

# Profile with perf (Linux) or Instruments (macOS)
perf record ./venn -f results -n 1
perf report
```

Record key metrics:
- **Time to first solution**: Target to match
- **Solutions per second**: Throughput metric
- **Memory usage**: Peak RSS
- **Hot functions**: From profiler (trail operations, predicate calls)

#### 2. Create Rust Benchmarks

Once Rust implementation is working, create matching benchmarks:

```rust
// benches/search_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use venn_search::*;

fn bench_find_one_solution(c: &mut Criterion) {
    c.bench_function("find_one_solution", |b| {
        b.iter(|| {
            // Run search to find one solution
            black_box(run_search_one());
        });
    });
}

criterion_group!(benches, bench_find_one_solution);
criterion_main!(benches);
```

#### 3. Compare Performance

Target performance ratios:
- **Initial implementation**: 0.5-2x C speed (acceptable)
- **After optimization**: 0.8-1.5x C speed (good)
- **Best case**: 1.0-2x C speed (excellent)

If Rust is significantly slower (>2x):
1. Profile with `cargo flamegraph` to find bottlenecks
2. Check for unnecessary allocations
3. Verify `--release` build with optimizations
4. Consider `#[inline]` for hot functions
5. Only then consider `unsafe` optimizations

#### 4. Performance Testing Commands

```bash
# Build with maximum optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Run benchmarks
cargo bench

# Profile with flamegraph
cargo flamegraph --bin venn -- -f results -n 1

# Check for unnecessary allocations
cargo install cargo-profdata
# (macOS: use Instruments)

# Compare binary sizes
ls -lh target/release/venn
ls -lh ../venntriangles/venn
```

#### 5. Optimization Priorities

Focus optimization effort on:
1. **Trail operations** - Called millions of times
2. **Predicate try/retry** - Core search loop
3. **PCO closure** - Expensive constraint propagation
4. **Memory allocations** - Minimize in hot paths

Don't optimize prematurely:
- GraphML output (runs once at end)
- CLI parsing (runs once at start)
- Statistics tracking (minimal overhead)

## Documentation Standards

- All public APIs must have doc comments
- Complex algorithms should reference the Carroll 2000 paper
- Cite specific C implementation functions when porting
- Explain non-obvious type safety invariants

## Advanced Rust Patterns to Explore

Since you're practicing advanced Rust:
- **Const generics** for compile-time NCOLORS validation
- **Trait objects** vs **static dispatch** for predicates
- **Unsafe** for performance-critical trail operations (carefully)
- **Iterators** for search space exploration
- **Zero-cost abstractions** for geometric types
- **Type states** for search phase tracking

## References

- Carroll 2000 paper (describing the algorithm)
- C implementation at ../venntriangles
- Images in /images directory (visual explanations of concepts)
