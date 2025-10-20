// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

#![cfg(feature = "ncolors_5")]

/* We have some detailed test data for the N=5 case, which is big enough to be complicated,
  but is easier to debug than the N=6 case.
*/
mod common;

use common::FixedInnerFacePredicate;
use state::statistics::{Counters, Statistics};
use venn_search::context::SearchContext;
use venn_search::engine::{EngineBuilder, OpenClosePredicate};
use venn_search::predicates::{FailPredicate, InitializePredicate, VennPredicate};
use venn_search::predicates::advanced_test::{
    OpenCloseFile, PrintEdgeCyclesPredicate, PrintFaceCyclesPredicate, PrintFacesPredicate,
    PrintHeaderPredicate,
};
// Runs a program: initialize, set the degrees as given, venn, count solutions
// by whether they are canonical or equivocal, and checks they match the expected.
fn run_test(
    neighbor_degrees: [u64; 5],
    expect_to_start: bool,
    expected_canonical: u64,
    expected_equivocal: u64,
) {
    let mut ctx = SearchContext::new();


    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(FixedInnerFacePredicate(neighbor_degrees)))
        .add(Statistics::counting_predicate(
            Counters::InnerFaceSolutions,
            None,
        ))
        .add(Box::new(VennPredicate::new()))
        .add(Statistics::counting_predicate(
            Counters::VennSolutions,
            None,
        ))
        .add(Box::new(OpenClosePredicate::new("open file", OpenCloseFile::new(String::from("solution")))))
        .add(Box::new(PrintHeaderPredicate {}))
        .add(Box::new(PrintFacesPredicate {}))
        .add(Box::new(PrintFaceCyclesPredicate {}))
        .add(Box::new(PrintEdgeCyclesPredicate::new(None)))
        .terminal(Box::new(FailPredicate))
        .build();

    engine.search(&mut ctx);

    assert_eq!(
        ctx.statistics.get(Counters::InnerFaceSolutions),
        if expect_to_start { 1 } else { 0 }
    );
    assert_eq!(
        ctx.statistics.get(Counters::VennSolutions),
        expected_canonical + expected_equivocal
    );
}

#[test]
fn test_55433() {
    // TEMPORARY: Getting 7 instead of 6. The 7th has a quadrilateral in it, and is not being
    // detected because the edge topology is not being saved correctly, and hence corner detection
    // is not working.
    run_test([5, 5, 4, 3, 3], true, 6, 0);
}

#[test]
fn test_33333() {
    run_test([3, 3, 3, 3, 3], false, 0, 0);
}

#[test]
fn test_44444() {
    run_test([4, 4, 4, 4, 4], true, 0, 2);
}

#[test]
fn test_55343() {
    run_test([5, 5, 3, 4, 3], false, 0, 0);
}

#[test]
fn test_54443() {
    run_test([5, 4, 4, 4, 3], true, 4, 0);
}


#[test]
fn test_54434() {
    run_test([5, 4, 4, 3, 4], true, 5, 0);
}