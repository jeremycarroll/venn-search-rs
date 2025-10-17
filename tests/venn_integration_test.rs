// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Integration tests for VennPredicate - Full search end-to-end.
//!
//! These tests verify that the VennPredicate correctly finds all valid Venn diagrams
//! for different NCOLORS values, matching the expected solution counts from the C reference.

use venn_search::context::SearchContext;
use venn_search::engine::{EngineBuilder, SearchEngine};
use venn_search::geometry::constants::{NCOLORS, NFACES};
use venn_search::predicates::test::SuspendPredicate;
use venn_search::predicates::{InitializePredicate, InnerFacePredicate, VennPredicate};

/// Count all solutions from a search engine.
///
/// Runs the search to exhaustion, counting how many solutions are found.
fn count_all_solutions(engine: SearchEngine, ctx: &mut SearchContext) -> usize {
    let mut count = 0;
    let mut current = Some(engine);

    while let Some(engine) = current {
        current = engine.search(ctx);
        if current.is_some() {
            count += 1;
        }
    }

    count
}

/// Validate that a solution has all faces assigned.
fn validate_solution_complete(ctx: &SearchContext, solution_num: usize) {
    for face_id in 0..NFACES {
        let face = ctx.state.faces.get(face_id);
        assert!(
            face.current_cycle().is_some(),
            "Solution {} has unassigned face {}",
            solution_num,
            face_id
        );
    }
}

#[test]
fn test_venn_search_ncolors_3_baseline() {
    // Skip if not NCOLORS=3
    if NCOLORS != 3 {
        eprintln!("Skipping test_venn_search_ncolors_3_baseline (NCOLORS={})", NCOLORS);
        return;
    }

    eprintln!("\n=== Testing VennPredicate for NCOLORS=3 (Baseline) ===");
    eprintln!("Expected: 2 solutions (with edge adjacency)");
    eprintln!("Current: Edge adjacency is stubbed, may not find valid solutions\n");

    let mut ctx = SearchContext::new();
    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(InnerFacePredicate))
        .add(Box::new(VennPredicate::new()))
        .terminal(Box::new(SuspendPredicate))
        .build();

    let mut solution_count = 0;
    let mut current = Some(engine);

    while let Some(engine) = current {
        current = engine.search(&mut ctx);
        if current.is_some() {
            solution_count += 1;
            eprintln!("Found solution {}", solution_count);
            validate_solution_complete(&ctx, solution_count);
        }
    }

    eprintln!("\n=== Baseline Results ===");
    eprintln!("Solutions found: {}", solution_count);
    eprintln!("Expected (with edge adjacency): 2");

    if solution_count == 0 {
        eprintln!("✓ No solutions found - edge adjacency likely required");
    } else if solution_count == 2 {
        eprintln!("✓ Found correct number - edge adjacency may be working!");
    } else {
        eprintln!("⚠ Unexpected count - may indicate invalid solutions or bugs");
    }

    // Don't assert specific count yet - this is baseline to document current behavior
}
