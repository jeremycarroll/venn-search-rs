// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

#![cfg(feature = "ncolors_3")]

/* We have some detailed test data for the N=5 case, which is big enough to be complicated,
  but is easier to debug than the N=6 case.
*/
use state::statistics::{Counters, Statistics};
use venn_search::context::SearchContext;
use venn_search::{propagation, state, Predicate, PredicateResult};
use std::fmt::Write;

use venn_search::engine::{EngineBuilder, OpenClosePredicate};
use venn_search::predicates::{
    FailPredicate, InitializePredicate, VennPredicate
};
use venn_search::predicates::advanced_test::{
    OpenCloseFile, PrintHeaderPredicate, PrintFacesPredicate,
    PrintFaceCyclesPredicate, PrintEdgeCyclesPredicate};
use venn_search::geometry::constants::NFACES;
use venn_search::symmetry::s6::{check_solution_canonicality, SymmetryType};


#[test]
fn test_venn3()  {
    let mut ctx = SearchContext::new();


    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(VennPredicate::new()))
        .add(Statistics::counting_predicate(
            Counters::VennSolutions,
            None,
        ))
        .add(Box::new(OpenClosePredicate::new("open file", OpenCloseFile::new(String::from("solution3")))))
        .add(Box::new(PrintFacesPredicate {}))
        .add(Box::new(PrintFaceCyclesPredicate {}))
        .add(Box::new(PrintEdgeCyclesPredicate::new(Some(|&x|x==4))))
        .terminal(Box::new(FailPredicate))
        .build();

    engine.search(&mut ctx);

    assert_eq!(
        ctx.statistics.get(Counters::VennSolutions),
        2
    );
}
