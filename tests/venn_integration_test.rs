// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Integration tests for VennPredicate - Full search end-to-end.
//!
//! These tests verify that the VennPredicate correctly finds all valid Venn diagrams
//! for different NCOLORS values, matching the expected solution counts from the C reference.

#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
use std::cell::RefCell;
#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
use std::rc::Rc;
#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
use venn_search::context::SearchContext;
#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
use venn_search::engine::{EngineBuilder, Predicate, PredicateResult};
#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
use venn_search::predicates::{
    FailPredicate, InitializePredicate, InnerFacePredicate, VennPredicate,
};

/// Simple counter predicate that increments on each solution.
#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
#[derive(Clone)]
struct CounterPredicate {
    count: Rc<RefCell<usize>>,
}

#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
impl CounterPredicate {
    fn new(count: Rc<RefCell<usize>>) -> Self {
        Self { count }
    }
}

#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
impl Predicate for CounterPredicate {
    fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        *self.count.borrow_mut() += 1;
        eprintln!("Found solution {}", *self.count.borrow());
        PredicateResult::Success
    }

    fn retry_pred(
        &mut self,
        _ctx: &mut SearchContext,
        _round: usize,
        _choice: usize,
    ) -> PredicateResult {
        PredicateResult::Failure
    }
}

/// Validation predicate that checks solution structure correctness.
#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
#[derive(Clone)]
struct ValidationPredicate;

#[cfg(any(feature = "ncolors_3", feature = "ncolors_4"))]
impl Predicate for ValidationPredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        use venn_search::geometry::constants::{NCOLORS, NFACES};

        // Validate all faces have assigned cycles
        for face_id in 0..NFACES {
            let face = &ctx.state.faces.faces[face_id];
            let current_cycle = face.current_cycle();

            assert!(
                current_cycle.is_some(),
                "Face {} has no assigned cycle",
                face_id
            );

            let cycle_id = current_cycle.unwrap();
            let cycle = ctx.memo.cycles.get(cycle_id);

            // Validate cycle has valid length (3 to NCOLORS)
            assert!(
                cycle.len() >= 3,
                "Face {} assigned cycle {} with invalid length {} (< 3)",
                face_id,
                cycle_id,
                cycle.len()
            );
            assert!(
                cycle.len() <= NCOLORS,
                "Face {} assigned cycle {} with invalid length {} (> NCOLORS={})",
                face_id,
                cycle_id,
                cycle.len(),
                NCOLORS
            );

            // Validate cycle was in the face's initial possible set
            let face_memo = ctx.memo.faces.get_face(face_id);
            assert!(
                face_memo.possible_cycles.contains(cycle_id),
                "Face {} assigned cycle {} that wasn't in initial possible cycles",
                face_id,
                cycle_id
            );
        }

        eprintln!("✓ Solution validation passed: all {} faces assigned valid cycles", NFACES);
        PredicateResult::Success
    }

    fn retry_pred(
        &mut self,
        _ctx: &mut SearchContext,
        _round: usize,
        _choice: usize,
    ) -> PredicateResult {
        PredicateResult::Failure
    }
}

#[test]
#[cfg(feature = "ncolors_3")]
fn test_known_solution_structure() {
    eprintln!("\n=== Testing Known Solution Structure Validation for NCOLORS=3 ===");
    eprintln!("Validating that found solutions have correct structure");

    let mut ctx = SearchContext::new();
    let solution_count = Rc::new(RefCell::new(0));

    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(InnerFacePredicate))
        .add(Box::new(VennPredicate::new()))
        .add(Box::new(ValidationPredicate)) // <-- Validates solution structure
        .add(Box::new(CounterPredicate::new(Rc::clone(&solution_count))))
        .terminal(Box::new(FailPredicate))
        .build();

    // Run search to exhaustion
    engine.search(&mut ctx);

    let final_count = *solution_count.borrow();
    eprintln!("\n=== Validation Results ===");
    eprintln!("Validated solutions: {}", final_count);

    // Should find at least one valid solution
    assert!(
        final_count > 0,
        "Should find at least one valid solution with correct structure"
    );
}

#[test]
#[cfg(feature = "ncolors_3")]
fn test_venn_search_ncolors_3_baseline() {
    eprintln!("\n=== Testing VennPredicate for NCOLORS=3 ===");
    eprintln!("Expected: 2 valid solutions");
    eprintln!("Testing full constraint propagation\n");

    let mut ctx = SearchContext::new();
    let solution_count = Rc::new(RefCell::new(0));

    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(InnerFacePredicate))
        .add(Box::new(VennPredicate::new()))
        .add(Box::new(CounterPredicate::new(Rc::clone(&solution_count))))
        .terminal(Box::new(FailPredicate))
        .build();

    // Run search to exhaustion (FailPredicate forces backtracking)
    engine.search(&mut ctx);

    let final_count = *solution_count.borrow();
    eprintln!("\n=== Results ===");
    eprintln!("Solutions found: {}", final_count);
    eprintln!("Expected: 2");

    // For NCOLORS=3, we expect exactly 2 solutions
    assert_eq!(final_count, 2, "Expected exactly 2 solutions for NCOLORS=3")
}

#[test]
#[cfg(feature = "ncolors_4")]
fn test_venn_search_ncolors_4() {
    eprintln!("\n=== Testing VennPredicate for NCOLORS=4 ===");
    eprintln!("Expected: 48 solutions (current implementation)");
    eprintln!("Note: Will be 3 when edge crossing limits implemented");
    eprintln!("Testing constraint propagation\n");

    let mut ctx = SearchContext::new();
    let solution_count = Rc::new(RefCell::new(0));

    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(InnerFacePredicate))
        .add(Box::new(VennPredicate::new()))
        .add(Box::new(CounterPredicate::new(Rc::clone(&solution_count))))
        .terminal(Box::new(FailPredicate))
        .build();

    // Run search to exhaustion
    engine.search(&mut ctx);

    let final_count = *solution_count.borrow();
    eprintln!("\n=== Results ===");
    eprintln!("Solutions found: {}", final_count);
    eprintln!("Current: 48 (without edge crossing limit checks)");
    eprintln!("Target: 3 (with full constraints)");

    // For NCOLORS=4 with current constraints, we find 48 solutions
    // This will reduce to 3 when edge crossing limit checks are added
    assert_eq!(
        final_count, 48,
        "Expected 48 solutions for NCOLORS=4 (current implementation)"
    )
}
