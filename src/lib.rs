// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Rust implementation of Venn triangle search algorithm.
//!
//! See <https://github.com/jeremycarroll/venntriangles> for the original implementation.
//! which searches for monotone simple 6-Venn diagrams drawable with six triangles.
//!
//! # Architecture
//!
//! The implementation uses a two-tier memory model:
//!
//! ## Tier 1: MEMO Data (Immutable)
//!
//! Precomputed data that never changes during search:
//! - Facial cycle constraint lookup tables
//! - Possible vertex configurations (480 entries for N=6)
//! - Edge and face relationship tables
//!
//! ## Tier 2: DYNAMIC Data (Mutable)
//!
//! Search state that changes during search, tracked on the trail:
//! - Trail - records state changes for O(1) backtracking
//! - Faces - current facial cycle assignments
//! - EdgeColorCount - crossing counts
//!
//! # Search Algorithm
//!
//! The search proceeds in three phases:
//!
//! 1. **InnerFacePredicate**: Find maximal 5-face degree signatures (~10-20 solutions)
//! 2. **VennPredicate**: For each signature, find valid facial cycle assignments
//! 3. **CornersPredicate**: For each Venn diagram, find valid corner mappings
//!
//! Each phase uses trail-based backtracking for efficient search.
//!
//! # Parallelization
//!
//! The architecture supports parallelization at the InnerFacePredicate boundary:
//! - Single-threaded initialization computes MEMO data
//! - Single-threaded InnerFacePredicate finds ~10-20 degree signatures
//! - Each degree signature spawns independent parallel search (Venn + Corners + GraphML)
//!
//! Expected speedup: 5-10x on modern multi-core systems.
//!
//! # References
//!
//! - Carroll, J. J. (2000). "Drawing Venn triangles." HP Laboratories Technical Report HPL-2000-73.
//!   <https://shiftleft.com/mirrors/www.hpl.hp.com/techreports/2000/HPL-2000-73.pdf>

pub mod context;
pub mod engine;
pub mod geometry;
pub mod memo;
pub mod predicates;
pub mod state;
pub mod trail;

// Re-export commonly used types
pub use context::SearchContext;
pub use engine::{Predicate, PredicateResult, SearchEngine};
pub use trail::Trail;
