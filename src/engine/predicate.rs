// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Predicate trait for non-deterministic search.
//!
//! The search engine works by trying predicates in sequence. Each predicate
//! represents a choice point in the search space. Predicates can succeed,
//! fail, or signal completion.
//!
//! # Example
//!
//! ```
//! use venn_search::engine::{Predicate, PredicateResult};
//! use venn_search::context::SearchContext;
//!
//! struct SimplePredicate;
//!
//! impl Predicate for SimplePredicate {
//!     fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
//!         // Return Choices(2) to try two alternatives
//!         PredicateResult::Choices(2)
//!     }
//!
//!     fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, choice: usize) -> PredicateResult {
//!         if choice < 2 {
//!             PredicateResult::Success  // This choice works
//!         } else {
//!             PredicateResult::Failure  // No more options
//!         }
//!     }
//! }
//! ```

use crate::context::SearchContext;

/// Result of attempting a predicate.
///
/// Based on the C engine model, predicates can return:
/// - `Success`: Move to next predicate in the sequence
/// - `SuccessSamePredicate`: Stay at same predicate, increment round (for iterative predicates)
/// - `Failure`: Backtrack to previous predicate
/// - `Choices(n)`: Predicate has n choices to explore via retry_pred
/// - `Suspend`: Pause execution for testing/inspection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateResult {
    /// Predicate succeeded. Move to next predicate in sequence.
    Success,

    /// Predicate succeeded but stay at same predicate with next round.
    /// Allows a single predicate to execute multiple times (e.g., once per face).
    SuccessSamePredicate,

    /// Predicate has no (more) valid choices. Backtrack to previous predicate.
    Failure,

    /// Predicate has multiple choices to explore.
    /// Engine will call retry_pred(round, choice) for each choice in 0..n.
    Choices(usize),

    /// Suspend execution. Engine returns control with state preserved.
    /// Useful for testing and inspection of intermediate states.
    Suspend,
}

/// A terminal predicate that ends a WAM program.
///
/// Terminal predicates are FAIL or SUSPEND - they never return Success.
/// This marker trait ensures that SearchEngine sequences always end properly.
///
/// The type system uses this trait to enforce that every predicate sequence
/// ends with a terminal predicate, preventing invalid programs at compile time.
pub trait TerminalPredicate: Predicate {}

/// Trait for search predicates in the non-deterministic engine.
///
/// Each predicate represents a choice point in the search. The engine
/// calls `try_pred` to attempt the predicate for the first time, and
/// `retry_pred` on backtracking to try alternative choices.
///
/// # Lifecycle
///
/// 1. Engine calls `try_pred` when first encountering the predicate
/// 2. If Success: engine advances to next predicate
/// 3. If Failure: engine backtracks to previous predicate
/// 4. On backtrack: engine calls `retry_pred` to try next option
/// 5. Repeat until Success (advance) or Failure (backtrack)
///
/// # Trail Integration
///
/// Predicates can modify `SearchContext` state. The engine automatically:
/// - Calls `trail.checkpoint()` before `try_pred` or `retry_pred`
/// - Calls `trail.rewind()` on failure to restore state
///
/// # Example: Choice Predicate
///
/// ```
/// use venn_search::engine::{Predicate, PredicateResult};
/// use venn_search::context::SearchContext;
///
/// struct ChoicePredicate {
///     options: Vec<i32>,
/// }
///
/// impl ChoicePredicate {
///     fn new(options: Vec<i32>) -> Self {
///         Self { options }
///     }
/// }
///
/// impl Predicate for ChoicePredicate {
///     fn try_pred(&mut self, _ctx: &mut SearchContext, _round: usize) -> PredicateResult {
///         if self.options.is_empty() {
///             PredicateResult::Failure
///         } else {
///             PredicateResult::Choices(self.options.len())
///         }
///     }
///
///     fn retry_pred(&mut self, _ctx: &mut SearchContext, _round: usize, choice: usize) -> PredicateResult {
///         if choice < self.options.len() {
///             // Could use self.options[choice] to set state in ctx
///             PredicateResult::Success
///         } else {
///             PredicateResult::Failure
///         }
///     }
/// }
/// ```
pub trait Predicate {
    /// Try this predicate for a given round.
    ///
    /// Called when the search engine executes this predicate for round `round`.
    /// The round starts at 0 and increments each time the predicate returns
    /// `SuccessSamePredicate`.
    ///
    /// Can return:
    /// - `Success`: Move to next predicate in sequence
    /// - `SuccessSamePredicate`: Stay at this predicate, increment round
    /// - `Failure`: Backtrack to previous predicate
    /// - `Choices(n)`: Enter choice mode, engine will call retry_pred for each choice
    /// - `Suspend`: Pause execution (for testing)
    ///
    /// The predicate can modify `ctx` state. Changes are automatically
    /// recorded on the trail and restored on backtrack.
    fn try_pred(&mut self, ctx: &mut SearchContext, round: usize) -> PredicateResult;

    /// Retry this predicate with a specific choice.
    ///
    /// Called after try_pred returns Choices(n), for each choice in 0..n.
    /// The engine will try choice 0, then on backtrack try choice 1, etc.
    ///
    /// Can return:
    /// - `Success`: This choice succeeded, move to next predicate
    /// - `SuccessSamePredicate`: This choice succeeded, stay at this predicate
    /// - `Failure`: This choice failed, try next choice (or backtrack if no more)
    ///
    /// Note: retry_pred cannot return Choices or Suspend (we're already in choice mode).
    ///
    /// The trail has already been rewound to the state before this choice was tried.
    fn retry_pred(&mut self, ctx: &mut SearchContext, round: usize, choice: usize) -> PredicateResult;

    /// Optional: Get a name for this predicate (for debugging).
    ///
    /// Default implementation returns the type name.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}
