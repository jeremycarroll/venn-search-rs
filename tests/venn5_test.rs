// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

#![cfg(feature = "ncolors_5")]

/* We have some detailed test data for the N=5 case, which is big enough to be complicated,
  but is easier to debug than the N=6 case.
*/
use state::statistics::{Counters, Statistics};
use venn_search::context::SearchContext;
use venn_search::{propagation, state, Predicate, PredicateResult};

use venn_search::engine::EngineBuilder;
use venn_search::predicates::{
    FailPredicate, InitializePredicate, VennPredicate,
};
use venn_search::geometry::constants::NFACES;
use venn_search::symmetry::s6::{check_solution_canonicality, SymmetryType};

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

/// Print and save the current solution state for debugging.
fn print_solution(ctx: &SearchContext, solution_number: u64) {
    use std::fs::File;
    use std::io::Write;

    // Check canonicality
    let symmetry = check_solution_canonicality(&ctx.state, &ctx.memo);
    let symmetry_str = match symmetry {
        SymmetryType::Canonical => "CANONICAL",
        SymmetryType::Equivocal => "EQUIVOCAL",
        SymmetryType::NonCanonical => "NON-CANONICAL",
    };

    eprintln!("\n=== Solution #{} ({}) ===", solution_number, symmetry_str);

    // Create output file
    let filename = format!("solution-{:02}.txt", solution_number);
    let mut file = File::create(&filename).expect("Failed to create solution file");

    writeln!(file, "Solution #{} ({})", solution_number, symmetry_str).unwrap();
    writeln!(file, "Face degrees: [5, 5, 4, 3, 3]\n").unwrap();

    // Print and save face cycle assignments
    for face_id in 0..NFACES {
        let face = &ctx.state.faces.faces[face_id];
        let face_memo = ctx.memo.faces.get_face(face_id);

        if let Some(cycle_id) = face.current_cycle() {
            let cycle = ctx.memo.cycles.get(cycle_id);
            let line = format!(
                "Face {:2} ({:06b}): cycle {:2} = {}",
                face_id,
                face_memo.colors.bits(),
                cycle_id,
                cycle
            );
            eprintln!("{}", line);
            writeln!(file, "{}", line).unwrap();
        } else {
            let line = format!("Face {:2} ({:06b}): UNASSIGNED", face_id, face_memo.colors.bits());
            eprintln!("{}", line);
            writeln!(file, "{}", line).unwrap();
        }
    }

    eprintln!("Saved to {}", filename);
}

// Runs a program: initialize, set the degrees as given, venn, count solutions
// by whether they are canonical or equivocal, and checks they match the expected.
fn run_test(
    neighbor_degrees: [u64; 5],
    expect_to_start: bool,
    expected_canonical: u64,
    expected_equivocal: u64,
) {
    let mut ctx = SearchContext::new();

    // Build a custom predicate that prints each solution before counting it
    struct PrintSolutionPredicate;
    impl Predicate for PrintSolutionPredicate {
        fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
            let count = ctx.statistics.get(Counters::VennSolutions);
            print_solution(ctx, count + 1);
            PredicateResult::Success
        }
    }

    let engine = EngineBuilder::new()
        .add(Box::new(InitializePredicate))
        .add(Box::new(FixedInnerFacePredicate(neighbor_degrees)))
        .add(Statistics::counting_predicate(
            Counters::InnerFaceSolutions,
            None,
        ))
        .add(Box::new(VennPredicate::new()))
        .add(Box::new(PrintSolutionPredicate))
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