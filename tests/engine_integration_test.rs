// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Integration tests for the search engine.
//!
//! These tests validate that the engine correctly:
//! - Runs predicates in sequence
//! - Handles Choices and retry_pred correctly
//! - Backtracks on failure
//! - Restores state via trail
//! - Supports SuccessSamePredicate for multi-round predicates
//! - Suspends execution when requested

use venn_search::context::SearchContext;
use venn_search::engine::SearchEngine;
use venn_search::predicates::test::{
    AlwaysFailPredicate, ChoicePredicate, IntegerRangePredicate, MultiRoundPredicate,
    SuspendPredicate,
};

#[test]
fn test_simple_integer_search_with_suspend() {
    let mut ctx = SearchContext::new();
    let mut engine = SearchEngine::new(vec![
        Box::new(IntegerRangePredicate::new(1, 11)),
        Box::new(SuspendPredicate),
    ]);

    // Should suspend after first predicate succeeds
    assert_eq!(engine.search(&mut ctx), Err(()));
    let (tries, retries) = engine.statistics();
    assert_eq!(tries, 2); // IntegerRange.try_pred + Suspend.try_pred
    assert_eq!(retries, 1); // IntegerRange.retry_pred(choice=0)
}

#[test]
fn test_two_integer_ranges() {
    let mut ctx = SearchContext::new();
    let mut engine = SearchEngine::new(vec![
        Box::new(IntegerRangePredicate::new(1, 3)),
        Box::new(IntegerRangePredicate::new(10, 12)),
        Box::new(SuspendPredicate),
    ]);

    // Should find solution with first choice of each
    assert_eq!(engine.search(&mut ctx), Err(()));
    let (tries, retries) = engine.statistics();
    assert_eq!(tries, 3); // All three predicates tried
    assert_eq!(retries, 2); // Two retry_pred calls (one per IntegerRange)
}

#[test]
fn test_choice_predicate_search() {
    let mut ctx = SearchContext::new();
    let mut engine = SearchEngine::new(vec![
        Box::new(ChoicePredicate::new(vec!["A", "B", "C"])),
        Box::new(SuspendPredicate),
    ]);

    // Should suspend after choosing first option
    assert_eq!(engine.search(&mut ctx), Err(()));
    let (tries, retries) = engine.statistics();
    assert_eq!(tries, 2); // Choice.try_pred + Suspend.try_pred
    assert_eq!(retries, 1); // Choice.retry_pred(choice=0)
}

#[test]
fn test_backtracking_with_failure() {
    let mut ctx = SearchContext::new();
    let mut engine = SearchEngine::new(vec![
        Box::new(IntegerRangePredicate::new(1, 3)),
        Box::new(AlwaysFailPredicate), // Force backtracking
        Box::new(SuspendPredicate),
    ]);

    // Should fail - second predicate always fails
    assert_eq!(engine.search(&mut ctx), Ok(false));
    let (tries, retries) = engine.statistics();
    assert!(tries >= 2); // At least IntegerRange + AlwaysFail
    assert!(retries >= 1); // Should have tried multiple integer choices
}

#[test]
fn test_backtracking_exhausts_options() {
    let mut ctx = SearchContext::new();

    // This will try all combinations of first * second integers
    // But AlwaysFail will force complete backtracking
    let mut engine = SearchEngine::new(vec![
        Box::new(IntegerRangePredicate::new(1, 4)), // 3 choices: 1, 2, 3
        Box::new(IntegerRangePredicate::new(10, 13)), // 3 choices: 10, 11, 12
        Box::new(AlwaysFailPredicate), // Always fail
        Box::new(SuspendPredicate),
    ]);

    assert_eq!(engine.search(&mut ctx), Ok(false));
    let (tries, retries) = engine.statistics();

    // Should have tried many combinations
    // Each combination tries: IntegerRange1.try + IntegerRange2.try + AlwaysFail.try
    // Plus retry_pred calls for each choice
    assert!(tries >= 3); // At least the three predicates
    assert!(retries > 3); // Should have tried multiple choices (3*3=9 combinations)
}

#[test]
fn test_empty_search_space() {
    let mut ctx = SearchContext::new();
    let mut engine = SearchEngine::new(vec![
        Box::new(IntegerRangePredicate::new(1, 1)), // Empty range
        Box::new(SuspendPredicate),
    ]);

    assert_eq!(engine.search(&mut ctx), Ok(false));
    let (tries, retries) = engine.statistics();
    assert_eq!(tries, 1); // Only tried first predicate
    assert_eq!(retries, 0); // Failed immediately on try_pred
}

#[test]
fn test_engine_reset() {
    let mut ctx = SearchContext::new();
    let mut engine = SearchEngine::new(vec![
        Box::new(IntegerRangePredicate::new(1, 5)),
        Box::new(SuspendPredicate),
    ]);

    // First search
    assert_eq!(engine.search(&mut ctx), Err(()));
    let (tries1, retries1) = engine.statistics();

    // Reset
    engine.reset();
    let (tries2, retries2) = engine.statistics();
    assert_eq!(tries2, 0);
    assert_eq!(retries2, 0);

    // Second search should work again
    assert_eq!(engine.search(&mut ctx), Err(()));
    let (tries3, retries3) = engine.statistics();
    assert_eq!(tries3, tries1);
    assert_eq!(retries3, retries1);
}

#[test]
fn test_multi_round_predicate() {
    let mut ctx = SearchContext::new();

    // This predicate will execute 3 times (rounds 0, 1, 2)
    let mut engine = SearchEngine::new(vec![
        Box::new(MultiRoundPredicate::new(3)),
        Box::new(SuspendPredicate),
    ]);

    assert_eq!(engine.search(&mut ctx), Err(()));
    let (tries, retries) = engine.statistics();

    // Should call try_pred 4 times: MultiRound(0, 1, 2) + Suspend
    assert_eq!(tries, 4);
    assert_eq!(retries, 0); // No retry_pred calls
}

#[test]
fn test_choices_with_backtracking() {
    let mut ctx = SearchContext::new();

    // First predicate has 2 choices, second always fails
    // Should try both choices of first predicate before giving up
    let mut engine = SearchEngine::new(vec![
        Box::new(ChoicePredicate::new(vec!["X", "Y"])),
        Box::new(AlwaysFailPredicate),
        Box::new(SuspendPredicate),
    ]);

    assert_eq!(engine.search(&mut ctx), Ok(false));
    let (tries, retries) = engine.statistics();

    // try_pred: Choice, AlwaysFail (x2)
    assert!(tries >= 3);
    // retry_pred: Choice(0), Choice(1)
    assert_eq!(retries, 2);
}

#[test]
fn test_complex_backtracking_scenario() {
    let mut ctx = SearchContext::new();

    // This creates a search tree:
    // - First predicate: 3 options (1, 2, 3)
    // - Second predicate: 2 options (A, B)
    // Should find solution immediately with (1, A) and suspend
    let mut engine = SearchEngine::new(vec![
        Box::new(IntegerRangePredicate::new(1, 4)), // 3 choices
        Box::new(ChoicePredicate::new(vec!["A", "B"])), // 2 choices
        Box::new(SuspendPredicate),
    ]);

    assert_eq!(engine.search(&mut ctx), Err(()));
    let (tries, retries) = engine.statistics();
    assert_eq!(tries, 3); // All three tried once
    assert_eq!(retries, 2); // IntegerRange.retry(0) + Choice.retry(0)
}

#[test]
fn test_empty_predicates() {
    let mut ctx = SearchContext::new();
    let mut engine = SearchEngine::new(vec![]);

    assert_eq!(engine.search(&mut ctx), Ok(false));
}
