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
//! - **Heap**: Large or variable-size data (Vec for faces, Box for 3D vertex array)
//!
//! Heap allocation rationale:
//! - Faces Vec: Adapts to NCOLORS (8 faces for N=3, 64 for N=6)
//! - Vertices Box: 3D array too large for stack (147 KB would overflow)
//! - Total heap: ~152 KB, well within acceptable limits for per-context copying
//!
//! # Contents
//!
//! - **Cycles**: All possible facial cycles (394 for NCOLORS=6)
//! - **Faces**: All face relationship tables, cycle constraints, adjacency lookups
//! - **Vertices**: All possible vertex configurations and crossing points
//!
//! # Size
//!
//! Measured for NCOLORS=6 (Phase 6 baseline):
//! - FacesMemo: ~5 KB (64 Face structs + 56-byte binomial array)
//! - VerticesMemo: ~147 KB (64×6×6 Option<Vertex> array)
//! - **Total: ~149 KB**
//!
//! Future phases will add more MEMO data (cycle constraints, edge tables, etc.),
//! with expected final size of ~200-300 KB.
//!
//! This size is excellent for copying per SearchContext, providing good
//! cache locality without excessive memory overhead.

pub mod cycles;
pub mod faces;
pub mod vertices;

pub use cycles::CyclesArray;
pub use faces::FacesMemo;
pub use vertices::VerticesMemo;
