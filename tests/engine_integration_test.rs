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
use venn_search::engine::EngineBuilder;
use venn_search::predicates::test::{
    AlwaysFailPredicate, ChoicePredicate, IntegerRangePredicate, MultiRoundPredicate,
    SuspendPredicate,
};

#[test]
fn test_simple_integer_search_with_suspend() {
    let mut ctx = SearchContext::new();
    let engine = EngineBuilder::new()
        .add(Box::new(IntegerRangePredicate::new(1, 11)))
        .terminal(Box::new(SuspendPredicate))
        .build();

    // Should suspend after first predicate succeeds
    let engine = engine.search(&mut ctx);
    assert!(engine.is_some()); // Suspended
    let engine = engine.unwrap();
    let (tries, retries) = engine.statistics();
    assert_eq!(tries, 2); // IntegerRange.try_pred + Suspend.try_pred
    assert_eq!(retries, 1); // IntegerRange.retry_pred(choice=0)
}

#[test]
fn test_two_integer_ranges() {
    let mut ctx = SearchContext::new();
    let engine = EngineBuilder::new()
        .add(Box::new(IntegerRangePredicate::new(1, 3)))
        .add(Box::new(IntegerRangePredicate::new(10, 12)))
        .terminal(Box::new(SuspendPredicate))
        .build();

    // Should find solution with first choice of each
    let engine = engine.search(&mut ctx);
    assert!(engine.is_some()); // Suspended
    let engine = engine.unwrap();
    let (tries, retries) = engine.statistics();
    assert_eq!(tries, 3); // All three predicates tried
    assert_eq!(retries, 2); // Two retry_pred calls (one per IntegerRange)
}

#[test]
fn test_choice_predicate_search() {
    let mut ctx = SearchContext::new();
    let engine = EngineBuilder::new()
        .add(Box::new(ChoicePredicate::new(vec!["A", "B", "C"])))
        .terminal(Box::new(SuspendPredicate))
        .build();

    // Should suspend after choosing first option
    let engine = engine.search(&mut ctx);
    assert!(engine.is_some()); // Suspended
    let engine = engine.unwrap();
    let (tries, retries) = engine.statistics();
    assert_eq!(tries, 2); // Choice.try_pred + Suspend.try_pred
    assert_eq!(retries, 1); // Choice.retry_pred(choice=0)
}

#[test]
fn test_backtracking_with_failure() {
    let mut ctx = SearchContext::new();
    let engine = EngineBuilder::new()
        .add(Box::new(IntegerRangePredicate::new(1, 3)))
        .add(Box::new(AlwaysFailPredicate)) // Force backtracking
        .terminal(Box::new(SuspendPredicate))
        .build();

    // Should fail - second predicate always fails
    let result = engine.search(&mut ctx);
    assert!(result.is_none()); // Exhausted - engine consumed
}

#[test]
fn test_backtracking_exhausts_options() {
    let mut ctx = SearchContext::new();

    // This will try all combinations of first * second integers
    // But AlwaysFail will force complete backtracking
    let engine = EngineBuilder::new()
        .add(Box::new(IntegerRangePredicate::new(1, 4))) // 3 choices: 1, 2, 3
        .add(Box::new(IntegerRangePredicate::new(10, 13))) // 3 choices: 10, 11, 12
        .add(Box::new(AlwaysFailPredicate)) // Always fail
        .terminal(Box::new(SuspendPredicate))
        .build();

    let result = engine.search(&mut ctx);
    assert!(result.is_none()); // Exhausted - engine consumed
}

#[test]
fn test_empty_search_space() {
    let mut ctx = SearchContext::new();
    let engine = EngineBuilder::new()
        .add(Box::new(IntegerRangePredicate::new(1, 1))) // Empty range
        .terminal(Box::new(SuspendPredicate))
        .build();

    let result = engine.search(&mut ctx);
    assert!(result.is_none()); // Exhausted - engine consumed
}

#[test]
fn test_multi_round_predicate() {
    let mut ctx = SearchContext::new();

    // This predicate will execute 3 times (rounds 0, 1, 2)
    let engine = EngineBuilder::new()
        .add(Box::new(MultiRoundPredicate::new(3)))
        .terminal(Box::new(SuspendPredicate))
        .build();

    let engine = engine.search(&mut ctx);
    assert!(engine.is_some()); // Suspended
    let engine = engine.unwrap();
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
    let engine = EngineBuilder::new()
        .add(Box::new(ChoicePredicate::new(vec!["X", "Y"])))
        .add(Box::new(AlwaysFailPredicate))
        .terminal(Box::new(SuspendPredicate))
        .build();

    let result = engine.search(&mut ctx);
    assert!(result.is_none()); // Exhausted - engine consumed
}

#[test]
fn test_complex_backtracking_scenario() {
    let mut ctx = SearchContext::new();

    // This creates a search tree:
    // - First predicate: 3 options (1, 2, 3)
    // - Second predicate: 2 options (A, B)
    // Should find solution immediately with (1, A) and suspend
    let engine = EngineBuilder::new()
        .add(Box::new(IntegerRangePredicate::new(1, 4))) // 3 choices
        .add(Box::new(ChoicePredicate::new(vec!["A", "B"]))) // 2 choices
        .terminal(Box::new(SuspendPredicate))
        .build();

    let engine = engine.search(&mut ctx);
    assert!(engine.is_some()); // Suspended
    let engine = engine.unwrap();
    let (tries, retries) = engine.statistics();
    assert_eq!(tries, 3); // All three tried once
    assert_eq!(retries, 2); // IntegerRange.retry(0) + Choice.retry(0)
}

#[test]
fn test_empty_predicates() {
    let mut ctx = SearchContext::new();
    let engine = EngineBuilder::new()
        .terminal(Box::new(SuspendPredicate))
        .build();

    let result = engine.search(&mut ctx);
    assert!(result.is_some()); // Actually suspends immediately with just terminal
}
