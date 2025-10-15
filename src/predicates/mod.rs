// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Search predicates.
//!
//! This module contains the predicates used in the search algorithm.
//! Each predicate represents a choice point in the search space.
//!
//! # Organization
//!
//! - `test`: Simple test predicates for validating the engine
//! - `initialize`: InitializePredicate for setting up MEMO data
//! - `innerface`: InnerFacePredicate for finding degree signatures
//! - Built-in predicates: `FailPredicate`, `SuspendPredicate`

pub mod initialize;
pub mod innerface;
pub mod test;

// Re-export main predicates for convenience
pub use initialize::InitializePredicate;
pub use innerface::InnerFacePredicate;

use crate::context::SearchContext;
use crate::engine::{Predicate, PredicateResult, TerminalPredicate};

/// Built-in fail predicate (Prolog's `fail.`).
///
/// This predicate always fails, forcing backtracking. It's a terminal predicate
/// that ends a search path without success, similar to Prolog's `fail.` built-in.
///
/// # Usage
///
/// Use `FailPredicate` to explicitly terminate unsuccessful search paths:
/// - As a terminal predicate to mark unsatisfiable branches
/// - To force exploration of all alternatives
///
/// # Example
///
/// ```
/// use venn_search::engine::EngineBuilder;
/// use venn_search::predicates::FailPredicate;
/// use venn_search::predicates::test::IntegerRangePredicate;
/// use venn_search::context::SearchContext;
///
/// let mut ctx = SearchContext::new();
/// let engine = EngineBuilder::new()
///     .add(Box::new(IntegerRangePredicate::new(1, 3)))
///     .terminal(Box::new(FailPredicate))
///     .build();
///
/// // Engine will exhaust all integer choices then fail
/// let result = engine.search(&mut ctx);
/// assert!(result.is_none()); // Failed - engine consumed
/// ```
#[derive(Debug)]
pub struct FailPredicate;

impl Predicate for FailPredicate {
    fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        PredicateResult::Failure
    }

    fn retry_pred(
        &mut self,
        _ctx: &mut SearchContext,
        _round: usize,
        _choice: usize,
    ) -> PredicateResult {
        PredicateResult::Failure
    }

    fn name(&self) -> &str {
        "Fail"
    }
}

/// FailPredicate is a terminal predicate (like Prolog's fail.).
impl TerminalPredicate for FailPredicate {}
