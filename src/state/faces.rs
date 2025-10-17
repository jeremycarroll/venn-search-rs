// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Dynamic face state structures.
//!
//! This module provides mutable per-face state that changes during search
//! and is tracked on the trail for backtracking.

use crate::geometry::{CycleId, CycleSet};
use crate::memo::FacesMemo;

/// Per-face dynamic state (mutable, trail-tracked).
#[derive(Debug)]
pub struct DynamicFace {
    /// Current cycle for this face - serves as both iterator cursor and assignment indicator.
    ///
    /// **None** = unassigned face (used in face selection heuristic)
    /// **Some(cycle_id)** = assigned face (either by choice or forcing)
    ///
    /// # Trail semantics (different depending on WHO sets it):
    ///
    /// 1. **try_pred**: Trail-sets to None (reset on backtrack)
    ///    - Matches C: `TRAIL_SET_POINTER(&face->cycle, NULL);`
    ///
    /// 2. **retry_pred**: Sets directly WITHOUT trail (iterator cursor)
    ///    - Matches C: `face->cycle = chooseCycle(face, face->cycle);`
    ///    - Comment in C: "Not on trail, otherwise it would get unset before the next retry."
    ///
    /// 3. **Constraint propagation** (PR #2): Trail-sets to Some(cycle_id) (forced assignment)
    ///    - Uses `ctx.force_face_cycle()` wrapper
    ///    - Matches C: `trailMaybeSetInt(&face->possibleCycles[i], ...)` in dynamicSetFaceCycleSetToSingleton
    pub current_cycle: Option<CycleId>,

    /// Set of possible cycles for this face (trail-tracked).
    /// Starts with all valid cycles, gets filtered by constraint propagation.
    pub possible_cycles: CycleSet,

    /// Count of possible cycles (trail-tracked, cached for performance).
    /// Updated whenever possible_cycles changes.
    pub cycle_count: u64,
}

impl DynamicFace {
    /// Create a new dynamic face from initial possible cycles.
    pub fn new(possible_cycles: CycleSet) -> Self {
        let cycle_count = possible_cycles.len() as u64;
        Self {
            current_cycle: None,
            possible_cycles,
            cycle_count,
        }
    }
}

/// All dynamic face state (NFACES faces).
#[derive(Debug)]
pub struct DynamicFaces {
    /// Per-face mutable state (heap-allocated Vec).
    pub faces: Vec<DynamicFace>,
}

impl DynamicFaces {
    /// Initialize from FacesMemo.
    ///
    /// Creates dynamic state for all faces, copying initial possible_cycles
    /// from the MEMO data.
    pub fn new(faces_memo: &FacesMemo) -> Self {
        let faces = faces_memo
            .faces
            .iter()
            .map(|memo_face| DynamicFace::new(memo_face.possible_cycles))
            .collect();

        Self { faces }
    }

    /// Get a dynamic face by ID.
    #[inline]
    pub fn get(&self, face_id: usize) -> &DynamicFace {
        &self.faces[face_id]
    }

    /// Get a mutable dynamic face by ID.
    #[inline]
    pub fn get_mut(&mut self, face_id: usize) -> &mut DynamicFace {
        &mut self.faces[face_id]
    }
}
