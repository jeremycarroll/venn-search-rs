// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Corner detection and disconnected curve checking.
//!
//! This module implements the Carroll 2000 corner detection algorithm
//! and checks for disconnected curves. For Venn diagrams drawable with
//! triangles, each curve can have at most 3 corners and must form a
//! single connected component.
