// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Vertex configuration and crossing detection.
//!
//! This module handles:
//! - Setting edge->to pointers to connect faces through vertices
//! - Tracking crossing counts between color pairs
//! - Enforcing the triangle constraint (max 6 crossings per pair)
