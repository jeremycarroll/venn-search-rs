// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Integration tests for VennPredicate - Full search end-to-end.
//!
//! These tests verify that the VennPredicate correctly finds all valid Venn diagrams
//! for different NCOLORS values, matching the expected solution counts from the C reference.

use venn_search::context::SearchContext;
use venn_search::engine::{EngineBuilder, Predicate, PredicateResult};
use venn_search::predicates::{FailPredicate, InitializePredicate, InnerFacePredicate, VennPredicate};
use std::cell::RefCell;
use std::rc::Rc;

/// Simple counter predicate that increments on each solution.
#[derive(Clone)]
struct CounterPredicate {
    count: Rc<RefCell<usize>>,
}

impl CounterPredicate {
    fn new(count: Rc<RefCell<usize>>) -> Self {
        Self { count }
    }
}

impl Predicate for CounterPredicate {
    fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        *self.count.borrow_mut() += 1;
        eprintln!("Found solution {}", *self.count.borrow());
        PredicateResult::Success
    }

    fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, _choice: usize) -> PredicateResult {
        PredicateResult::Failure
    }
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
    assert_eq!(
        final_count, 2,
        "Expected exactly 2 solutions for NCOLORS=3"
    )
}
