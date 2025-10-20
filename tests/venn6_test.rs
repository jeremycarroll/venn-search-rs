// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

#![cfg(any(
    feature = "ncolors_6",
    not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
))]

use std::io::Write;

mod common;

use common::FixedInnerFacePredicate;
use venn_search::context::SearchContext;
use venn_search::state::statistics::{Counters, Statistics};

use venn_search::engine::predicate::OpenClose;
use venn_search::engine::{EngineBuilder, OpenClosePredicate};
use venn_search::predicates::advanced_test::OpenCloseFile;
use venn_search::predicates::{
    FailPredicate, InitializePredicate, InnerFacePredicate, VennPredicate,
};
use venn_search::state::statistics::Counters::VennSolutions;

#[derive(Debug)]
pub struct PrintSolutionCountPerInnerFace {
    on_enter: u64,
}

impl OpenClose for PrintSolutionCountPerInnerFace {
    fn open(&mut self, ctx: &mut SearchContext) -> bool {
        self.on_enter = ctx.statistics.get(VennSolutions);
        true
    }

    fn close(&mut self, ctx: &mut SearchContext) {
        let writer = ctx
            .state
            .output
            .as_deref_mut()
            .expect("Must open file to save solution");
        writeln!(
            writer,
            "{:?} has {} solutions.",
            ctx.state.current_face_degrees,
            ctx.statistics.get(VennSolutions) - self.on_enter
        )
        .unwrap();
    }
}

fn run_test(
    neighbor_degrees: [u64; 6],
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
fn test_664443() {
    // From RESULTS.md: 664443 has 5 solutions (all canonical)
    run_test([6, 6, 4, 4, 4, 3], true, 5, 0);
}

#[test]
fn test_664434() {
    // From RESULTS.md: 664434 has 2 solutions (all canonical)
    run_test([6, 6, 4, 4, 3, 4], true, 2, 0);
}

#[test]
fn test_655443() {
    // From RESULTS.md: 655443 has 6 solutions (all canonical)
    run_test([6, 5, 5, 4, 4, 3], true, 6, 0);
}

#[test]
fn test_all() {
    let mut ctx = SearchContext::new();

    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(OpenClosePredicate::new(
            "open file",
            OpenCloseFile::new(String::from("counts")),
        )))
        .add(Box::new(InnerFacePredicate))
        .add(Statistics::counting_predicate(
            Counters::InnerFaceSolutions,
            None,
        ))
        .add(Box::new(OpenClosePredicate::new(
            "open file",
            PrintSolutionCountPerInnerFace { on_enter: 0 },
        )))
        .add(Box::new(VennPredicate::new()))
        .add(Statistics::counting_predicate(
            Counters::VennSolutions,
            None,
        ))
        .terminal(Box::new(FailPredicate))
        .build();

    engine.search(&mut ctx);

    assert_eq!(ctx.statistics.get(Counters::VennSolutions), 233);
    assert_eq!(ctx.statistics.get(Counters::InnerFaceSolutions), 39);
}
