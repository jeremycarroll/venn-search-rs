// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! VennPredicate - Main Venn diagram search.
//!
//! This predicate searches for facial cycle assignments across all NFACES faces.

use crate::context::SearchContext;
use crate::engine::{Predicate, PredicateResult};
use crate::geometry::constants::{NCOLORS, NFACES};
use crate::propagation;

/// VennPredicate finds valid facial cycle assignments.
///
/// Runs for up to NFACES rounds (0..NFACES), choosing a facial cycle for each face.
/// Uses a fail-fast heuristic: always chooses the face with fewest remaining possible cycles.
///
/// # Algorithm (Skeleton - full implementation in PR #2-4)
///
/// 1. **try_pred(round)**:
///    - Find next unassigned face with minimum possible cycles
///    - If no face found → Success (all faces assigned)
///    - Store face ID for this round
///    - Trail-set current_cycle to None (restored on backtrack)
///    - Return Choices(cycle_count)
///
/// 2. **retry_pred(round, choice)**:
///    - Get face ID from round
///    - Choose next cycle from possible_cycles using current_cycle as cursor
///    - Set current_cycle directly (NOT trail-tracked, iterator usage)
///    - [PR #2] Propagate constraints
///    - [PR #2] Check for immediate failure
///    - Return SuccessSamePredicate
#[derive(Debug)]
pub struct VennPredicate {
    /// Faces chosen at each round (indexed by round number).
    /// Stack-allocated [usize; NFACES] for O(1) access.
    faces_in_order: [usize; NFACES],
}

impl VennPredicate {
    pub fn new() -> Self {
        Self {
            faces_in_order: [0; NFACES],
        }
    }
}

impl Default for VennPredicate {
    fn default() -> Self {
        Self::new()
    }
}

impl Predicate for VennPredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, round: usize) -> PredicateResult {
        // For NCOLORS > 4, set up central face at round 0 if not already done
        #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
        if round == 0 {
            let inner_face_id = NFACES - 1;
            if ctx.state.faces.faces[inner_face_id].current_cycle().is_none() {
                // Not yet set up (InnerFacePredicate not run or didn't set it up)
                // Set up with no restrictions (all zeros)
                let no_restrictions = [0u64; NCOLORS];
                if let Err(_failure) = propagation::setup_central_face(
                    &ctx.memo,
                    &mut ctx.state,
                    &mut ctx.trail,
                    &no_restrictions,
                ) {
                    return PredicateResult::Failure;
                }
            }

            // S6 symmetry check - check canonicality BEFORE starting search
            // This prunes non-canonical branches early, mirroring C code behavior
            use crate::symmetry::s6::{check_solution_canonicality, SymmetryType};

            match check_solution_canonicality(&ctx.state, &ctx.memo) {
                SymmetryType::Canonical | SymmetryType::Equivocal => {
                    // Accept - this is a potentially valid canonical starting point
                }
                SymmetryType::NonCanonical => {
                    // Reject - this configuration is non-canonical, prune this branch
                    return PredicateResult::Failure;
                }
            }
        }

        // Find next unassigned face with minimum cycle count
        let face_id = choose_next_face(ctx);

        if let Some(face_id) = face_id {
            // Store face for retry_pred
            self.faces_in_order[round] = face_id;

            // Trail-set current_cycle to None (will be restored on backtrack)
            ctx.reset_face_cycle(face_id);

            // Get number of choices
            let cycle_count = ctx.get_face_cycle_count(face_id);

            PredicateResult::Choices(cycle_count as usize)
        } else {
            // All faces assigned - run final validation checks

            // 1. Validate face cycles (faces with M colors form single cycle of length C(NCOLORS, M))
            if let Err(_failure) = propagation::validate_face_cycles(&ctx.memo, &ctx.state) {
                return PredicateResult::Failure;
            }

            // S6 check happens at round 0 (before search starts), not here at the end
            // This matches the C code structure and prunes branches early

            PredicateResult::Success
        }
    }

    fn retry_pred(
        &mut self,
        ctx: &mut SearchContext,
        round: usize,
        _choice: usize,
    ) -> PredicateResult {
        let face_id = self.faces_in_order[round];

        // Get current_cycle to use as iterator cursor
        let current_cycle = ctx.state.faces.faces[face_id].current_cycle();

        // Choose next cycle from possible_cycles
        let next_cycle = choose_next_cycle(ctx, face_id, current_cycle);

        // Set current_cycle directly (NOT trail-tracked, iterator usage)
        // Not trail-tracked, otherwise it would get unset before the next retry.
        ctx.state.faces.faces[face_id].set_current_cycle(Some(next_cycle));

        // Constraint propagation
        if let Err(_failure) = propagation::propagate_cycle_choice(
            &ctx.memo,
            &mut ctx.state,
            &mut ctx.trail,
            face_id,
            next_cycle,
            0,
        ) {
            // Propagation failed - engine will backtrack
            return PredicateResult::Failure;
        }

        PredicateResult::SuccessSamePredicate
    }

    fn name(&self) -> &str {
        "Venn"
    }
}

/// Choose next unassigned face with minimum cycle count (fail-fast heuristic).
fn choose_next_face(ctx: &SearchContext) -> Option<usize> {
    let mut min_count = u64::MAX;
    let mut best_face = None;

    for face_id in 0..NFACES {
        let face = &ctx.state.faces.faces[face_id];

        // Skip if already assigned (current_cycle != None)
        if face.current_cycle().is_some() {
            continue;
        }

        if face.cycle_count < min_count {
            min_count = face.cycle_count;
            best_face = Some(face_id);
        }
    }

    best_face
}

/// Choose next cycle from possible_cycles using current as cursor.
///
/// If current is None, returns first cycle.
/// Otherwise, returns next cycle after current.
fn choose_next_cycle(ctx: &SearchContext, face_id: usize, current: Option<u64>) -> u64 {
    let possible_cycles = ctx.get_face_possible_cycles(face_id);

    if let Some(current_id) = current {
        // Find next cycle after current
        // TODO(optimization): This can be optimized. The iterator iterates over bits
        // in the CycleSet bitset. We can use the cursor (current_id) to jump directly
        // to the correct u64 word in the bitset (word_idx = current_id / 64), then
        // iterate only over the remaining bits in that word and subsequent words,
        // rather than using find which checks every bit from the start.
        possible_cycles
            .iter()
            .find(|&id| id > current_id)
            .expect("No more cycles available")
    } else {
        // First cycle
        possible_cycles.iter().next().expect("No cycles available")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_venn_predicate_creation() {
        let pred = VennPredicate::new();
        assert_eq!(pred.name(), "Venn");
        assert_eq!(pred.faces_in_order.len(), NFACES);
    }

    #[test]
    fn test_dynamic_faces_initialization() {
        let ctx = SearchContext::new();

        // All faces should have dynamic state
        assert_eq!(ctx.state.faces.faces.len(), NFACES);

        // All faces should start with no assigned cycle
        for face_id in 0..NFACES {
            let face = ctx.state.faces.get(face_id);
            assert!(face.current_cycle().is_none());
        }
    }

    #[test]
    fn test_choose_next_face_simple() {
        let ctx = SearchContext::new();

        // Should choose a face (exact face depends on MEMO initialization)
        let face_id = choose_next_face(&ctx);
        assert!(face_id.is_some());
    }

    #[test]
    fn test_choose_next_face_skip_assigned() {
        let mut ctx = SearchContext::new();

        // Assign a cycle to face 0
        ctx.state.faces.faces[0].set_current_cycle(Some(0));

        // Should choose a different face
        let face_id = choose_next_face(&ctx);
        assert!(face_id.is_some());
        assert_ne!(face_id.unwrap(), 0);
    }

    #[test]
    fn test_choose_next_cycle_first() {
        let ctx = SearchContext::new();

        // Find a face with at least one possible cycle
        let face_id = choose_next_face(&ctx).expect("Should have unassigned face");

        // Choose first cycle (current = None)
        let first_cycle = choose_next_cycle(&ctx, face_id, None);

        // Should be in possible_cycles
        assert!(ctx.get_face_possible_cycles(face_id).contains(first_cycle));
    }

    #[test]
    fn test_choose_next_cycle_iterator() {
        let ctx = SearchContext::new();

        // Find a face with multiple possible cycles
        let face_id = (0..NFACES)
            .find(|&id| ctx.get_face_cycle_count(id) >= 2)
            .expect("Should have face with multiple cycles");

        // Choose first cycle
        let first = choose_next_cycle(&ctx, face_id, None);

        // Choose next cycle
        let second = choose_next_cycle(&ctx, face_id, Some(first));

        // Should be different
        assert_ne!(first, second);
        assert!(ctx.get_face_possible_cycles(face_id).contains(second));
    }

    #[test]
    fn test_venn_try_pred_round_0() {
        let mut ctx = SearchContext::new();
        let mut pred = VennPredicate::new();

        let result = pred.try_pred(&mut ctx, 0);

        // Should return Choices(N) for some N > 0
        match result {
            PredicateResult::Choices(n) => assert!(n > 0),
            _ => panic!("Expected Choices, got {:?}", result),
        }
    }

    #[test]
    fn test_venn_try_pred_resets_cycle() {
        let mut ctx = SearchContext::new();
        let mut pred = VennPredicate::new();

        // Run try_pred to choose a face
        pred.try_pred(&mut ctx, 0);

        // After try_pred, the chosen face should have current_cycle = None
        let face_id = pred.faces_in_order[0];
        assert!(ctx.state.faces.faces[face_id].current_cycle().is_none());
    }

    #[test]
    fn test_venn_retry_pred_assigns_cycle() {
        let mut ctx = SearchContext::new();
        let mut pred = VennPredicate::new();

        // Run try_pred first to set up state
        pred.try_pred(&mut ctx, 0);

        // Run retry_pred
        let result = pred.retry_pred(&mut ctx, 0, 0);

        // May succeed or fail depending on constraint propagation
        // (with corner detection, early cycles may violate crossing limits)
        match result {
            PredicateResult::SuccessSamePredicate => {
                // If it succeeded, should have assigned a cycle
                let face_id = pred.faces_in_order[0];
                assert!(ctx.state.faces.faces[face_id].current_cycle().is_some());
            }
            PredicateResult::Failure => {
                // Propagation failed (e.g., crossing limit exceeded) - this is OK
                // The engine will backtrack and try another cycle
            }
            _ => panic!("Expected SuccessSamePredicate or Failure, got {:?}", result),
        }
    }

    #[test]
    fn test_cycle_count_matches_possible_cycles() {
        let ctx = SearchContext::new();

        for face_id in 0..NFACES {
            let face = ctx.state.faces.get(face_id);
            let expected_count = face.possible_cycles.len() as u64;
            assert_eq!(face.cycle_count, expected_count);
        }
    }
}
