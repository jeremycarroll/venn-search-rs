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
    /// Encoded as u64 for trail tracking: 0 = None, n+1 = Some(n)
    ///
    /// **0** = unassigned face (used in face selection heuristic)
    /// **n+1** = assigned to cycle n (either by choice or forcing)
    ///
    /// # Trail semantics (different depending on WHO sets it):
    ///
    /// 1. **try_pred**: Trail-sets to 0/None (reset on backtrack)
    ///
    /// 2. **retry_pred**: Sets directly WITHOUT trail (iterator cursor)
    ///    - Not trail-tracked, otherwise it would get unset before the next retry.
    ///
    /// 3. **Constraint propagation**: Trail-sets to n+1/Some(n) (forced assignment)
    ///    - Uses `ctx.set_face_cycle()` wrapper
    pub(crate) current_cycle_encoded: u64,

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
            current_cycle_encoded: 0, // None
            possible_cycles,
            cycle_count,
        }
    }

    /// Get current cycle (decodes from u64).
    #[inline]
    pub fn current_cycle(&self) -> Option<CycleId> {
        if self.current_cycle_encoded == 0 {
            None
        } else {
            Some(self.current_cycle_encoded - 1)
        }
    }

    /// Set current cycle (encodes to u64).
    ///
    /// This is for direct assignment (NOT trail-tracked).
    /// Use SearchContext::reset_face_cycle() or set_face_cycle() for trail-tracked updates.
    #[inline]
    pub fn set_current_cycle(&mut self, cycle: Option<CycleId>) {
        self.current_cycle_encoded = match cycle {
            None => 0,
            Some(id) => id + 1,
        };
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
