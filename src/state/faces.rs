// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Dynamic face state structures.
//!
//! This module provides mutable per-face state that changes during search
//! and is tracked on the trail for backtracking.

use crate::geometry::constants::{NCOLORS, NCYCLES};
use crate::geometry::{CycleId, CycleSet};
use crate::memo::FacesMemo;
use crate::state::DynamicEdge;

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

    /// DYNAMIC edge state for this face (one per color, trail-tracked).
    ///
    /// `edge_dynamic[i]` contains the runtime state for edge of color i on this face.
    /// This is the DYNAMIC part that pairs with EdgeMemo in Face.edges[i].
    ///
    /// The `to_encoded` field in each DynamicEdge is set during vertex propagation
    /// and must be trail-tracked for backtracking.
    pub edge_dynamic: [DynamicEdge; NCOLORS],

    /// Next face in the dual graph cycle (trail-tracked).
    ///
    /// Encoded as u64: 0 = unset, n+1 = face_id n
    ///
    /// Faces with M colors form a single cycle in the dual graph of length C(NCOLORS, M).
    /// This pointer links to the next face in that cycle.
    pub(crate) next_face_id_encoded: u64,

    /// Previous face in the dual graph cycle (trail-tracked).
    ///
    /// Encoded as u64: 0 = unset, n+1 = face_id n
    ///
    /// This pointer links to the previous face in the dual graph cycle.
    pub(crate) previous_face_id_encoded: u64,
}

impl DynamicFace {
    /// Create a new dynamic face from initial possible cycles.
    pub fn new(possible_cycles: CycleSet) -> Self {
        let cycle_count = possible_cycles.len() as u64;
        Self {
            current_cycle_encoded: 0, // None
            possible_cycles,
            cycle_count,
            edge_dynamic: [DynamicEdge::new(); NCOLORS],
            next_face_id_encoded: 0,     // unset
            previous_face_id_encoded: 0, // unset
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

    /// Get next face in dual graph cycle (decodes from u64).
    #[inline]
    pub fn next_face(&self) -> Option<usize> {
        if self.next_face_id_encoded == 0 {
            None
        } else {
            Some((self.next_face_id_encoded - 1) as usize)
        }
    }

    /// Get previous face in dual graph cycle (decodes from u64).
    #[inline]
    pub fn previous_face(&self) -> Option<usize> {
        if self.previous_face_id_encoded == 0 {
            None
        } else {
            Some((self.previous_face_id_encoded - 1) as usize)
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

/// Shared optional index encoding. Validate before adding the sentinel offset.
pub(crate) fn encode_optional_index(index: Option<u64>, limit: usize) -> u64 {
    match index {
        None => 0,
        Some(id) => {
            assert!(id < limit as u64, "Optional index {} out of range 0..{}", id, limit);
            id.checked_add(1).expect("Optional index overflow")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::constants::NFACES;

    #[test]
    fn optional_index_encoding_boundaries() {
        for limit in [NCYCLES, NFACES] {
            assert_eq!(encode_optional_index(None, limit), 0);
            assert_eq!(encode_optional_index(Some(0), limit), 1);
            assert_eq!(encode_optional_index(Some((limit - 1) as u64), limit), limit as u64);
            assert!(
                std::panic::catch_unwind(|| encode_optional_index(Some(limit as u64), limit)).is_err()
            );
        }
    }
}
