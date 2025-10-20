// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Common test utilities shared across integration tests.

use venn_search::context::SearchContext;
use venn_search::{propagation, Predicate, PredicateResult};

/// A predicate that fixes the inner face to a specific degree sequence.
///
/// This is used in tests to search for Venn diagrams with a known
/// degree sequence, bypassing the InnerFacePredicate enumeration.
#[derive(Debug)]
pub struct FixedInnerFacePredicate<const N: usize>(pub [u64; N]);

impl<const N: usize> Predicate for FixedInnerFacePredicate<N> {
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
        ctx.state.current_face_degrees = self.0;
        PredicateResult::Success
    }

    fn name(&self) -> &str {
        "FixedInnerFace"
    }
}
