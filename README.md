# venn-search-rs

Rust implementation of a search algorithm for monotone simple 6-Venn diagrams drawable with six triangles.

## Overview

This program searches for Venn diagrams of six triangles, as described in Carroll (2000). It finds monotone simple 6-Venn diagrams that satisfy necessary conditions for being stretchable into a diagram drawn with six triangles.

The search uses a non-deterministic backtracking algorithm with an efficient trail-based state management system.

## Status

✅ **Phase 7 Complete** - Core Venn diagram search is working!

- **Results**: Finds all 233 solutions for N=6 in ~3.5 seconds
- **Features**: Full constraint propagation, canonicality filtering under D₆ symmetry
- **Tests**: All passing for N=3 (2 solutions), N=4 (3), N=5 (23), N=6 (233)
- **Next**: Code cleanup and Phase 8 (corner detection)
- **C Reference**: See `c-reference/` directory or the [original C repo](https://github.com/jeremycarroll/venntriangles) (tag v1.1-pco)

## Building

Requires Rust 2021 edition or later.

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run the search
cargo run --release -- -f ../results
```

## Algorithm

The search is divided into three main phases:

1. **Degree Signature**: Finding maximal sequences of 6 integers that form valid 5-face degree signatures
2. **Facial Cycles**: Finding 64 facial cycles defining a Venn diagram with this signature
3. **Corner Mapping**: Finding edge-to-corner mappings where every pair of lines crosses at most once

The algorithm uses:
- **Trail-based backtracking**: Efficient O(1) state restoration during search
- **Predicate-based search**: Modular search phases with independent predicates
- **Partial Cyclic Orders (PCO)**: Ensuring proper geometric constraints on line crossings

## Output

Results are written in GraphML format, defining a planar graph labeled to show 18 pseudoline segments in six sets of three.

## Command Line Options

```bash
# Output to a specific directory
cargo run --release -- -f DIRECTORY

# Find solutions with specific 5-face degree sequence
cargo run --release -- -f DIRECTORY -d 664443

# Find just Venn diagram solutions without variants
cargo run --release -- -f DIRECTORY -n 1 -j 1
```

## Architecture

Key components:

- **Trail System**: Efficient backtracking via recorded state changes
- **Search Engine**: Generic non-deterministic search framework
- **Geometric Types**: Type-safe representations of colors, edges, vertices, faces
- **Predicates**: Modular search phases (Initialize, InnerFace, Venn, Corners)
- **Alternating Operators**: Generalized framework for PCO and Chirotopes

See [CLAUDE.md](CLAUDE.md) for detailed architecture documentation.

## Development

```bash
# Run all tests
cargo test

# Run clippy for linting
cargo clippy

# Format code
cargo fmt

# Build documentation
cargo doc --open
```

## References

- Carroll, J.J. (2000). "Drawing Venn triangles." HP LABORATORIES TECHNICAL REPORT HPL-2000-73. [PDF](https://shiftleft.com/mirrors/www.hpl.hp.com/techreports/2000/HPL-2000-73.pdf)
- Original C implementation: https://github.com/jeremycarroll/venntriangles

## License

See [LICENSE](LICENSE) file for details.

## Images

The `images/` directory contains visual explanations of key concepts used in the search algorithm.
