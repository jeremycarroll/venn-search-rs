// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Geometric types for Venn diagrams.
//!
//! This module contains type-safe representations of geometric primitives:
//! - Color: Edge labels (0..NCOLORS-1)
//! - Cycle: Sequences of edge colors around faces
//! - Edge: Directed, labeled sides of faces
//! - Vertex: Oriented meeting points of curves
//! - Face: Regions bounded by cycles
//!
//! To be implemented during Phase 2.

pub mod color;
pub mod color_set;
pub mod constants;
pub mod cycle;
pub mod cycle_set;

// Re-export for convenience
pub use color::Color;
pub use color_set::ColorSet;
pub use constants::*;
pub use cycle::{Cycle, CycleId};
pub use cycle_set::CycleSet;
