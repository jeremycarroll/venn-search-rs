// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! VENN_CLEANUP_MEASUREMENT: temporary paired-ref probe, removed by VC-FINAL.
//! This file is injected unchanged into both disposable checkouts by the runner.

#![cfg(feature = "ncolors_6")]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;
use venn_search::engine::EngineBuilder;
use venn_search::predicates::{
    FailPredicate, InitializePredicate, InnerFacePredicate, VennPredicate,
};
use venn_search::{Predicate, PredicateResult, SearchContext};

#[derive(Debug)]
struct QuietCounter(Rc<Cell<usize>>);

impl Predicate for QuietCounter {
    fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        self.0.set(self.0.get() + 1);
        PredicateResult::Success
    }

    fn name(&self) -> &str {
        "QuietCounter"
    }
}

#[test]
#[ignore = "VENN_CLEANUP_MEASUREMENT: run serially via scripts/measure-cleanup.sh"]
fn full_search() {
    let mut ctx = SearchContext::new();
    let count = Rc::new(Cell::new(0));
    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(InnerFacePredicate))
        .add(Box::new(VennPredicate::new()))
        .add(Box::new(QuietCounter(Rc::clone(&count))))
        .terminal(Box::new(FailPredicate))
        .build();
    // Excludes MEMO/context construction, compilation, output and file I/O.
    let start = Instant::now();
    let suspended = engine.search(&mut ctx);
    let elapsed = start.elapsed();
    assert!(suspended.is_none());
    assert_eq!(count.get(), 233);
    println!("VENN_CLEANUP_MEASUREMENT search_ns={}", elapsed.as_nanos());
}
