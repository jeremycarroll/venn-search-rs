// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Common test utilities shared across integration tests.

use venn_search::context::SearchContext;
use venn_search::geometry::constants::NCOLORS;
use venn_search::{propagation, Predicate, PredicateResult};

/// A predicate that fixes the inner face to a specific degree sequence.
///
/// This is used in tests to search for Venn diagrams with a known
/// degree sequence, bypassing the InnerFacePredicate enumeration.
#[derive(Debug)]
pub struct FixedInnerFacePredicate(pub [u64; NCOLORS]);

impl Predicate for FixedInnerFacePredicate {
    fn try_pred(&mut self, ctx: &mut SearchContext, _round: usize) -> PredicateResult {
        let (memo, state) = ctx.parts_mut();
        if let Err(failure) = propagation::setup_central_face(memo, state, &self.0)
        {
            eprintln!(
                "Could not set face degree to {:?}, with {}",
                self.0, failure
            );
            return PredicateResult::Failure;
        }
        for (round, degree) in self.0.iter().copied().enumerate() {
            ctx.set_face_degree(round, degree);
        }
        PredicateResult::Success
    }

    fn name(&self) -> &str {
        "FixedInnerFace"
    }
}
