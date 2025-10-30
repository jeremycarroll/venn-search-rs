// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Tier 2: DYNAMIC state (mutable, tracked on trail).
//!
//! This module contains all mutable search state:
//! - Edge state (current vertex connections)
//! - Faces state (current facial cycle assignments)
//! - EdgeColorCount (crossing counts)
//! - Other mutable search variables
//!
//! All state modifications are tracked on the trail for O(1) backtracking.

pub mod edge;
pub mod faces;
pub mod statistics;

pub use edge::EdgeDynamic;
pub use faces::{DynamicFace, DynamicFaces};
pub use statistics::Statistics;
