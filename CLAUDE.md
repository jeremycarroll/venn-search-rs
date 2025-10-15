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
- ✅ **Phase 1 Complete (Oct 14, 2025)**: Memory Architecture & Trail System
  - Trail system with checkpoint/rewind (Vec-based, ID-tracked)
  - Trailed<T> wrapper for type-safe tracked state
  - SearchContext combining MEMO + DYNAMIC tiers
  - 29 tests passing (21 unit + 8 integration)
  - Basic Color type skeleton
  - See [docs/PHASE1_COMPLETE.md](docs/PHASE1_COMPLETE.md) for details
- ✅ **Phase 2 Complete (Oct 14, 2025)**: Geometric Types
  - Color, ColorSet, Cycle, CycleSet types with full API
  - Edge, Vertex, Face types with relationships
  - Constants module (NCOLORS, NFACES, NPOINTS, etc.)
  - 87 tests passing (all type tests + Phase 1)
- ✅ **Phase 3 Complete (Oct 14, 2025)**: Non-Deterministic Search Engine
  - Predicate trait with try_pred(round) and retry_pred(round, choice)
  - PredicateResult: Success, SuccessSamePredicate, Failure, Choices(n), Suspend
  - SearchEngine with WAM-like execution model
  - Stack-based execution tracking rounds and choices
  - Test predicates: IntegerRange, Choice, Suspend, AlwaysFail, MultiRound
  - Trail enhanced with rewind_to() for stack-based rewinding
  - **Consuming API**: search(self) -> Option<Self> enforces WAM semantics via ownership
  - **Type-safe builder**: EngineBuilder with typestate enforces terminal predicates at compile time
  - TerminalPredicate marker trait prevents invalid programs
  - 117 tests passing (93 unit + 11 integration + 13 doc tests)
  - Engine ready for real predicates

**In Progress:**
- 🚧 Nothing currently

**Not Started:**
- ⬜ **Phase 4**: GitHub Actions CI/CD (test automation for NCOLORS=3,4,5,6)
- ⬜ **Phase 5**: InitializePredicate & InnerFacePredicate (finding 5-face degree signatures)
- ⬜ **Phase 6**: MEMO Data Structures (complex precomputed lookup tables, builder pattern)
- ⬜ **Phase 7**: VennPredicate (main Venn diagram search - the critical phase)
- ⬜ **Phase 8**: Testing & Validation (real searches, performance benchmarking)
- ⬜ **Future Phases**: TBD after Phase 8 (Log, Save, Corners, GraphML predicates)
- ⬜ Alternating operators (PCO, Chirotope)
- ⬜ CLI argument parsing

**Next Immediate Steps:**
1. ✅ ~~Implement trail system (foundation for everything)~~ **COMPLETE**
2. ✅ ~~Port basic geometric types (Color, ColorSet, Cycle)~~ **COMPLETE**
3. ✅ ~~Implement search engine framework~~ **COMPLETE**
4. Set up GitHub Actions CI/CD for automated testing
5. Implement InitializePredicate and InnerFacePredicate
6. Build MEMO data structures with builder pattern
7. Implement VennPredicate for main search
8. Validate with real searches and performance benchmarks

## Reference C Implementation

The original C implementation is at: https://github.com/roll/venntriangles

Key releases to reference:
- Tag [`v1.0`](https://github.com/roll/venntriangles/releases/tag/v1.0): Original working Venn 6-triangle search
- Tag [`v1.1-pco`](https://github.com/roll/venntriangles/releases/tag/v1.1): Adds generalized alternating ternary operators (PCO and Chirotopes)

See `c-reference/` directory for copied C source files (from v1.1-pco tag).

## Architecture Overview

This program searches for monotone simple 6-Venn diagrams drawable with six triangles, as described in [Carroll 2000]. The search is divided into three main phases:

1. Finding maximal sequences of 6 integers making a 5-face degree signature
2. Finding 64 facial cycles defining a Venn diagram with this signature
3. Finding an edge-to-corner mapping where every pair of lines crosses at most once

Output is in GraphML format defining a planar graph with 18 pseudoline segments in six sets of three.

**For detailed information**, see:
- **[docs/DESIGN.md](docs/DESIGN.md)** - Comprehensive design documentation including the non-deterministic engine, memory management, predicates, and implementation details
- **[docs/MATH.md](docs/MATH.md)** - Mathematical foundations, Venn diagram theory, isomorphism types, and pseudoline arrangements
- **[docs/RESULTS.md](docs/RESULTS.md)** - Expected results: 233 solutions with 1.7M variations, performance benchmarks
- **[docs/TESTS.md](docs/TESTS.md)** - Test suite documentation with visual diagrams for 3-, 4-, 5-, and 6-Venn cases

## Memory Architecture

**CRITICAL DESIGN DECISION**: The Rust implementation uses a two-tier memory model designed to enable future parallelization while maintaining the C version's performance characteristics.

### Two-Tier Model

The C implementation uses global static variables for all state. Rust replaces this with explicit heap allocation and clear separation between immutable and mutable state.

**Tier 1: MEMO Data (Immutable, Computed Once)**

All data computed during the `Initialize` predicate that never changes during search:
- Facial cycle constraint lookup tables (bitwise operations)
- Possible vertex configurations (480 entries for N=6)
- Possible edge relationships
- Cycle containment sets (for triples i,j,k)
- All other precomputed lookup tables

This data is **search-invariant** - it cannot depend on the current search state because that would require tracking on the trail for backtracking.

**Rust strategy:**
- Compute once during initialization
- Store in `MemoizedData` struct
- Each `SearchContext` either owns a copy (if small) or borrows a `&'static` reference (if moderate)
- Size estimate: ~100KB-1MB (exact size determined during implementation)
- Decision on copy vs. reference made based on measured size

**Tier 2: DYNAMIC Data (Mutable, Per-Search)**

All data that changes during search, tracked on the trail:
- `Trail` - records all state changes for O(1) backtracking
- `Faces` - current facial cycle assignments, edges, vertices
- `EdgeColorCount` - crossing counts for current solution
- Other state marked DYNAMIC in C code (see [docs/DESIGN.md](docs/DESIGN.md))

**Rust strategy:**
- Each `SearchContext` owns its mutable state
- Changes recorded on trail before modification
- On backtrack, trail rewinds to restore previous state
- Pre-allocated capacities to avoid reallocation during search

### Heap Allocation Strategy

Unlike the C version's global static variables, the Rust version uses heap allocation:

**Benefits:**
- Enables multiple independent `SearchContext` instances
- Allows future parallelization at InnerFacePredicate boundary (see below)
- Better testing (can run searches in isolation)
- More idiomatic Rust (explicit ownership, no global mut, no unsafe)

**Performance:**
- One-time allocation cost during initialization (negligible)
- No per-operation overhead vs. global statics
- Pre-allocated capacities avoid reallocation during search
- MEMO data has excellent cache locality (read-only, frequently accessed)

### Memory Layout

**Current implementation: Single-threaded**

```rust
pub struct MemoizedData {
    cycle_constraints: CycleConstraints,
    possible_vertices: PossibleVertices,
    // Other MEMO fields
}

pub struct SearchContext {
    memo: MemoizedData,  // Owned copy (or &'static reference, TBD)
    trail: Trail,
    faces: Faces,
    edge_color_count: EdgeColorCount,
    // Other DYNAMIC state
}

impl SearchContext {
    pub fn new() -> Self {
        // Compute MEMO data once
        let memo = MemoizedData::initialize();

        SearchContext {
            memo,
            trail: Trail::with_capacity(10000),  // Pre-allocate for search depth
            faces: Faces::new(),
            edge_color_count: EdgeColorCount::new(),
        }
    }
}
```

### Parallelization Strategy (Future Enhancement)

The memory architecture enables coarse-grain parallelization at the **InnerFacePredicate boundary**.

**Parallelization point:**
After InnerFacePredicate finds each 5-face degree signature (~10-20 solutions), spawn independent search for each.

**Why this boundary:**
- InnerFacePredicate is quick (< 1ms total for all degree signatures)
- Venn search per degree signature takes ~200-500ms
- Natural independence - each degree signature is a separate problem
- Matches the 1999/2000 implementation's coarse-grain parallel approach

**Implementation approach (when ready):**

1. Single-threaded initialization computes all MEMO data
2. InnerFacePredicate runs single-threaded, finds ~10-20 degree signatures
3. For each degree signature solution, create new `SearchContext`:
   - Copy or share MEMO data (depends on measured size)
   - Fresh DYNAMIC state (trail, faces, counters)
4. Use rayon or spawn threads for parallel Venn + Corners + GraphML searches
5. Each thread works independently with its own DYNAMIC state
6. Collect results via channels to writer thread

**Current status:** Architecture is parallelization-ready, but implement single-threaded first to validate correctness.

**Expected speedup:** 5-10x on modern multi-core CPUs (limited by ~10-20 parallel tasks, not by the 233 final solutions).

**Key architectural decision:** We avoid painting ourselves into a single-threaded corner by:
- Using heap allocation (not global statics)
- Owning state per context (enables Send + Sync)
- Separating read-only MEMO from mutable DYNAMIC state
- No shared mutable state across threads

## Core Design Principles

### 1. Trail-Based Backtracking (CRITICAL)

The trail system is the core efficiency mechanism - **do not remove or simplify away**:

- **Purpose**: Efficient O(1) backtracking by recording state changes
- **C Implementation**: Array of trail entries with pointer-based rewind
- **Rust Implementation**: Should use `Vec<TrailEntry>` with index-based rewind, wrapped in type-safe API

**Rust approach (aligned with Memory Architecture above)**:
```rust
struct Trail {
    entries: Vec<TrailEntry>,
    checkpoint: usize,
}

// Trailed values don't hold trail reference
struct Trailed<T> {
    value: T,
}

impl<T: Copy> Trailed<T> {
    fn set(&mut self, ctx: &mut SearchContext, new_value: T) {
        // Record old value on trail before updating
        ctx.trail.record_change(/* identifier */, self.value);
        self.value = new_value;
    }

    fn get(&self) -> T {
        self.value
    }
}
```

**Key insight**: The trail makes backtracking O(1) instead of O(n) and automatically handles all state restoration.

#### Trail System Design Decisions

**How to provide Trail access to Trailed values?**

The C implementation uses a global static trail. Rust uses explicit context passing.

**Recommended approach: SearchContext with owned trail**

```rust
pub struct SearchContext {
    memo: MemoizedData,       // Read-only MEMO data (Tier 1)
    trail: Trail,             // Owned mutable trail (Tier 2)
    faces: Faces,             // Owned mutable state (Tier 2)
    edge_color_count: EdgeColorCount,
    // Other DYNAMIC state
}

// Trailed values don't hold trail reference
pub struct Trailed<T> {
    value: T,
}

impl<T: Copy> Trailed<T> {
    pub fn set(&mut self, ctx: &mut SearchContext, new_value: T) {
        // Record old value on trail
        ctx.trail.record_change(/* address/id */, self.value);
        self.value = new_value;
    }

    pub fn get(&self) -> T {
        self.value
    }
}
```

**Key design points:**
- ✅ Trail owned by `SearchContext`, passed explicitly as `&mut`
- ✅ No runtime overhead (no RefCell, no Arc, no atomic ops)
- ✅ Each search context is independent (enables parallelization)
- ✅ Safe Rust with clear ownership
- ✅ Explicit context passing matches Rust idioms
- ❌ More verbose than C's global access (acceptable trade-off for safety and parallelization)

**Alternative approaches NOT recommended:**

1. **`Rc<RefCell<Trail>>` - Runtime overhead + prevents Send**
   - ❌ Runtime overhead from RefCell borrow checking
   - ❌ Prevents `Send` trait (can't move between threads)
   - ❌ Shared mutable state anti-pattern in Rust

2. **`Arc<Mutex<Trail>>` - Lock contention defeats parallelization**
   - ❌ Lock contention across threads
   - ❌ Defeats the purpose of parallelization
   - ❌ More complex than needed

3. **`*mut Trail` with unsafe - No benefit over safe approach**
   - ❌ Requires unsafe code throughout
   - ❌ Defeats Rust's safety guarantees
   - ❌ No performance benefit over context-passing

4. **Thread-local storage - Limits parallelization**
   - ❌ Limits parallelization opportunities
   - ❌ Hidden global state (reduces testability)
   - ❌ Less clear than explicit context

**Implementation path:**
1. Use `SearchContext` with explicit context passing
2. Pre-allocate trail capacity to minimize allocations
3. Each independent search owns its trail (enables parallelization)
4. Profile to verify performance matches C implementation

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

**Organized by memory tier and architectural concerns:**

```
src/
├── main.rs              - Entry point, CLI argument parsing
├── lib.rs               - Library root
│
├── memo/                - Tier 1: MEMO data (immutable, computed once)
│   ├── mod.rs           - MemoizedData struct
│   ├── cycles.rs        - Cycle constraint lookup tables
│   ├── vertices.rs      - Possible vertex configurations
│   └── initialize.rs    - Compute all MEMO data during initialization
│
├── context/             - SearchContext (combines both tiers)
│   └── mod.rs           - SearchContext struct, owns trail + state + references memo
│
├── trail/               - Tier 2: Trail system (mutable, per-search)
│   ├── mod.rs           - Trail struct
│   └── trailed.rs       - Trailed<T> wrapper type
│
├── state/               - Tier 2: DYNAMIC state (mutable, per-search)
│   ├── faces.rs         - Faces structure
│   ├── edges.rs         - Edge structures
│   └── counters.rs      - EdgeColorCount and other tracked state
│
├── engine/              - Non-deterministic search engine
│   ├── mod.rs
│   ├── predicate.rs     - Predicate trait and types
│   └── stack.rs         - Search stack management
│
├── geometry/            - Core geometric types (used across tiers)
│   ├── mod.rs
│   ├── color.rs
│   ├── cycle.rs
│   ├── edge.rs
│   ├── vertex.rs
│   └── face.rs
│
├── alternating/         - Alternating ternary operators
│   ├── mod.rs
│   ├── pco.rs          - Partial cyclic orders
│   └── chirotope.rs    - Chirotope support
│
├── predicates/          - Search predicates
│   ├── mod.rs
│   ├── initialize.rs    - Builds MEMO data
│   ├── innerface.rs     - Finds degree signatures (parallelization boundary)
│   ├── venn.rs          - Main Venn diagram search
│   ├── corners.rs       - Corner assignment
│   └── save.rs          - Solution serialization
│
├── triangles.rs         - Triangle/line intersection logic
├── graphml.rs           - GraphML output generation
└── statistics.rs        - Search statistics tracking
```

**Key organizational principles:**
- `memo/` contains all Tier 1 (MEMO) immutable data
- `trail/` and `state/` contain Tier 2 (DYNAMIC) mutable per-search data
- `context/` ties it all together in `SearchContext`
- `predicates/initialize.rs` populates MEMO data
- `predicates/innerface.rs` is the natural parallelization boundary

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

1. **Read the design documentation**
   ```bash
   # Start with comprehensive design overview
   less docs/DESIGN.md         # Architecture, engine, predicates, file layout
   less docs/MATH.md           # Mathematical foundations
   less docs/TESTS.md          # Test strategy and expected behavior
   ```

2. **Study the C implementation**
   ```bash
   # Read the key files to understand the algorithm
   cd ../venntriangles
   less trail.h trail.c        # Trail system
   less engine.h engine.c      # Search engine
   less color.h cycle.h        # Basic types
   less venn.c                 # Main search
   ```

   See [docs/DESIGN.md](docs/DESIGN.md) for complete file organization and naming conventions.

3. **Run C implementation to understand behavior**
   ```bash
   cd ../venntriangles
   make
   ./venn -f results -n 1      # Find one solution
   # Examine the output to understand expected behavior
   ```

   Expected results are documented in [docs/RESULTS.md](docs/RESULTS.md).

4. **Extract test cases from C tests**
   ```bash
   # Look at test files in C implementation
   ls ../venntriangles/tests/
   # Identify which tests to port first
   ```

   See [docs/TESTS.md](docs/TESTS.md) for detailed test documentation with visual diagrams.

### Phase 1: Memory Architecture & Foundation (Week 1-2)

**Goal**: Establish the two-tier memory model, trail system, and SearchContext. This is the architectural foundation that enables future parallelization.

```bash
# Create module structure (memory-focused)
mkdir -p src/{memo,context,trail,state}
touch src/lib.rs
touch src/memo/mod.rs
touch src/context/mod.rs
touch src/trail/{mod.rs,trailed.rs}
touch src/state/mod.rs
```

**Step 1: Design MemoizedData structure**
- Identify all MEMO fields from C code (fields marked with MEMO annotation in `c-reference/`)
- Create `MemoizedData` struct skeleton in `src/memo/mod.rs`
- Estimate memory size (will determine copy vs. `&'static` reference strategy)
- Document which C global variables map to which MEMO fields

**Step 2: Implement Trail system**
- `Trail` struct with `Vec<TrailEntry>` in `src/trail/mod.rs`
- `checkpoint()` and `rewind()` operations
- Test independently with simple values
- **Reference**: `c-reference/trail.h`, `c-reference/trail.c`

**Step 3: Implement SearchContext**
- Create `SearchContext` in `src/context/mod.rs`
- Combine MEMO (Tier 1) + DYNAMIC state (Tier 2) in single struct
- Implement `SearchContext::new()` with pre-allocated capacities
- Test creating multiple independent contexts
- Verify contexts don't interfere with each other

**Step 4: Implement Trailed<T>**
- Wrapper type in `src/trail/trailed.rs`
- Requires `&mut SearchContext` to modify (automatic trail recording)
- Test checkpoint/rewind with Trailed values in SearchContext

**Implementation checklist:**
- [x] `MemoizedData` skeleton with size estimate
- [x] `Trail` struct with `Vec<TrailEntry>`
- [x] `checkpoint()` and `rewind()` methods
- [x] `SearchContext` combining both tiers
- [x] `Trailed<T>` wrapper with automatic trail recording
- [x] Unit tests for trail operations
- [x] Independence tests (multiple SearchContext instances)
- [x] Benchmark trail performance vs. C implementation

**Success criteria:**
- Can create multiple `SearchContext` instances
- Each has independent trail and state
- Changes to one don't affect others
- Trail rewind correctly restores state
- Memory architecture ready for future parallelization

**Reference files**: `c-reference/trail.h`, `c-reference/trail.c`, `c-reference/core.h` (for MEMO/DYNAMIC annotations)

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

### Phase 4: GitHub Actions CI/CD (Week 5)

**Goal**: Set up automated testing infrastructure to run tests for all NCOLORS values on every push.

```bash
mkdir -p .github/workflows
touch .github/workflows/ci.yml
```

**Implementation checklist:**
- [ ] Create GitHub Actions workflow file
- [ ] Run `cargo test` for NCOLORS=3,4,5,6 (note: currently only N=6 implemented)
- [ ] Run `cargo test --doc` for documentation tests
- [ ] Run `cargo clippy` for linting
- [ ] Run `cargo fmt --check` for formatting validation
- [ ] Trigger on: push to PRs and push to main
- [ ] Cache cargo dependencies for faster builds
- [ ] Report test results clearly

**Workflow structure:**
```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        ncolors: [3, 4, 5, 6]
    steps:
      - uses: actions/checkout@v3
      - name: Run tests for NCOLORS=${{ matrix.ncolors }}
      # ... (when multiple NCOLORS support is added)
```

**Success criteria:**
- All tests pass on CI before merge
- Clear test results visible in PR checks
- Fast feedback loop (< 5 minutes per run)

**Reference**: See docs/TESTS.md for expected test coverage at each NCOLORS value

### Phase 5: InitializePredicate & InnerFacePredicate (Week 6-7)

**Goal**: Implement the first two real predicates that find 5-face degree signatures.

```bash
touch src/predicates/initialize.rs
touch src/predicates/innerface.rs
```

**Implementation checklist:**
- [ ] InitializePredicate: Single-call deterministic setup
  - [ ] Compute all MEMO data (cycle constraints, vertex configs)
  - [ ] Initialize Faces global state structure
  - [ ] Idempotent (can be called multiple times safely)
  - [ ] Never undone by backtracking
- [ ] InnerFacePredicate: Non-deterministic 5-face degree signature search
  - [ ] 6 non-deterministic calls to choose degree sequence
  - [ ] Degrees sum to 27 (total edges in N=6 Venn diagram)
  - [ ] Generate maximal sequences via choices
  - [ ] Uses SuccessSamePredicate for each round (0-5)
  - [ ] **This is the parallelization boundary** (save state here)
- [ ] Integration tests with known degree signatures
- [ ] Validate against NCOLORS=3,4,5 test cases

**Key architectural note:**
InnerFacePredicate is where parallelization will occur in the future. After this predicate finds each degree signature (~10-20 solutions), we can spawn independent searches for the Venn search phase.

**Success criteria:**
- Finds all valid 5-face degree signatures for NCOLORS=6
- Can run to completion with engine + SuspendPredicate
- State correctly saved for later predicates to use

**Reference files**: `c-reference/initialize.c`, `c-reference/innerface.c`, `c-reference/nondeterminism.c`

### Phase 6: MEMO Data Structures (Week 8-9)

**Goal**: Implement complex precomputed lookup tables and static state structures.

```bash
mkdir -p src/memo
touch src/memo/mod.rs
touch src/memo/cycles.rs
touch src/memo/vertices.rs
touch src/memo/edges.rs
touch src/memo/builder.rs
```

**Implementation checklist:**
- [ ] MemoizedData structure with all precomputed tables
  - [ ] Cycle constraint lookup tables (bitwise operations)
  - [ ] Possible vertex configurations (480 entries for N=6)
  - [ ] Possible edge relationships
  - [ ] Cycle containment sets (for triples i,j,k)
  - [ ] Edge adjacency constraints
  - [ ] Vertex adjacency constraints
- [ ] Builder pattern for initialization
  - [ ] Type-safe construction via builder
  - [ ] Validate all constraints during build
  - [ ] Immutable after construction
- [ ] Measure size of MemoizedData
  - [ ] If < 1MB: Copy per SearchContext (good cache locality)
  - [ ] If > 1MB: Use `&'static` via `Box::leak()` (zero copy)
- [ ] Integration with SearchContext
- [ ] Tests for all lookup operations

**Key design decision:**
The MEMO data is computed once and never changes during search. This allows aggressive optimization and enables parallelization since threads can share read-only MEMO data.

**Success criteria:**
- All MEMO tables correctly precomputed
- Size measured and strategy chosen (copy vs. reference)
- Fast lookup operations (bitwise operations where possible)
- Ready for VennPredicate to use

**Reference files**: `c-reference/initialize.c` (MEMO data computation), `c-reference/face.h`, `c-reference/vertex.h`, `c-reference/edge.h`

### Phase 7: VennPredicate (Week 10-12)

**Goal**: Implement the main Venn diagram search - the most critical predicate.

```bash
touch src/predicates/venn.rs
```

**Implementation checklist:**
- [ ] VennPredicate: Main non-deterministic Venn diagram search
  - [ ] Up to 64 non-deterministic calls (one per face)
  - [ ] Choose facial cycle for each face
  - [ ] Constraint propagation via MEMO tables
  - [ ] Enforce Venn property (all intersections present)
  - [ ] Track remaining possible cycles per face
  - [ ] Choose face with fewest remaining options (heuristic)
  - [ ] Backtrack on failure (empty possible cycles)
- [ ] Constraint checking predicates
  - [ ] Cycle containment constraints
  - [ ] Edge adjacency constraints
  - [ ] Vertex configuration constraints
- [ ] Integration with trail for O(1) backtracking
- [ ] Statistics tracking (for debugging and performance)
- [ ] Comprehensive tests with NCOLORS=3,4,5,6

**Key performance note:**
This is the most computationally intensive phase. The 1999 implementation took about a year of CPU time to run up to this point, but the logic is much improved now. Expected runtime: ~200-500ms per degree signature with modern optimizations.

**Success criteria:**
- Finds valid Venn diagrams for each degree signature
- Correctly backtracks on constraint violations
- Performance comparable to C implementation
- Ready for real searches

**Reference files**: `c-reference/venn.c`, `c-reference/search.c`, `c-reference/failure.c`

### Phase 8: Testing & Validation (Week 13-14)

**Goal**: Validate the implementation with real searches and performance benchmarking.

**Implementation checklist:**
- [ ] Real search tests for NCOLORS=3,4,5,6
  - [ ] N=3: Expect 1 solution (trivial case)
  - [ ] N=4: Expect 3 solutions
  - [ ] N=5: Expect 23 solutions
  - [ ] N=6: Expect 233 solutions (target)
- [ ] Performance benchmarking
  - [ ] Compare with C implementation baseline
  - [ ] Measure time to first solution
  - [ ] Measure solutions per second
  - [ ] Profile hot paths (trail, predicates, MEMO lookups)
- [ ] Memory profiling
  - [ ] Measure SearchContext size
  - [ ] Verify no memory leaks
  - [ ] Check trail performance (O(1) backtrack)
- [ ] Validate against known solutions
  - [ ] GraphML output comparison (when implemented)
  - [ ] Solution isomorphism checking
- [ ] Documentation of results in docs/RESULTS.md

**Success criteria:**
- All expected solutions found for NCOLORS=3,4,5,6
- Performance within 0.5-2x of C implementation
- No memory leaks or performance regressions
- Ready for remaining predicates (Corners, GraphML)

**Reference**: See docs/RESULTS.md for expected performance and solution counts

### Future Phases (TBD after Phase 8)

After Phase 8, regroup to plan remaining predicates:

**Remaining predicates to implement:**
- LogPredicate: Deterministic logging (forward/backward execution tracking)
- SavePredicate: Write solutions to files
- CornersPredicate: 6 calls to assign 18 corners to faces
- GraphMLPredicate: Write variation in GraphML format

**Additional work:**
- PCO (Partial Cyclic Orders) for line crossing constraints
- Chirotope support for oriented matroid testing
- CLI argument parsing and output management
- Final performance optimization and parallel execution

The decision on how many phases to use for this remaining work will be made after assessing progress through Phase 8.

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

*See the [Memory Architecture](#memory-architecture) section above for detailed discussion of memory management strategy.*

### Memory and Ownership
- **C**: Global static variables, single copy of all state, trail also global static
- **Rust**: Heap-allocated `SearchContext` with owned state, enables multiple independent instances for parallelization

### Ownership Model
- **C**: Implicit global ownership, careful conventions to avoid conflicts
- **Rust**: Explicit ownership via `SearchContext`, compiler-enforced safety, enables Send + Sync

### Null Handling
- **C**: NULL pointers with careful manual checking
- **Rust**: `Option<T>` with compiler-enforced checking

### Array Bounds
- **C**: Manual bounds checking, potential for bugs
- **Rust**: Automatic bounds checking, panics on overflow (checked in debug, unchecked in release for performance)

### Constants
- **C**: Compile-time NCOLORS via `-DNCOLORS=6` preprocessor flag
- **Rust**: Const generics where possible, or runtime configuration

### Parallelization
- **C**: Global statics prevent easy parallelization (would require process-level parallelism)
- **Rust**: Independent `SearchContext` instances enable thread-level parallelization at InnerFacePredicate boundary

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

#### Testing Memory Architecture

**Critical tests for parallelization readiness:**

```rust
#[test]
fn test_independent_search_contexts() {
    // Create two independent contexts
    let mut ctx1 = SearchContext::new();
    let mut ctx2 = SearchContext::new();

    // Modify ctx1's state
    ctx1.faces.set_some_value(/* ... */);
    ctx1.trail.checkpoint();

    // Verify ctx2 is completely unaffected
    assert!(ctx2.faces.is_unmodified());

    // Modify ctx2
    ctx2.faces.set_different_value(/* ... */);

    // Verify ctx1 still has its original changes
    assert_eq!(ctx1.faces.get_value(), /* expected */);
}

#[test]
fn test_memo_data_immutable() {
    let ctx = SearchContext::new();

    // Get pointer/reference to MEMO data
    let memo_ptr = &ctx.memo as *const _;

    // Perform search operations that modify DYNAMIC state
    // ... (checkpoint, modify faces, rewind, etc.)

    // MEMO data should be completely unchanged
    assert_eq!(&ctx.memo as *const _, memo_ptr);

    // Verify MEMO data contents unchanged
    // (check specific fields haven't been modified)
}

#[test]
fn test_context_clone_for_parallel() {
    let ctx1 = SearchContext::new();

    // Simulate creating context for parallel search
    let ctx2 = SearchContext::new();  // Or ctx1.clone_for_parallel() if implemented

    // Both should have access to same MEMO data (by copy or reference)
    assert_eq!(ctx1.memo.some_lookup_value(), ctx2.memo.some_lookup_value());

    // But independent DYNAMIC state
    ctx1.trail.checkpoint();
    ctx2.trail.checkpoint();

    // Modify ctx1
    ctx1.faces.set_value(/* ... */);

    // ctx2 should be unaffected
    assert_ne!(ctx1.faces.get_value(), ctx2.faces.get_value());
}

#[test]
fn test_memory_overhead_per_context() {
    use std::mem::size_of;

    // Measure SearchContext size
    let ctx_size = size_of::<SearchContext>();

    // Should be reasonable (few KB for DYNAMIC state)
    // MEMO data size should be documented
    println!("SearchContext size: {} bytes", ctx_size);

    // If using copies of MEMO data, total should be acceptable
    // for ~10-20 parallel threads
}
```

**Testing guidelines:**
- Always test multiple independent `SearchContext` instances
- Verify MEMO data is truly immutable
- Test that trail rewind doesn't affect other contexts
- Measure memory overhead per context
- Verify Send + Sync traits if parallelization implemented

## Performance Considerations

The C implementation is highly optimized:
- Trail-based backtracking for O(1) state restoration
- Careful memory layout for cache efficiency
- Minimal allocations during search

**Rust goals**:
- Match or exceed C performance (single-threaded)
- Enable 5-10x speedup via parallelization (future)
- Profile and optimize hot paths
- Use inline annotations where beneficial
- Consider unsafe blocks for critical paths (only after profiling shows benefit)

### Memory Model Performance

**Heap allocation overhead:**

| Strategy | Cost | Benefit |
|----------|------|---------|
| One-time SearchContext allocation | ~few microseconds at startup | Enables multiple independent contexts |
| Pre-allocated Vec capacities | Zero ongoing cost | Avoids reallocation during search |
| MEMO data per context (if copied) | ~100KB-1MB per context | Simple, good cache locality |
| MEMO data `&'static` (if referenced) | Zero per context | Shared across all contexts |

**Expected performance:**
- Heap allocation: Negligible (~0.001% of search time)
- Pre-allocated capacities: Zero reallocation overhead
- MEMO data copying (if <1MB): ~few microseconds, excellent cache locality
- Overall: Should match C single-threaded performance within measurement error

**Parallelization benefit far outweighs any overhead:**
- Serial overhead: < 0.01% (heap allocation, context setup)
- Parallel speedup: 5-10x (with ~10-20 independent searches)
- Net benefit: ~500-1000x improvement over any serial overhead

**Decision on MEMO data strategy:**
- Measure `size_of::<MemoizedData>()` during Phase 1
- If < 1MB: Copy per context (simpler, good cache locality)
- If > 1MB: Use `&'static` via `Box::leak()` (zero copy overhead)
- Either way: Performance should be excellent

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
- Complex algorithms should reference [Carroll 2000] or relevant sections in [docs/MATH.md](docs/MATH.md)
- Cite specific C implementation functions when porting (see [docs/DESIGN.md](docs/DESIGN.md) for C code organization)
- Explain non-obvious type safety invariants
- For geometric concepts and mathematical background, link to [docs/MATH.md](docs/MATH.md)
- For test expectations and validation approach, reference [docs/TESTS.md](docs/TESTS.md)
- **IMPORTANT**: Do not refer to "the C implementation" or "the C code" in Rust program internals (source code, doc comments, inline documentation). The Rust implementation should stand on its own. References to `c-reference/` files are acceptable in CLAUDE.md, commit messages, and migration documentation, but not in the Rust codebase itself.

## Advanced Rust Patterns to Explore

Since you're practicing advanced Rust:
- **Const generics** for compile-time NCOLORS validation
- **Trait objects** vs **static dispatch** for predicates
- **Unsafe** for performance-critical trail operations (carefully)
- **Iterators** for search space exploration
- **Zero-cost abstractions** for geometric types
- **Type states** for search phase tracking

## References

### Primary References

[Carroll 2000]: Carroll, Jeremy J. "Drawing Venn triangles." HP LABORATORIES TECHNICAL REPORT HPL-2000-73 (2000). [PDF](https://shiftleft.com/mirrors/www.hpl.hp.com/techreports/2000/HPL-2000-73.pdf)

### Additional Resources

- **C implementation**: https://github.com/roll/venntriangles (releases: [v1.0](https://github.com/roll/venntriangles/releases/tag/v1.0), [v1.1-pco](https://github.com/roll/venntriangles/releases/tag/v1.1))
- **C reference copy**: `c-reference/` directory (25 .c files, 22 .h files from v1.1-pco)
- **Design documentation**: [docs/DESIGN.md](docs/DESIGN.md) - Detailed architecture, engine, predicates, naming conventions
- **Mathematical theory**: [docs/MATH.md](docs/MATH.md) - Venn diagrams, FISCs, isomorphism, pseudolines, and additional references:
  - Bultena, Bette, Branko Grünbaum, and Frank Ruskey. "Convex drawings of intersecting families of simple closed curves." CCCG. 1999.
  - Grünbaum, Branko. "The importance of being straight." Proc. 12th Biannual Intern. Seminar of the Canadian Math. Congress. 1970.
  - Felsner, Stefan, and Jacob E. Goodman. "Pseudoline arrangements." Handbook of Discrete and Computational Geometry. 2017.
- **Expected results**: [docs/RESULTS.md](docs/RESULTS.md) - 233 solutions, 1.7M variations, performance data
- **Test documentation**: [docs/TESTS.md](docs/TESTS.md) - Visual test cases for 3-, 4-, 5-, 6-Venn diagrams
- **Visual examples**: `/images` directory in C implementation (diagrams for understanding concepts)
