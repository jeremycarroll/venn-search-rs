// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! InitializePredicate - Sets up all MEMO data structures.
//!
//! This predicate runs exactly once at the start of the search to initialize
//! all precomputed lookup tables and static structures.

use crate::context::SearchContext;
use crate::engine::{Predicate, PredicateResult, TerminalPredicate};

/// InitializePredicate performs one-time initialization of MEMO data.
///
/// This predicate is deterministic and runs exactly once (round=0 only).
/// It never backtracks and never produces choices.
///
/// # Phase 6 Implementation
///
/// In Phase 6, this predicate will:
/// - Compute all facial cycle constraint lookup tables
/// - Initialize possible vertex configurations
/// - Set up edge and face relationship tables
/// - Freeze the trail to prevent backtracking past initialization
///
/// # Current Implementation
///
/// For Phase 5, this is a minimal skeleton that:
/// - Asserts round == 0 (programming error otherwise)
/// - Returns Success to advance to the next predicate
#[derive(Debug)]
pub struct InitializePredicate;

impl Predicate for InitializePredicate {
    fn try_pred(&mut self, _ctx: &mut SearchContext, round: usize) -> PredicateResult {
        assert_eq!(
            round, 0,
            "InitializePredicate must be called exactly once with round=0"
        );

        // TODO Phase 6: Real initialization
        // - Compute all MEMO data structures
        // - Initialize faces/edges/vertices
        // - Freeze trail to prevent backtracking past init

        PredicateResult::Success
    }

    fn name(&self) -> &str {
        "Initialize"
    }
}

/// InitializePredicate can be used as a terminal predicate (though not typically useful).
impl TerminalPredicate for InitializePredicate {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_round_0() {
        let mut ctx = SearchContext::new();
        let mut pred = InitializePredicate;

        let result = pred.try_pred(&mut ctx, 0);
        assert_eq!(result, PredicateResult::Success);
    }

    #[test]
    #[should_panic(expected = "must be called exactly once with round=0")]
    fn test_initialize_round_1_panics() {
        let mut ctx = SearchContext::new();
        let mut pred = InitializePredicate;

        // Should panic on round != 0
        pred.try_pred(&mut ctx, 1);
    }

    #[test]
    #[should_panic(expected = "retry_pred should never be called")]
    fn test_initialize_retry_panics() {
        let mut ctx = SearchContext::new();
        let mut pred = InitializePredicate;

        // Should panic - InitializePredicate never creates choices
        pred.retry_pred(&mut ctx, 0, 0);
    }
}
