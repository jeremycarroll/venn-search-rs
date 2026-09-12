// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! InnerFacePredicate - Finds degree signatures for the NCOLORS symmetric faces.
//!
//! This predicate non-deterministically searches for sequences of face degrees
//! that sum to TOTAL_CENTRAL_NEIGHBOR_DEGREE and are canonical under S6 symmetry.

use crate::context::SearchContext;
use crate::engine::{Predicate, PredicateResult};
use crate::geometry::constants::{NCOLORS, TOTAL_CENTRAL_NEIGHBOR_DEGREE};
#[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
use crate::propagation;
use crate::symmetry::{check_symmetry, SymmetryType};

/// InnerFacePredicate finds valid degree signatures for the NCOLORS symmetric faces.
///
/// This predicate runs for NCOLORS rounds (0..NCOLORS), choosing a degree for each face.
/// On the final round (NCOLORS), it validates:
/// 1. Sum of degrees equals TOTAL_CENTRAL_NEIGHBOR_DEGREE
/// 2. Sequence is canonical or equivocal under S6 dihedral symmetry
///
/// # Choices
///
/// For each face, we try degrees from NCOLORS down to 3:
/// - Choice 0 → degree NCOLORS
/// - Choice 1 → degree NCOLORS-1
/// - ...
/// - Choice NCOLORS-3 → degree 3
///
/// This gives us (NCOLORS - 2) choices per face.
///
/// # Examples
///
/// For NCOLORS=6, we search for sequences of 6 degrees that:
/// - Each degree is in range [3, 6]
/// - Sum to 27
/// - Are canonical or equivocal under D_6 symmetry
///
/// Expected to find 56 canonical degree signatures.
#[derive(Debug)]
pub struct InnerFacePredicate;

impl Predicate for InnerFacePredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, round: usize) -> PredicateResult {
        if round == NCOLORS {
            // Final round - validate the complete sequence
            let degrees = ctx.get_face_degrees();

            // Check sum constraint
            let sum: u64 = degrees.iter().sum();
            if sum != TOTAL_CENTRAL_NEIGHBOR_DEGREE as u64 {
                return PredicateResult::Failure;
            }

            // Check S6 symmetry
            // Convert u64 array to u8 array for symmetry checking
            let degrees_u8: [u8; NCOLORS] = {
                let mut arr = [0u8; NCOLORS];
                for i in 0..NCOLORS {
                    arr[i] = degrees[i] as u8;
                }
                arr
            };

            let symmetry = check_symmetry(&degrees_u8);
            match symmetry {
                SymmetryType::NonCanonical => PredicateResult::Failure,
                SymmetryType::Canonical | SymmetryType::Equivocal => {
                    // Only set up central face for NCOLORS > 4
                    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
                    {
                        // Copy degrees array to avoid borrow checker issues
                        let degrees_copy = *degrees;
                        // Set up central face configuration before proceeding to VennPredicate
                        if let Err(_failure) = propagation::setup_central_face(
                            &ctx.memo,
                            &mut ctx.state,
                            &mut ctx.trail,
                            &degrees_copy,
                        ) {
                            // Setup failed - constraints are unsatisfiable for this degree signature
                            return PredicateResult::Failure;
                        }
                    }

                    PredicateResult::Success
                }
            }
        } else {
            // Generate choices for this round
            PredicateResult::Choices(NCOLORS - 2)
        }
    }

    fn retry_pred(
        &mut self,
        ctx: &mut SearchContext,
        round: usize,
        choice: usize,
    ) -> PredicateResult {
        // Map choice to degree: 0→NCOLORS, 1→NCOLORS-1, ..., (NCOLORS-3)→3
        let degree = (NCOLORS - choice) as u64;

        // Invariant: The engine guarantees choice is in range [0, NCOLORS-3], which
        // ensures degree is in [3, NCOLORS]. This assertion catches programming errors
        // if retry_pred is called incorrectly outside the engine.
        debug_assert!(
            degree >= 3 && degree <= NCOLORS as u64,
            "Invalid choice {} produces out-of-range degree {}. Engine should only provide choices 0..{}",
            choice,
            degree,
            NCOLORS - 2
        );

        // Set the degree for this round
        ctx.set_face_degree(round, degree);

        PredicateResult::SuccessSamePredicate
    }

    fn name(&self) -> &str {
        "InnerFace"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineBuilder;
    use crate::predicates::test::SuspendPredicate;

    #[test]
    fn test_innerface_try_pred_choices() {
        let mut ctx = SearchContext::new();
        let mut pred = InnerFacePredicate;

        // Round 0-5 should return Choices
        for round in 0..NCOLORS {
            let result = pred.try_pred(&mut ctx, round);
            assert_eq!(result, PredicateResult::Choices(NCOLORS - 2));
        }
    }

    #[test]
    fn test_innerface_retry_pred_sets_degree() {
        let mut ctx = SearchContext::new();
        let mut pred = InnerFacePredicate;

        // Choice 0 → degree NCOLORS
        let result = pred.retry_pred(&mut ctx, 0, 0);
        assert_eq!(result, PredicateResult::SuccessSamePredicate);
        assert_eq!(ctx.get_face_degree(0), NCOLORS as u64);

        // For NCOLORS >= 4, test choice 1 → degree NCOLORS-1
        if NCOLORS >= 4 {
            let result = pred.retry_pred(&mut ctx, 1, 1);
            assert_eq!(result, PredicateResult::SuccessSamePredicate);
            assert_eq!(ctx.get_face_degree(1), (NCOLORS - 1) as u64);
        }
    }

    #[test]
    fn test_innerface_final_round_checks_sum() {
        let mut ctx = SearchContext::new();
        let mut pred = InnerFacePredicate;

        // Set degrees that don't sum correctly
        // For NCOLORS=3: all 4s would be 12, not 9
        // For NCOLORS=6: all 3s would be 18, not 27
        let wrong_degree = if NCOLORS == 3 { 4 } else { 3 };
        for i in 0..NCOLORS {
            ctx.set_face_degree(i, wrong_degree);
        }

        let result = pred.try_pred(&mut ctx, NCOLORS);
        assert_eq!(result, PredicateResult::Failure);
    }

    #[test]
    #[cfg(any(
        feature = "ncolors_6",
        not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
    ))]
    fn test_innerface_canonical_sequence_succeeds() {
        let mut ctx = SearchContext::new();
        let mut pred = InnerFacePredicate;

        // Set a known valid canonical sequence for NCOLORS=6: [6,6,4,4,4,3]
        // This sequence has 5 solutions according to counts_00000.txt
        let degrees = [6u64, 6, 4, 4, 4, 3];
        for (i, &degree) in degrees.iter().enumerate() {
            ctx.set_face_degree(i, degree);
        }

        let result = pred.try_pred(&mut ctx, NCOLORS);
        assert_eq!(result, PredicateResult::Success);
    }

    #[test]
    #[cfg(any(
        feature = "ncolors_6",
        not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
    ))]
    fn test_innerface_noncanonical_sequence_fails() {
        let mut ctx = SearchContext::new();
        let mut pred = InnerFacePredicate;

        // Set a known non-canonical sequence for NCOLORS=6: [6,6,3,4,5,3]
        let degrees = [6u64, 6, 3, 4, 5, 3];
        for (i, &degree) in degrees.iter().enumerate() {
            ctx.set_face_degree(i, degree);
        }

        let result = pred.try_pred(&mut ctx, NCOLORS);
        assert_eq!(result, PredicateResult::Failure);
    }

    #[test]
    fn test_innerface_with_engine_finds_solutions() {
        // This test runs a real search to find some degree signatures
        let mut ctx = SearchContext::new();

        let engine = EngineBuilder::new()
            .add(Box::new(InnerFacePredicate))
            .terminal(Box::new(SuspendPredicate))
            .build();

        // Run search - should find at least one solution
        let result = engine.search(&mut ctx);
        assert!(
            result.is_some(),
            "Should find at least one degree signature"
        );

        // Verify the degrees in the context sum correctly
        let degrees = ctx.get_face_degrees();
        let sum: u64 = degrees.iter().sum();
        assert_eq!(sum, TOTAL_CENTRAL_NEIGHBOR_DEGREE as u64);

        // Verify the sequence is canonical or equivocal
        let degrees_u8: [u8; NCOLORS] = {
            let mut arr = [0u8; NCOLORS];
            for i in 0..NCOLORS {
                arr[i] = degrees[i] as u8;
            }
            arr
        };
        let symmetry = check_symmetry(&degrees_u8);
        assert_ne!(symmetry, SymmetryType::NonCanonical);
    }
}
