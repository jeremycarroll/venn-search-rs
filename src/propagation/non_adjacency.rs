// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Non-adjacency constraint propagation.
//!
//! This module implements propagation of constraints between faces that
//! don't share edges or vertices:
//! - Faces not sharing a color must use cycles omitting that color
//! - Faces not sharing a vertex must use cycles omitting certain edges
