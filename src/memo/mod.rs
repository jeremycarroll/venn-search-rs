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
//! # Construction
//!
//! `MemoizedData::new` eagerly constructs a fresh set of tables for each context:
//! 1. Generate cycles in their fixed ID order and initialize cycle lookup tables.
//! 2. Build faces, reversed edges and monotonicity/adjacency tables.
//! 3. Build vertex configurations and populate their incoming edges.
//! 4. Link each face edge to its possible vertices.
//!
//! Small fixed tables are stored inline; cycles and faces use Vec, and the
//! sparse vertex grid uses Box. Clone makes independent copies of these owned
//! allocations. Search reads the completed tables without modifying them.
//!
//! # Contents
//!
//! - **CyclesArray**: All possible facial cycles (394 for NCOLORS=6)
//! - **CyclesMemo**: Cycle constraint lookup tables (pairs, triples, omitting)
//! - **FacesMemo**: Faces, reversed edges, adjacency and monotonicity constraints
//! - **VerticesMemo**: Possible crossing configurations and lookup by vertex ID

pub mod cycles;
pub mod faces;
pub mod vertices;

pub use cycles::{CyclesArray, CyclesMemo};
pub use faces::FacesMemo;
pub use vertices::VerticesMemo;
