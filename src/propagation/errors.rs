// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Error types for constraint propagation.

use std::fmt;
use strum_macros::EnumCount as EnumCountMacro;

/// Errors that can occur during constraint propagation.
#[derive(Debug, Clone, PartialEq, Eq, EnumCountMacro)]
pub enum PropagationFailure {
    /// Face has no remaining possible cycles after constraint propagation.
    NoMatchingCycles { face_id: usize, depth: usize },

    /// Face is already assigned a cycle that conflicts with new constraints.
    ConflictingConstraints {
        face_id: usize,
        assigned_cycle: u64,
        depth: usize,
    },

    /// Propagation depth exceeded (likely infinite recursion bug).
    DepthExceeded { depth: usize },

    /// Crossing limit exceeded between a color pair (triangle constraint violation).
    CrossingLimitExceeded {
        color_i: usize,
        color_j: usize,
        count: usize,
        max_allowed: usize,
        depth: usize,
    },

    /// Too many corners detected on a curve (triangle constraint violation).
    /// Triangles have at most 3 corners per curve.
    TooManyCorners {
        color: usize,
        corner_count: usize,
        max_allowed: usize,
        depth: usize,
    },

    /// Disconnected curve detected (curve forms multiple separate loops).
    /// A curve should form a single connected component visiting all its edges.
    DisconnectedCurve {
        color: usize,
        edges_visited: usize,
        total_edges: usize,
        depth: usize,
    },
}

impl fmt::Display for PropagationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropagationFailure::NoMatchingCycles { face_id, depth } => {
                write!(
                    f,
                    "Face {} has no matching cycles (depth {})",
                    face_id, depth
                )
            }
            PropagationFailure::ConflictingConstraints {
                face_id,
                assigned_cycle,
                depth,
            } => {
                write!(
                    f,
                    "Face {} assigned cycle {} conflicts with constraints (depth {})",
                    face_id, assigned_cycle, depth
                )
            }
            PropagationFailure::DepthExceeded { depth } => {
                write!(f, "Propagation depth {} exceeded max", depth)
            }
            PropagationFailure::CrossingLimitExceeded {
                color_i,
                color_j,
                count,
                max_allowed,
                depth,
            } => {
                write!(
                    f,
                    "Colors {} and {} cross {} times (max {}) (depth {})",
                    color_i, color_j, count, max_allowed, depth
                )
            }
            PropagationFailure::TooManyCorners {
                color,
                corner_count,
                max_allowed,
                depth,
            } => {
                write!(
                    f,
                    "Color {} requires {} corners (max {} for triangles) (depth {})",
                    color, corner_count, max_allowed, depth
                )
            }
            PropagationFailure::DisconnectedCurve {
                color,
                edges_visited,
                total_edges,
                depth,
            } => {
                write!(
                    f,
                    "Color {} curve is disconnected: visited {} edges but {} total (depth {})",
                    color, edges_visited, total_edges, depth
                )
            }
        }
    }
}
