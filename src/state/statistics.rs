// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Statistics
//!
//! Statistics are stored in the context, and can be incremented by special predicates,
//! or by additional methods.

use crate::context::SearchContext;
use crate::engine::{Predicate, PredicateResult};
use crate::propagation::PropagationFailure;
use strum::EnumCount;
use strum_macros::EnumCount as EnumCountMacro;

#[derive(EnumCountMacro, Copy, Clone)]
#[repr(u8)]
pub enum Counters {
    VennSolutions,
    EquivocalSolutions,
    InnerFaceSolutions,
}

const COUNT: usize = Counters::COUNT + PropagationFailure::COUNT;

#[derive(Debug, Default)]
pub struct Statistics {
    stats: [u64; COUNT],
}

impl Statistics {
    pub fn new() -> Self {
        Statistics::default()
    }
    /// A predicate that will increment the given counter, whenever a condition holds (or always).
    pub fn counting_predicate(
        counter: Counters,
        filter: Option<fn(&SearchContext) -> bool>,
    ) -> Box<dyn Predicate> {
        Box::new(CountingPredicate {
            filter: filter.unwrap_or(|_ctxt| true),
            counter,
        })
    }
    /// Increment the specified counter by 1.
    fn increment_counter(&mut self, counter: Counters) {
        self.stats[counter as usize] += 1;
    }

    /// Get the current value of the specified counter.
    pub fn get(&self, counter: Counters) -> u64 {
        self.stats[counter as usize]
    }
}

struct CountingPredicate {
    filter: fn(&SearchContext) -> bool,
    counter: Counters,
}

impl Predicate for CountingPredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        if (self.filter)(ctx) {
            let statistics = &mut ctx.statistics;
            statistics.increment_counter(self.counter);
        }
        PredicateResult::Success
    }
}
