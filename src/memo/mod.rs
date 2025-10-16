// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Memoized (MEMO) data structures.
//!
//! This module contains all precomputed lookup tables and immutable data
//! computed once during initialization. These structures enable efficient
//! constraint propagation during Venn diagram search.
//!
//! # Architecture
//!
//! MEMO data is Tier 1 in the two-tier memory model:
//! - **Tier 1 (MEMO)**: Immutable, computed once, stored in `MemoizedData`
//! - **Tier 2 (DYNAMIC)**: Mutable, tracked on trail, stored in `DynamicState`
//!
//! Both tiers are owned by `SearchContext` and use mixed stack/heap allocation.
//!
//! # Memory Allocation
//!
//! MEMO structures use a mix of stack and heap allocation:
//! - **Stack**: Small fixed-size arrays (e.g., binomial coefficients: 7 × u64 = 56 bytes)
//!   - CyclesMemo lookup tables: ~16 KB (stack-allocated arrays)
//! - **Heap**: Large or variable-size data (Vec for cycles and faces, Box for 3D vertex array)
//!
//! Heap allocation rationale:
//! - CyclesArray Vec: Adapts to NCOLORS (2 for N=3, 394 for N=6)
//! - Faces Vec: Adapts to NCOLORS (8 faces for N=3, 64 for N=6)
//! - Next/Previous arrays: NFACES × NCYCLES too large for stack (64 × 394 ≈ 25 KB each)
//! - Vertices Box: 3D array too large for stack (147 KB would overflow)
//! - Total heap: ~214 KB, well within acceptable limits for per-context copying
//!
//! # Contents
//!
//! - **CyclesArray**: All possible facial cycles (394 for NCOLORS=6)
//! - **CyclesMemo**: Cycle constraint lookup tables (pairs, triples, omitting)
//! - **FacesMemo**: Face relationship tables, adjacency lookups, monotonicity constraints
//! - **VerticesMemo**: All possible vertex configurations and crossing points
//!
//! # Size
//!
//! Measured for NCOLORS=6 (Phase 6 complete):
//! - CyclesArray: ~12 KB (394 Cycle structs in Vec)
//! - CyclesMemo: ~16 KB (lookup tables: pairs, triples, omitting - stack-allocated)
//! - FacesMemo: ~55 KB (64 Face structs + 56-byte binomial array + 2 × 25 KB next/previous arrays)
//! - VerticesMemo: ~147 KB (64×6×6 Option<Vertex> array in Box)
//! - **Total: ~230 KB**
//!
//! Future phases will add more MEMO data (edge tables, PCO/Chirotope, etc.),
//! with expected final size of ~250-300 KB.
//!
//! This size is excellent for copying per SearchContext, providing good
//! cache locality without excessive memory overhead.

pub mod cycles;
pub mod faces;
pub mod vertices;

pub use cycles::{CyclesArray, CyclesMemo};
pub use faces::FacesMemo;
pub use vertices::VerticesMemo;
