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
//! - **Tier 1 (MEMO)**: Immutable, computed once, can be shared across parallel searches
//! - **Tier 2 (DYNAMIC)**: Mutable, tracked on trail for backtracking
//!
//! # Contents
//!
//! - **Faces**: All face relationship tables, cycle constraints, adjacency lookups
//! - **Vertices**: All possible vertex configurations and crossing points
//!
//! # Size
//!
//! For NCOLORS=6:
//! - Faces: ~400 KB (64 faces × relationship tables)
//! - Vertices: ~30 KB (480 vertices)
//! - Total: ~500 KB - 1 MB
//!
//! This size is reasonable for copying per SearchContext, providing good
//! cache locality without excessive memory overhead.

pub mod faces;
pub mod vertices;

pub use faces::FacesMemo;
pub use vertices::VerticesMemo;
