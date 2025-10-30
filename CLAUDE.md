# CLAUDE.md

This file provides guidance to Claude Code when working with this Rust rewrite of the venntriangles C codebase.

## Purpose and Audience

**Purpose:** This is a complete, self-contained research artifact. The goal is that a single motivated PhD student in the future can pick this up, understand it deeply, and build upon it.

**Not optimizing for:** GitHub stars, community building, broad adoption.

**Optimizing for:** One researcher who picks this up in 5 years and can fully reconstruct the reasoning.

**Documentation philosophy:**
- Stand completely alone (no broken links to external resources that may vanish)
- Explain the "why" as much as the "how" (mathematical foundations, design rationale)
- Visual aids and detailed explanations are about teaching, not just reference
- Code should be exemplary - showing how to think about the problem
- Design decisions should be documented (Prolog inspiration, trail architecture, memory model)

This context matters for anyone (AI or human) working on this project: we're creating an intellectual artifact that must be comprehensible decades from now, not optimizing for short-term convenience.

## Quick Start

```bash
# Build and test
cargo build --release
cargo test
cargo clippy
cargo fmt

# Run the search (Phase 7 complete - finds all 233 solutions for N=6)
# Note: CLI arguments and output generation (Phase 8+) not yet implemented
# cargo run --release -- -f ../results
# cargo run --release -- -f ../results -d 664443
# cargo run --release -- -f ../results -n 1 -j 1
```

## Migration Status

This is a Rust rewrite of the C implementation at ../venntriangles (tag: v1.1-pco).

**Migration approach**: Incremental port with architecture improvements, leveraging Rust's type system while preserving the proven search algorithm.

### Current Status (as of January 2025)

**Completed:**
- ✅ **Phase 1-6 Complete**: Core Infrastructure
  - Trail system, geometric types, search engine, MEMO data structures
  - All foundational components tested and working

- ✅ **Phase 7 Complete (Jan 2025)**: VennPredicate - Main Search
  - Non-deterministic search for facial cycle assignments
  - Full constraint propagation (edge adjacency, non-adjacent, non-vertex-adjacent)
  - Fail-fast heuristic (choose face with fewest options)
  - Solution validation (face cycles, dihedral symmetry)
  - **Canonicality check implemented**: Rejects non-canonical solutions under D₆
  - **Performance**: ~3.5 seconds to find all 233 solutions (NCOLORS=6)
  - **All tests passing**: N=3 (2), N=4 (3), N=5 (23), N=6 (233 solutions)

**Current Milestone**: Core search is complete and producing correct results!

**Next:**
- ⬜ **Code cleanup**: Review clarity, conciseness, documentation (see docs/CLEANUP.md)
- ⬜ **Phase 8**: Corner detection and graph realization
- ⬜ **Future**: GraphML output, PCO/Chirotope, CLI, parallelization

## Reference C Implementation

The original C implementation is at: https://github.com/roll/venntriangles

Key releases:
- Tag [`v1.0`](https://github.com/roll/venntriangles/releases/tag/v1.0): Original working Venn 6-triangle search
- Tag [`v1.1-pco`](https://github.com/roll/venntriangles/releases/tag/v1.1): Generalized alternating ternary operators (PCO, Chirotopes)

See `c-reference/` directory for copied C source files (from v1.1-pco tag).

## Architecture Overview

This program searches for monotone simple 6-Venn diagrams drawable with six triangles, as described in [Carroll 2000]. The search has three main phases:

1. Finding maximal sequences of 6 integers making a 5-face degree signature
2. Finding 64 facial cycles defining a Venn diagram with this signature
3. Finding an edge-to-corner mapping where every pair of lines crosses at most once

Output is in GraphML format defining a planar graph with 18 pseudoline segments in six sets of three.

**For detailed information**, see:
- **[docs/DESIGN.md](docs/DESIGN.md)** - Comprehensive design documentation
- **[docs/MATH.md](docs/MATH.md)** - Mathematical foundations
- **[docs/RESULTS.md](docs/RESULTS.md)** - Expected results: 233 solutions with 1.7M variations
- **[docs/TESTS.md](docs/TESTS.md)** - Test suite documentation

## Memory Architecture

**CRITICAL DESIGN DECISION**: Two-tier memory model for parallelization.

**Tier 1: MEMO Data (Immutable, Computed Once)**
- Facial cycle constraint lookup tables
- Possible vertex configurations (480 entries for N=6)
- All precomputed lookup tables
- **Strategy**: Compute once, owned by SearchContext (copy or &'static reference depending on size)

**Tier 2: DYNAMIC Data (Mutable, Per-Search)**
- `Trail` - records all state changes for O(1) backtracking
- `Faces` - current facial cycle assignments
- `EdgeColorCount` - crossing counts
- **Strategy**: Each SearchContext owns its mutable state, tracked on trail

**Current implementation**: Single-threaded. Architecture is parallelization-ready.

**Parallelization point** (future): After InnerFacePredicate finds each degree signature (~56 solutions for N=6), spawn independent search for each.

**Key architectural decisions**:
- Use heap allocation (not global statics) → enables multiple independent SearchContext instances
- Own state per context → enables Send + Sync for future parallelization
- Separate read-only MEMO from mutable DYNAMIC state
- No shared mutable state across threads

## Core Design Principles

### 1. Trail-Based Backtracking (CRITICAL)

The trail system is the core efficiency mechanism - **do not remove or simplify away**:

- **Purpose**: Efficient O(1) backtracking by recording state changes
- **Implementation**: `Vec<TrailEntry>` with index-based rewind, type-safe API
- **Usage**: Trail owned by `SearchContext`, passed explicitly as `&mut`

**Key insight**: The trail makes backtracking O(1) instead of O(n) and automatically handles all state restoration.

### 2. Non-Deterministic Search Engine

Implements backtracking search via predicates:
- Each predicate has `try_pred` (first attempt) and `retry_pred` (backtracking) methods
- Engine maintains a stack of predicate states
- Trail system handles all state restoration on backtrack
- **Critical**: When backtracking from Failure, must pop stack until finding a choice point (not just pop once)

### 3. Type-Safe Geometric Primitives

Use newtypes and enums for type safety:
- Colors (0..NCOLORS-1)
- Edges (directed, with reversal)
- Vertices (oriented meeting points)
- Faces (regions bounded by cycles)
- Cycles (sequences of edge colors)

## Next Steps

### Immediate: Code Cleanup and Review

See **[docs/CLEANUP.md](docs/CLEANUP.md)** for comprehensive review.

**Priority items:**
- Break up large mod.rs files
- Improve code clarity and documentation
- Simplify complex functions
- Remove debug code
- Review naming consistency

### Phase 8: Corner Detection and Graph Realization

Implement corner assignment and geometric realizability constraints:
- CornersPredicate: Assign 18 corners to edge endpoints
- Crossing count validation (max 6 per color pair for triangles)
- PCO (Partial Cyclic Orders) for line crossing constraints
- GraphML output generation

### Future Phases

- Chirotope support for oriented matroid testing
- CLI argument parsing and output management
- Parallelization (spawn searches per inner face degree)
- Performance optimization and profiling

## Key Differences from C

### Memory and Ownership
- **C**: Global static variables, single copy of all state
- **Rust**: Heap-allocated `SearchContext` with owned state, enables multiple independent instances

### Null Handling
- **C**: NULL pointers with careful manual checking
- **Rust**: `Option<T>` with compiler-enforced checking

### Parallelization
- **C**: Global statics prevent easy parallelization
- **Rust**: Independent `SearchContext` instances enable thread-level parallelization at InnerFacePredicate boundary

## Dependencies and Tooling

### Recommended Crates

**CLI and Configuration:**
- `clap` (v4.x) - Command line argument parsing
- `anyhow` - Ergonomic error handling

**Serialization/Output:**
- `quick-xml` or `xml-rs` - GraphML output generation

**Testing:**
- Built-in `cargo test` framework
- `proptest` - Property-based testing for geometric invariants
- `criterion` - Benchmarking framework

**Optional/Advanced:**
- `rayon` - Parallelization
- `tracing` - Structured logging
- `smallvec` - Stack-allocated vectors

### Development Tools

```bash
# Essential
cargo install cargo-edit      # cargo add/rm commands
rustup component add clippy   # Linting
rustup component add rustfmt  # Formatting

# Recommended
cargo install cargo-flamegraph # Performance profiling
```

## Testing Strategy

**Rust testing**:
- Port existing test cases from C as unit tests
- Add property-based tests where appropriate
- Ensure tests validate against known solutions
- Test both the trail system and engine independently

**Running tests**:
```bash
cargo test                          # All tests
cargo test --test trail_tests       # Specific test file
cargo test -- --nocapture           # With output
cargo test test_trail_checkpoint    # Single test
```

## Performance Considerations

**Rust goals**:
- Match or exceed C performance (single-threaded)
- Enable 5-10x speedup via parallelization (future)
- Profile and optimize hot paths

**Optimization priorities**:
1. **Trail operations** - Called millions of times
2. **Predicate try/retry** - Core search loop
3. **PCO closure** - Expensive constraint propagation
4. **Memory allocations** - Minimize in hot paths

**Performance testing**:
```bash
# Build with maximum optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Profile with flamegraph
cargo flamegraph --bin venn -- -f results -n 1
```

## Documentation Standards

- All public APIs must have doc comments
- Complex algorithms should reference [Carroll 2000] or relevant sections in docs/
- Explain non-obvious type safety invariants
- **IMPORTANT**: Do not refer to "the C implementation" or "the C code" in Rust program internals (source code, doc comments, inline documentation). The Rust implementation should stand on its own. References to `c-reference/` files are acceptable in CLAUDE.md, commit messages, and migration documentation only.

## Advanced Rust Patterns to Explore

- **Const generics** for compile-time NCOLORS validation
- **Trait objects** vs **static dispatch** for predicates
- **Unsafe** for performance-critical trail operations (carefully, only after profiling)
- **Iterators** for search space exploration
- **Zero-cost abstractions** for geometric types
- **Type states** for search phase tracking

## References

### Primary References

[Carroll 2000]: Carroll, Jeremy J. "Drawing Venn triangles." HP LABORATORIES TECHNICAL REPORT HPL-2000-73 (2000). [PDF](https://shiftleft.com/mirrors/www.hpl.hp.com/techreports/2000/HPL-2000-73.pdf)

### Additional Resources

- **C implementation**: https://github.com/roll/venntriangles (releases: [v1.0](https://github.com/roll/venntriangles/releases/tag/v1.0), [v1.1-pco](https://github.com/roll/venntriangles/releases/tag/v1.1))
- **C reference copy**: `c-reference/` directory (25 .c files, 22 .h files from v1.1-pco)
- **Design documentation**: [docs/DESIGN.md](docs/DESIGN.md) - Detailed architecture, engine, predicates
- **Mathematical theory**: [docs/MATH.md](docs/MATH.md) - Venn diagrams, FISCs, isomorphism, pseudolines, and additional references:
  - Bultena, Bette, Branko Grünbaum, and Frank Ruskey. "Convex drawings of intersecting families of simple closed curves." CCCG. 1999.
  - Grünbaum, Branko. "The importance of being straight." Proc. 12th Biannual Intern. Seminar of the Canadian Math. Congress. 1970.
  - Felsner, Stefan, and Jacob E. Goodman. "Pseudoline arrangements." Handbook of Discrete and Computational Geometry. 2017.
- **Expected results**: [docs/RESULTS.md](docs/RESULTS.md) - 233 solutions, 1.7M variations, performance data
- **Test documentation**: [docs/TESTS.md](docs/TESTS.md) - Test suite documentation with visual diagrams
