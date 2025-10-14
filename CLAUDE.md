# CLAUDE.md

This file provides guidance to Claude Code when working with this Rust rewrite of the venntriangles C codebase.

## Quick Start

```bash
# Build the project
cargo build --release

# Run tests
cargo test

# Run the venn search program
cargo run --release -- -f ../results

# Find solutions with specific 5-face degree sequence
cargo run --release -- -f ../results -d 664443

# Find just Venn diagram solutions without variants
cargo run --release -- -f ../results -n 1 -j 1

# Run clippy for linting
cargo clippy

# Format code
cargo fmt
```

## Migration Status

This is a Rust rewrite of the C implementation at ../venntriangles (tag: v1.1-pco).

**Migration approach**: Incremental port with architecture improvements, leveraging Rust's type system while preserving the proven search algorithm.

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
