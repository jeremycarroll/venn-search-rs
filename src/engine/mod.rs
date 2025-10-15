// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Non-deterministic search engine.
//!
//! This module implements a backtracking search engine that runs predicates
//! in sequence. The engine coordinates with the trail system to provide
//! automatic state restoration on backtracking.
//!
//! # Architecture
//!
//! The engine maintains a stack of predicate execution states. Each stack entry tracks:
//! - Which predicate is executing
//! - Current round number (for predicates that execute multiple times)
//! - Choice mode state (whether we're trying alternatives)
//! - Current choice index (when in choice mode)
//!
//! The engine follows the C implementation's WAM-like execution model:
//! 1. Call try_pred(round) on each predicate
//! 2. If Success: advance to next predicate
//! 3. If SuccessSamePredicate: increment round, stay at same predicate
//! 4. If Choices(n): enter choice mode, call retry_pred(round, 0..n-1)
//! 5. If Failure: backtrack to previous stack entry
//! 6. If Suspend: pause and return control to caller
//!
//! # Example
//!
//! ```no_run
//! use venn_search::engine::{SearchEngine, Predicate, PredicateResult};
//! use venn_search::context::SearchContext;
//!
//! struct SimplePredicate;
//!
//! impl Predicate for SimplePredicate {
//!     fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
//!         PredicateResult::Success
//!     }
//!
//!     fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, _choice: usize) -> PredicateResult {
//!         PredicateResult::Failure
//!     }
//! }
//!
//! let mut ctx = SearchContext::new();
//! let mut engine = SearchEngine::new(vec![Box::new(SimplePredicate)]);
//! let found = engine.search(&mut ctx);
//! ```

pub mod predicate;

pub use predicate::{Predicate, PredicateResult};

use crate::context::SearchContext;

/// Maximum depth of the predicate stack.
const MAX_STACK_SIZE: usize = 1000;

/// Stack entry tracking the state of one predicate execution.
#[derive(Debug)]
struct StackEntry {
    /// Index of the predicate in the predicates list.
    predicate_index: usize,

    /// Current round number (incremented by SuccessSamePredicate).
    round: usize,

    /// Whether we're in choice mode (exploring alternatives).
    in_choice_mode: bool,

    /// Current choice being tried (when in_choice_mode is true).
    current_choice: usize,

    /// Total number of choices (when in_choice_mode is true).
    num_choices: usize,

    /// Trail checkpoint for this stack entry.
    trail_checkpoint: usize,
}

/// Search engine that coordinates predicate execution and backtracking.
///
/// The engine runs predicates in sequence, managing rounds, choices, and
/// backtracking automatically via the trail system.
pub struct SearchEngine {
    /// List of predicates to execute in sequence.
    predicates: Vec<Box<dyn Predicate>>,

    /// Stack of predicate execution states.
    stack: Vec<StackEntry>,

    /// Statistics: number of try_pred calls.
    try_count: u64,

    /// Statistics: number of retry_pred calls (backtracks).
    retry_count: u64,
}

impl SearchEngine {
    /// Create a new search engine with the given predicates.
    ///
    /// Predicates will be tried in the order given. The search terminates when:
    /// - A predicate returns Suspend (paused for inspection)
    /// - All predicates complete and we backtrack past the first predicate (failure)
    /// - A terminal predicate signals completion
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use venn_search::engine::{SearchEngine, Predicate, PredicateResult};
    /// # use venn_search::context::SearchContext;
    /// # struct P1; impl Predicate for P1 {
    /// #     fn try_pred(&mut self, _: &mut SearchContext, _: usize) -> PredicateResult { PredicateResult::Success }
    /// #     fn retry_pred(&mut self, _: &mut SearchContext, _: usize, _: usize) -> PredicateResult { PredicateResult::Failure }
    /// # }
    /// let engine = SearchEngine::new(vec![Box::new(P1)]);
    /// ```
    pub fn new(predicates: Vec<Box<dyn Predicate>>) -> Self {
        Self {
            predicates,
            stack: Vec::with_capacity(MAX_STACK_SIZE),
            try_count: 0,
            retry_count: 0,
        }
    }

    /// Run the search to find one solution.
    ///
    /// Returns:
    /// - `Ok(true)` if search completed successfully
    /// - `Ok(false)` if search exhausted all possibilities without finding a solution
    /// - `Err(())` if search was suspended (PREDICATE_SUSPEND)
    ///
    /// The search modifies `ctx` with the solution state if found.
    #[allow(clippy::result_unit_err)]
    pub fn search(&mut self, ctx: &mut SearchContext) -> Result<bool, ()> {
        // Initialize with first predicate
        self.stack.clear();
        self.try_count = 0;
        self.retry_count = 0;

        if self.predicates.is_empty() {
            return Ok(false);
        }

        // Push initial stack entry for first predicate
        self.stack.push(StackEntry {
            predicate_index: 0,
            round: 0,
            in_choice_mode: false,
            current_choice: 0,
            num_choices: 0,
            trail_checkpoint: ctx.trail.len(),
        });

        // Main execution loop
        loop {
            // Check if we've backtracked past the first predicate
            if self.stack.is_empty() {
                return Ok(false); // Search exhausted
            }

            let entry = self.stack.last_mut().unwrap();

            // Rewind trail to this entry's checkpoint
            ctx.trail.rewind_to(entry.trail_checkpoint);

            if !entry.in_choice_mode {
                // Call mode: try_pred
                let result = {
                    let pred_idx = entry.predicate_index;
                    let round = entry.round;
                    self.try_count += 1;
                    self.predicates[pred_idx].try_pred(ctx, round)
                };

                match result {
                    PredicateResult::Success => {
                        // Move to next predicate (or complete if at end)
                        if !self.push_next_predicate(ctx) {
                            return Ok(true); // Reached end of sequence!
                        }
                    }
                    PredicateResult::SuccessSamePredicate => {
                        // Stay at same predicate, increment round
                        self.push_same_predicate(ctx);
                    }
                    PredicateResult::Failure => {
                        // Backtrack
                        self.stack.pop();
                    }
                    PredicateResult::Choices(n) => {
                        // Enter choice mode
                        let entry = self.stack.last_mut().unwrap();
                        entry.in_choice_mode = true;
                        entry.current_choice = 0;
                        entry.num_choices = n;
                        entry.trail_checkpoint = ctx.trail.len();
                    }
                    PredicateResult::Suspend => {
                        // Pause execution
                        return Err(());
                    }
                }
            } else {
                // Choice mode: retry_pred
                let entry = self.stack.last_mut().unwrap();

                // Check if we've exhausted all choices
                if entry.current_choice >= entry.num_choices {
                    // Backtrack
                    self.stack.pop();
                    continue;
                }

                let result = {
                    let pred_idx = entry.predicate_index;
                    let round = entry.round;
                    let choice = entry.current_choice;
                    entry.current_choice += 1;
                    self.retry_count += 1;
                    self.predicates[pred_idx].retry_pred(ctx, round, choice)
                };

                match result {
                    PredicateResult::Success => {
                        // Move to next predicate (or complete if at end)
                        if !self.push_next_predicate(ctx) {
                            return Ok(true); // Reached end of sequence!
                        }
                    }
                    PredicateResult::SuccessSamePredicate => {
                        // Stay at same predicate, increment round
                        self.push_same_predicate(ctx);
                    }
                    PredicateResult::Failure => {
                        // Try next choice (loop continues)
                    }
                    PredicateResult::Choices(_) | PredicateResult::Suspend => {
                        // Invalid: retry_pred cannot return Choices or Suspend
                        panic!("retry_pred returned invalid result: {:?}", result);
                    }
                }
            }
        }
    }

    /// Push a new stack entry for the next predicate in sequence.
    ///
    /// Returns true if a stack entry was pushed, false if we reached the end of the sequence.
    fn push_next_predicate(&mut self, ctx: &mut SearchContext) -> bool {
        let current = self.stack.last().unwrap();
        let next_index = current.predicate_index + 1;

        if next_index >= self.predicates.len() {
            // Reached end of predicate sequence!
            return false;
        }

        self.stack.push(StackEntry {
            predicate_index: next_index,
            round: 0,
            in_choice_mode: false,
            current_choice: 0,
            num_choices: 0,
            trail_checkpoint: ctx.trail.len(),
        });
        true
    }

    /// Push a new stack entry for the same predicate with incremented round.
    fn push_same_predicate(&mut self, ctx: &mut SearchContext) {
        let current = self.stack.last().unwrap();
        let next_round = current.round + 1;
        let pred_index = current.predicate_index;

        self.stack.push(StackEntry {
            predicate_index: pred_index,
            round: next_round,
            in_choice_mode: false,
            current_choice: 0,
            num_choices: 0,
            trail_checkpoint: ctx.trail.len(),
        });
    }

    /// Get statistics about the search.
    ///
    /// Returns (try_count, retry_count) showing how many times predicates
    /// were tried and retried.
    pub fn statistics(&self) -> (u64, u64) {
        (self.try_count, self.retry_count)
    }

    /// Reset the engine to initial state.
    ///
    /// Clears statistics and resets the stack.
    pub fn reset(&mut self) {
        self.stack.clear();
        self.try_count = 0;
        self.retry_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test predicate that always succeeds.
    struct AlwaysSucceed;

    impl Predicate for AlwaysSucceed {
        fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
            PredicateResult::Success
        }

        fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, _choice: usize) -> PredicateResult {
            PredicateResult::Failure // No retry
        }
    }

    /// Test predicate that always fails.
    struct AlwaysFail;

    impl Predicate for AlwaysFail {
        fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
            PredicateResult::Failure
        }

        fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, _choice: usize) -> PredicateResult {
            PredicateResult::Failure
        }
    }

    #[test]
    fn test_simple_success() {
        let mut ctx = SearchContext::new();
        let mut engine = SearchEngine::new(vec![
            Box::new(AlwaysSucceed),
        ]);

        assert_eq!(engine.search(&mut ctx), Ok(true));
        let (tries, retries) = engine.statistics();
        assert_eq!(tries, 1); // One try_pred call
        assert_eq!(retries, 0); // No backtracks
    }

    #[test]
    fn test_immediate_failure() {
        let mut ctx = SearchContext::new();
        let mut engine = SearchEngine::new(vec![
            Box::new(AlwaysFail),
        ]);

        assert_eq!(engine.search(&mut ctx), Ok(false));
        let (tries, retries) = engine.statistics();
        assert_eq!(tries, 1); // First predicate tried once
        assert_eq!(retries, 0); // Failed immediately, no retry
    }

    #[test]
    fn test_empty_predicates() {
        let mut ctx = SearchContext::new();
        let mut engine = SearchEngine::new(vec![]);

        assert_eq!(engine.search(&mut ctx), Ok(false));
    }
}
