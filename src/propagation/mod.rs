// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Constraint propagation for Venn diagram search.
//!
//! This module implements the cascading constraint propagation algorithm that prunes
//! the search space from ~10^150 configurations to a tractable size.
//!
//! # Algorithm Overview
//!
//! When a face is assigned a cycle:
//! 1. Eliminate incompatible cycles from related faces
//! 2. If any face reduces to **exactly 1 possible cycle**, automatically assign it
//! 3. Recursively propagate that new assignment (CASCADE)
//! 4. Fail immediately if any face has **zero possible cycles**
//!
//! This cascading effect (step 2-3) is what makes the search tractable.
//!
//! # Constraint Types
//!
//! **Edge Adjacency** (uses `cycle_pairs`, `cycle_triples`):
//! - Faces sharing an edge must have compatible cycles
//! - Example: If face uses cycle with edge a→b, then face across that edge
//!   must also have a cycle containing edge a→b
//!
//! **Non-Adjacent Faces** (uses `cycles_omitting_one_color`):
//! - Faces that don't share a color must use cycles omitting that color
//! - Example: If face uses cycle with colors {a,b,c}, then the face adjacent
//!   only through color d must use a cycle omitting d
//!
//! **Non-Vertex-Adjacent Faces** (uses `cycles_omitting_color_pair`):
//! - Faces that don't share a vertex must use cycles omitting certain edges
//! - Example: If cycle doesn't contain edge i→j, then doubly-adjacent face
//!   must use a cycle omitting edge i→j
//!
//! # Depth Tracking
//!
//! The `depth` parameter tracks recursion depth for:
//! - Debugging (failure messages show where constraint originated)
//! - Stack overflow prevention (depth ≤ NFACES = 64)
//! - Statistics (how deep cascades go)
//!
//! # Module Organization
//!
//! - `errors` - PropagationFailure error types
//! - `core` - Main orchestration functions (propagate_cycle_choice, restrict_face_cycles)
//! - `adjacency` - Edge adjacency constraint propagation
//! - `non_adjacency` - Non-adjacent and non-vertex-adjacent face constraints
//! - `vertices` - Vertex configuration and crossing detection
//! - `corners` - Corner detection and disconnected curve checking
//! - `color_removal` - Completed color removal optimization
//! - `setup` - Central face configuration
//! - `validation` - Post-search validation

// Submodules
mod adjacency;
mod color_removal;
mod core;
mod corners;
mod errors;
mod non_adjacency;
mod setup;
mod validation;
mod vertices;

// Re-exports
pub use core::{propagate_cycle_choice, restrict_face_cycles};
pub use errors::PropagationFailure;
pub use setup::setup_central_face;
pub use validation::validate_face_cycles;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::SearchContext;
    use crate::geometry::constants::NCYCLES;

    #[test]
    fn test_propagation_failure_display() {
        let fail1 = PropagationFailure::NoMatchingCycles {
            face_id: 5,
            depth: 2,
        };
        assert!(format!("{}", fail1).contains("Face 5"));
        assert!(format!("{}", fail1).contains("depth 2"));

        let fail2 = PropagationFailure::ConflictingConstraints {
            face_id: 10,
            assigned_cycle: 42,
            depth: 3,
        };
        assert!(format!("{}", fail2).contains("Face 10"));
        assert!(format!("{}", fail2).contains("cycle 42"));

        let fail3 = PropagationFailure::DepthExceeded { depth: 150 };
        assert!(format!("{}", fail3).contains("150"));
    }

    #[test]
    fn test_direction_tables_populated() {
        let ctx = SearchContext::new();

        // Check all cycles have non-empty direction tables
        for cycle_id in 0..NCYCLES as u64 {
            let cycle = ctx.memo.cycles.get(cycle_id);

            for i in 0..cycle.len() {
                let same_dir = cycle.same_direction(i);
                let opp_dir = cycle.opposite_direction(i);

                // Direction tables should have at least one cycle
                assert!(
                    !same_dir.is_empty(),
                    "Cycle {} edge {} has empty same_direction table",
                    cycle_id,
                    i
                );
                assert!(
                    !opp_dir.is_empty(),
                    "Cycle {} edge {} has empty opposite_direction table",
                    cycle_id,
                    i
                );
            }
        }
    }
}
