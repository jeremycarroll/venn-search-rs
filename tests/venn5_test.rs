// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

#![cfg(feature = "ncolors_5")]

/* We have some detailed test data for the N=5 case, which is big enoygh to be complicated,
  but is easier to debug than the N=6 case.
*/
use state::statistics::{Counters, Statistics};
use venn_search::context::SearchContext;
use venn_search::{propagation, state, Predicate, PredicateResult};

use venn_search::engine::EngineBuilder;
use venn_search::predicates::{
    FailPredicate, InitializePredicate, VennPredicate,
};

#[derive(Debug)]
pub struct FixedInnerFacePredicate([u64; 5]);

impl Predicate for FixedInnerFacePredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        if let Err(failure) =
            propagation::setup_central_face(&ctx.memo, &mut ctx.state, &mut ctx.trail, &self.0)
        {
            eprintln!(
                "Could not set face degree to {:?}, with {}",
                &self.0, &failure
            );
            return PredicateResult::Failure;
        }
        PredicateResult::Success
    }
    fn name(&self) -> &str {
        "InnerFace"
    }
}

pub struct SolutionCountingPredicate {}

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
    run_test([5, 5, 4, 3, 3], true, 6, 0);
}

#[test]
fn test_33333() {
    run_test([3, 3, 3, 3, 3], false, 0, 0);
}
