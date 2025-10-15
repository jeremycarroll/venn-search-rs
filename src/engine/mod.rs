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
//! struct SuspendPredicate;
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
//! impl Predicate for SuspendPredicate {
//!     fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
//!         PredicateResult::Suspend
//!     }
//!
//!     fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, _choice: usize) -> PredicateResult {
//!         PredicateResult::Failure
//!     }
//! }
//!
//! let mut ctx = SearchContext::new();
//! // All WAM programs must end with FAIL or SUSPEND
//! let engine = SearchEngine::new(vec![
//!     Box::new(SimplePredicate),
//!     Box::new(SuspendPredicate),  // Terminal predicate
//! ]);
//!
//! // Engine is consumed, returns Some(engine) if suspended
//! if let Some(_engine) = engine.search(&mut ctx) {
//!     // Can resume with _engine.search(&mut ctx)
//! }
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
    /// # struct Suspend; impl Predicate for Suspend {
    /// #     fn try_pred(&mut self, _: &mut SearchContext, _: usize) -> PredicateResult { PredicateResult::Suspend }
    /// #     fn retry_pred(&mut self, _: &mut SearchContext, _: usize, _: usize) -> PredicateResult { PredicateResult::Failure }
    /// # }
    /// let engine = SearchEngine::new(vec![Box::new(P1), Box::new(Suspend)]);
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
    /// Consumes the engine and returns:
    /// - `Some(engine)` if suspended (PREDICATE_SUSPEND) - can resume by calling search() again
    /// - `None` if exhausted (backtracked past first predicate) - search is complete
    ///
    /// The search modifies `ctx` with the solution state. Success is indicated via
    /// side effects (state in `ctx`), not by the return value. This matches the WAM
    /// execution model where programs never "complete" - they either fail or suspend.
    ///
    /// This consuming API enforces correct usage: suspended engines can be resumed,
    /// but exhausted engines are consumed and cannot be accidentally reused.
    ///
    /// # Panics
    ///
    /// Panics if the predicate sequence is invalid (reaches the end without FAIL or SUSPEND).
    /// All valid WAM programs must terminate with a FAIL or SUSPEND predicate.
    ///
    /// # Example
    ///
    /// ```
    /// # use venn_search::engine::{SearchEngine, Predicate, PredicateResult};
    /// # use venn_search::context::SearchContext;
    /// # use venn_search::predicates::test::SuspendPredicate;
    /// # struct MyPredicate;
    /// # impl Predicate for MyPredicate {
    /// #     fn try_pred(&mut self, _: &mut SearchContext, _: usize) -> PredicateResult {
    /// #         PredicateResult::Success
    /// #     }
    /// #     fn retry_pred(&mut self, _: &mut SearchContext, _: usize, _: usize) -> PredicateResult {
    /// #         PredicateResult::Failure
    /// #     }
    /// # }
    /// let mut ctx = SearchContext::new();
    /// let engine = SearchEngine::new(vec![
    ///     Box::new(MyPredicate),
    ///     Box::new(SuspendPredicate),
    /// ]);
    ///
    /// // Engine is consumed, returns Some if suspended
    /// if let Some(engine) = engine.search(&mut ctx) {
    ///     // Can resume the suspended engine
    ///     let _result = engine.search(&mut ctx);
    /// }
    /// // If None, search exhausted - engine is consumed
    /// ```
    pub fn search(mut self, ctx: &mut SearchContext) -> Option<Self> {
        // Initialize with first predicate
        self.stack.clear();
        self.try_count = 0;
        self.retry_count = 0;

        if self.predicates.is_empty() {
            return None;  // Empty is exhausted
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
                return None; // Search exhausted (all choices failed)
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
                        // Move to next predicate
                        self.push_next_predicate(ctx);
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
                        // Pause execution, return engine for resumption
                        return Some(self); // Suspended - can resume
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
                        // Move to next predicate
                        self.push_next_predicate(ctx);
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
    /// Panics if we've reached the end of the predicate sequence, as this indicates
    /// an invalid program (all valid programs must end with FAIL or SUSPEND).
    fn push_next_predicate(&mut self, ctx: &mut SearchContext) {
        let current = self.stack.last().unwrap();
        let next_index = current.predicate_index + 1;

        if next_index >= self.predicates.len() {
            panic!(
                "Invalid predicate sequence: reached end without FAIL or SUSPEND. \
                 All WAM programs must terminate with a FAIL or SUSPEND predicate."
            );
        }

        self.stack.push(StackEntry {
            predicate_index: next_index,
            round: 0,
            in_choice_mode: false,
            current_choice: 0,
            num_choices: 0,
            trail_checkpoint: ctx.trail.len(),
        });
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

    /// Test predicate that suspends.
    struct Suspend;

    impl Predicate for Suspend {
        fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
            PredicateResult::Suspend
        }

        fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, _choice: usize) -> PredicateResult {
            PredicateResult::Failure
        }
    }

    #[test]
    fn test_simple_success_with_suspend() {
        let mut ctx = SearchContext::new();
        let engine = SearchEngine::new(vec![
            Box::new(AlwaysSucceed),
            Box::new(Suspend),  // Terminal predicate
        ]);

        let engine = engine.search(&mut ctx);
        assert!(engine.is_some()); // Suspended - engine returned
        let engine = engine.unwrap();
        assert_eq!(engine.statistics(), (2, 0)); // AlwaysSucceed + Suspend, no retries
    }

    #[test]
    fn test_immediate_failure() {
        let mut ctx = SearchContext::new();
        let engine = SearchEngine::new(vec![
            Box::new(AlwaysFail),
        ]);

        let result = engine.search(&mut ctx);
        assert!(result.is_none()); // Exhausted - engine consumed
    }

    #[test]
    fn test_empty_predicates() {
        let mut ctx = SearchContext::new();
        let engine = SearchEngine::new(vec![]);

        let result = engine.search(&mut ctx);
        assert!(result.is_none()); // Empty is exhausted
    }

    #[test]
    #[should_panic(expected = "Invalid predicate sequence")]
    fn test_invalid_program_without_terminal() {
        let mut ctx = SearchContext::new();
        let engine = SearchEngine::new(vec![
            Box::new(AlwaysSucceed),  // Missing terminal predicate!
        ]);

        let _ = engine.search(&mut ctx); // Should panic
    }
}
