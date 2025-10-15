// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Test predicates for validating the search engine.
//!
//! These predicates are simple examples that demonstrate how the engine works
//! without the complexity of geometric constraints. They're useful for:
//! - Testing the engine's backtracking logic
//! - Validating trail integration
//! - Providing examples for implementing real predicates
//!
//! # Execution Model
//!
//! These predicates follow the WAM-like execution model:
//! - `try_pred(round)` is called first and can return Choices(n)
//! - Engine then calls `retry_pred(round, choice)` for choice in 0..n
//! - Searches succeed via side effects, then Suspend or Fail to terminate

use crate::context::SearchContext;
use crate::engine::{Predicate, PredicateResult, TerminalPredicate};

/// Predicate that tries integers in a range using the Choices model.
///
/// This demonstrates the choice mechanism:
/// - try_pred returns Choices(n) where n is the range size
/// - retry_pred(round, choice) maps choice to an integer in [start, start+choice)
///
/// # Example
///
/// ```
/// use venn_search::engine::{EngineBuilder, Predicate, PredicateResult};
/// use venn_search::predicates::test::{IntegerRangePredicate, SuspendPredicate};
/// use venn_search::context::SearchContext;
///
/// let mut ctx = SearchContext::new();
/// let engine = EngineBuilder::new()
///     .add(Box::new(IntegerRangePredicate::new(1, 4)))  // Try 1, 2, 3
///     .terminal(Box::new(SuspendPredicate))              // Terminal predicate
///     .build();
///
/// // Will try integers and suspend
/// let engine = engine.search(&mut ctx);
/// assert!(engine.is_some()); // Suspended - engine returned
/// ```
#[derive(Debug)]
pub struct IntegerRangePredicate {
    start: i32,
    end: i32,
}

impl IntegerRangePredicate {
    /// Create a new IntegerRangePredicate that tries integers in [start, end).
    pub fn new(start: i32, end: i32) -> Self {
        Self { start, end }
    }

    /// Get the count of integers in the range.
    pub fn count(&self) -> usize {
        (self.end - self.start).max(0) as usize
    }
}

impl Predicate for IntegerRangePredicate {
    fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        let count = self.count();
        if count > 0 {
            PredicateResult::Choices(count)
        } else {
            PredicateResult::Failure
        }
    }

    fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, choice: usize) -> PredicateResult {
        // Map choice to integer value
        let value = self.start + choice as i32;
        if value < self.end {
            // In a real predicate, we'd set state in ctx here
            PredicateResult::Success
        } else {
            PredicateResult::Failure
        }
    }

    fn name(&self) -> &str {
        "IntegerRange"
    }
}

/// Predicate that tries a fixed list of choices using the Choices model.
///
/// # Example
///
/// ```
/// use venn_search::engine::{EngineBuilder, Predicate, PredicateResult};
/// use venn_search::predicates::test::{ChoicePredicate, SuspendPredicate};
/// use venn_search::context::SearchContext;
///
/// let mut ctx = SearchContext::new();
/// let engine = EngineBuilder::new()
///     .add(Box::new(ChoicePredicate::new(vec!["A", "B", "C"])))
///     .terminal(Box::new(SuspendPredicate))
///     .build();
///
/// let engine = engine.search(&mut ctx);
/// assert!(engine.is_some()); // Suspended - engine returned
/// ```
#[derive(Debug)]
pub struct ChoicePredicate<T: Clone> {
    options: Vec<T>,
}

impl<T: Clone> ChoicePredicate<T> {
    /// Create a new ChoicePredicate with the given options.
    pub fn new(options: Vec<T>) -> Self {
        Self { options }
    }
}

impl<T: Clone> Predicate for ChoicePredicate<T> {
    fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        if self.options.is_empty() {
            PredicateResult::Failure
        } else {
            PredicateResult::Choices(self.options.len())
        }
    }

    fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, choice: usize) -> PredicateResult {
        if choice < self.options.len() {
            // In a real predicate, we'd use self.options[choice] to set state in ctx
            PredicateResult::Success
        } else {
            PredicateResult::Failure
        }
    }

    fn name(&self) -> &str {
        "Choice"
    }
}

/// Predicate that suspends execution (for testing).
///
/// This is useful for testing intermediate states without needing a full search.
#[derive(Debug)]
pub struct SuspendPredicate;

impl Predicate for SuspendPredicate {
    fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        PredicateResult::Suspend
    }

    fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, _choice: usize) -> PredicateResult {
        // Suspend predicate never retries
        PredicateResult::Failure
    }

    fn name(&self) -> &str {
        "Suspend"
    }
}

/// Implement TerminalPredicate for SuspendPredicate.
impl TerminalPredicate for SuspendPredicate {}

// AlwaysFailPredicate has been moved to predicates::FailPredicate (built-in predicate)
// Re-export for backward compatibility with tests
pub use crate::predicates::FailPredicate as AlwaysFailPredicate;

/// Predicate that succeeds N times using SuccessSamePredicate (for testing rounds).
///
/// Demonstrates how predicates can execute multiple rounds:
/// - Round 0, 1, ..., N-2: return SuccessSamePredicate
/// - Round N-1: return Success to advance
#[derive(Debug)]
pub struct MultiRoundPredicate {
    rounds: usize,
}

impl MultiRoundPredicate {
    /// Create a predicate that executes for `rounds` rounds.
    pub fn new(rounds: usize) -> Self {
        Self { rounds }
    }
}

impl Predicate for MultiRoundPredicate {
    fn try_pred(&mut self, _ctx: &mut SearchContext, round: usize) -> PredicateResult {
        if round + 1 < self.rounds {
            // More rounds to go
            PredicateResult::SuccessSamePredicate
        } else if round + 1 == self.rounds {
            // Last round, advance to next predicate
            PredicateResult::Success
        } else {
            // Shouldn't happen, but fail if called past rounds
            PredicateResult::Failure
        }
    }

    fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, _choice: usize) -> PredicateResult {
        // This predicate doesn't use choices
        PredicateResult::Failure
    }

    fn name(&self) -> &str {
        "MultiRound"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_range_predicate() {
        let mut ctx = SearchContext::new();
        let mut pred = IntegerRangePredicate::new(1, 4);

        // try_pred should return Choices(3) for range 1..4
        assert_eq!(pred.try_pred(&mut ctx, 0), PredicateResult::Choices(3));

        // retry_pred with each choice should succeed
        assert_eq!(pred.retry_pred(&mut ctx, 0, 0), PredicateResult::Success); // value 1
        assert_eq!(pred.retry_pred(&mut ctx, 0, 1), PredicateResult::Success); // value 2
        assert_eq!(pred.retry_pred(&mut ctx, 0, 2), PredicateResult::Success); // value 3
        assert_eq!(pred.retry_pred(&mut ctx, 0, 3), PredicateResult::Failure); // past end
    }

    #[test]
    fn test_choice_predicate() {
        let mut ctx = SearchContext::new();
        let mut pred = ChoicePredicate::new(vec!["A", "B", "C"]);

        // try_pred should return Choices(3)
        assert_eq!(pred.try_pred(&mut ctx, 0), PredicateResult::Choices(3));

        // retry_pred with each choice should succeed
        assert_eq!(pred.retry_pred(&mut ctx, 0, 0), PredicateResult::Success);
        assert_eq!(pred.retry_pred(&mut ctx, 0, 1), PredicateResult::Success);
        assert_eq!(pred.retry_pred(&mut ctx, 0, 2), PredicateResult::Success);
        assert_eq!(pred.retry_pred(&mut ctx, 0, 3), PredicateResult::Failure);
    }

    #[test]
    fn test_empty_choice_predicate() {
        let mut ctx = SearchContext::new();
        let mut pred: ChoicePredicate<i32> = ChoicePredicate::new(vec![]);

        // Should fail immediately with empty options
        assert_eq!(pred.try_pred(&mut ctx, 0), PredicateResult::Failure);
    }

    #[test]
    fn test_suspend_predicate() {
        let mut ctx = SearchContext::new();
        let mut pred = SuspendPredicate;

        // Should suspend immediately
        assert_eq!(pred.try_pred(&mut ctx, 0), PredicateResult::Suspend);

        // Retry should fail
        assert_eq!(pred.retry_pred(&mut ctx, 0, 0), PredicateResult::Failure);
    }

    #[test]
    fn test_fail_predicate() {
        let mut ctx = SearchContext::new();
        let mut pred = AlwaysFailPredicate; // Re-exported from FailPredicate

        // Should always fail
        assert_eq!(pred.try_pred(&mut ctx, 0), PredicateResult::Failure);
        assert_eq!(pred.retry_pred(&mut ctx, 0, 0), PredicateResult::Failure);
    }

    #[test]
    fn test_multi_round_predicate() {
        let mut ctx = SearchContext::new();
        let mut pred = MultiRoundPredicate::new(3);

        // Round 0 and 1 should return SuccessSamePredicate
        assert_eq!(pred.try_pred(&mut ctx, 0), PredicateResult::SuccessSamePredicate);
        assert_eq!(pred.try_pred(&mut ctx, 1), PredicateResult::SuccessSamePredicate);

        // Round 2 (last) should return Success
        assert_eq!(pred.try_pred(&mut ctx, 2), PredicateResult::Success);

        // Past last round should fail
        assert_eq!(pred.try_pred(&mut ctx, 3), PredicateResult::Failure);
    }
}
