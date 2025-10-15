// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Geometric types for Venn diagrams.
//!
//! This module contains type-safe representations of geometric primitives:
//! - Color: Edge labels (0..NCOLORS-1)
//! - ColorSet: Bitset of colors
//! - Cycle: Sequences of edge colors around faces
//! - CycleSet: Bitset of cycle IDs
//! - Edge: Directed, labeled sides of faces
//! - Vertex: Oriented meeting points of curves
//! - Face: Regions bounded by cycles

pub mod color;
pub mod color_set;
pub mod constants;
pub mod cycle;
pub mod cycle_set;
pub mod edge;
pub mod face;
pub mod vertex;

// Re-export for convenience
pub use color::Color;
pub use color_set::ColorSet;
pub use constants::*;
pub use cycle::{Cycle, CycleId};
pub use cycle_set::CycleSet;
pub use edge::{Edge, EdgeId};
pub use face::{Face, FaceId};
pub use vertex::{Vertex, VertexId};
