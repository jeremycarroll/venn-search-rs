// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Tier 2: DYNAMIC state (mutable, tracked on trail).
//!
//! This module will contain all mutable search state:
//! - Faces state (current facial cycle assignments)
//! - EdgeColorCount (crossing counts)
//! - Other mutable search variables
//!
//! All state modifications are tracked on the trail for O(1) backtracking.
//!
//! To be implemented during Phase 3-4.
