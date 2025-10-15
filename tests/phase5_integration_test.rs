// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Integration tests for Phase 5: InitializePredicate and InnerFacePredicate
//!
//! These tests verify that the predicates work correctly together in the engine.

use std::sync::atomic::{AtomicUsize, Ordering};
use venn_search::context::SearchContext;
use venn_search::engine::{EngineBuilder, Predicate, PredicateResult, TerminalPredicate};
use venn_search::predicates::{InitializePredicate, InnerFacePredicate};

#[test]
fn test_initialize_and_innerface_together() {
    // Test that InitializePredicate and InnerFacePredicate work together to find ONE solution
    use venn_search::predicates::test::SuspendPredicate;

    let mut ctx = SearchContext::new();

    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(InnerFacePredicate))
        .terminal(Box::new(SuspendPredicate))
        .build();

    // Run search - should find one degree signature and suspend
    let result = engine.search(&mut ctx);
    assert!(
        result.is_some(),
        "Should find at least one degree signature"
    );

    // Verify it's a valid solution
    let degrees = ctx.get_face_degrees();
    let sum: u64 = degrees.iter().sum();
    use venn_search::geometry::constants::TOTAL_CENTRAL_NEIGHBOR_DEGREE;
    assert_eq!(sum, TOTAL_CENTRAL_NEIGHBOR_DEGREE as u64);
}

/// Counter predicate that counts solutions and always fails to continue search.
///
/// This mimics the C test's `foundSolution` predicate.
#[derive(Debug)]
struct CounterPredicate {
    counter: &'static AtomicUsize,
}

impl CounterPredicate {
    fn new(counter: &'static AtomicUsize) -> Self {
        Self { counter }
    }
}

impl Predicate for CounterPredicate {
    fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        // Increment counter and fail to continue search
        self.counter.fetch_add(1, Ordering::SeqCst);
        PredicateResult::Failure
    }

    fn retry_pred(
        &mut self,
        _ctx: &mut SearchContext,
        _round: usize,
        _choice: usize,
    ) -> PredicateResult {
        PredicateResult::Failure
    }

    fn name(&self) -> &str {
        "Counter"
    }
}

impl TerminalPredicate for CounterPredicate {}

#[test]
#[cfg(any(
    feature = "ncolors_6",
    not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
))]
fn test_find_all_degree_signatures_n6() {
    // This test exhaustively searches for all canonical degree signatures for NCOLORS=6
    // Expected to find exactly 56 canonical solutions
    static SOLUTION_COUNT: AtomicUsize = AtomicUsize::new(0);
    SOLUTION_COUNT.store(0, Ordering::SeqCst);

    let mut ctx = SearchContext::new();

    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(InnerFacePredicate))
        .terminal(Box::new(CounterPredicate::new(&SOLUTION_COUNT)))
        .build();

    // Run exhaustive search
    let result = engine.search(&mut ctx);
    assert!(result.is_none(), "Search should exhaust and fail");

    let count = SOLUTION_COUNT.load(Ordering::SeqCst);
    println!("Found {} canonical degree signatures for NCOLORS=6", count);
    assert_eq!(
        count, 56,
        "Expected 56 canonical degree signatures for NCOLORS=6"
    );
}

#[test]
#[cfg(feature = "ncolors_3")]
fn test_degree_signatures_n3() {
    // For NCOLORS=3, there should be 1 canonical/equivocal solution: [3,3,3]
    static SOLUTION_COUNT: AtomicUsize = AtomicUsize::new(0);
    SOLUTION_COUNT.store(0, Ordering::SeqCst);

    let mut ctx = SearchContext::new();

    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(InnerFacePredicate))
        .terminal(Box::new(CounterPredicate::new(&SOLUTION_COUNT)))
        .build();

    let result = engine.search(&mut ctx);
    assert!(result.is_none(), "Search should exhaust and fail");

    let count = SOLUTION_COUNT.load(Ordering::SeqCst);
    println!("Found {} degree signatures for NCOLORS=3", count);
    assert!(count >= 1, "Should find at least 1 solution for NCOLORS=3");
}

#[test]
fn test_known_canonical_examples() {
    // Test a few known canonical sequences
    use venn_search::geometry::constants::NCOLORS;

    let test_cases: Vec<(Vec<u64>, bool)> = if NCOLORS == 6 {
        vec![
            (vec![6, 6, 3, 5, 4, 3], true),  // Canonical
            (vec![6, 6, 3, 4, 5, 3], false), // Non-canonical (reflection)
            (vec![5, 4, 5, 4, 5, 4], true),  // Equivocal
        ]
    } else if NCOLORS == 3 {
        vec![(vec![3, 3, 3], true)] // Canonical/Equivocal
    } else {
        vec![] // Add test cases for N=4,5 as needed
    };

    for (degrees_vec, should_pass) in test_cases {
        let mut ctx = SearchContext::new();
        for (i, &degree) in degrees_vec.iter().enumerate() {
            ctx.set_face_degree(i, degree);
        }

        let mut pred = InnerFacePredicate;
        let result = pred.try_pred(&mut ctx, NCOLORS);

        if should_pass {
            assert_eq!(
                result,
                PredicateResult::Success,
                "Sequence {:?} should be accepted",
                degrees_vec
            );
        } else {
            assert_eq!(
                result,
                PredicateResult::Failure,
                "Sequence {:?} should be rejected",
                degrees_vec
            );
        }
    }
}
