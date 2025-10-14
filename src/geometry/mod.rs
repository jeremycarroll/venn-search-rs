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

// Re-export for convenience
pub use color::Color;
