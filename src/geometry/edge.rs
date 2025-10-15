// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Edge type for representing curve segments in Venn diagrams.
//!
//! An edge represents a segment of a curve that forms part of the boundary
//! between two faces. Each edge has a color (which curve it belongs to) and
//! connects to other edges at vertices.
//!
//! # Phase 2 Skeleton
//!
//! This is a skeleton implementation providing the basic type structure.
//! Full initialization and pointer management will be implemented in Phase 3
//! as part of the Initialize predicate.

use crate::geometry::{Color, ColorSet};

/// Unique identifier for an edge.
///
/// Edges are numbered sequentially during initialization.
pub type EdgeId = usize;

/// A directed edge representing a segment of a curve.
///
/// Each edge has a corresponding reversed edge going in the opposite direction.
/// The two edges share the same color but belong to adjacent faces whose
/// color sets differ by exactly one color.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Unique identifier for this edge.
    pub id: EdgeId,

    /// The color (curve) this edge belongs to.
    pub color: Color,

    /// The set of colors defining the face this edge bounds.
    ///
    /// This is the "inside" face when traversing the edge in its forward direction.
    pub face_colors: ColorSet,

    /// ID of the edge going in the reverse direction.
    ///
    /// The reversed edge has the same color but bounds the adjacent face
    /// (whose color set differs by exactly one color).
    pub reversed_id: EdgeId,
}

impl Edge {
    /// Create a new edge (used during initialization).
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this edge
    /// * `color` - The curve color this edge belongs to
    /// * `face_colors` - Colors of the face this edge bounds
    /// * `reversed_id` - ID of the corresponding reversed edge
    ///
    /// # Note
    ///
    /// Edges are typically created in pairs during face initialization,
    /// with each edge knowing its reverse partner's ID.
    pub fn new(id: EdgeId, color: Color, face_colors: ColorSet, reversed_id: EdgeId) -> Self {
        Self {
            id,
            color,
            face_colors,
            reversed_id,
        }
    }

    /// Check if this edge is clockwise around its face.
    ///
    /// An edge is clockwise if its color is a member of the face's color set.
    pub fn is_clockwise(&self) -> bool {
        self.face_colors.contains(self.color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_creation() {
        let color = Color::new(0); // Color 'a'
        let face_colors = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);

        let edge = Edge::new(0, color, face_colors, 1);

        assert_eq!(edge.id, 0);
        assert_eq!(edge.color, color);
        assert_eq!(edge.face_colors, face_colors);
        assert_eq!(edge.reversed_id, 1);
    }

    #[test]
    fn test_is_clockwise() {
        let color_a = Color::new(0);

        // Edge with color 'a' on face {a, b, c} is clockwise
        let face_abc = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let edge1 = Edge::new(0, color_a, face_abc, 1);
        assert!(edge1.is_clockwise());

        // Edge with color 'a' on face {b, c} is not clockwise (edge is on the "outside")
        let face_bc = ColorSet::from_colors(&[Color::new(1), Color::new(2)]);
        let edge2 = Edge::new(2, color_a, face_bc, 3);
        assert!(!edge2.is_clockwise());
    }

    #[test]
    fn test_edge_pair_reversal() {
        // Create a pair of reversed edges
        let color = Color::new(0);
        let face1 = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let face2 = ColorSet::from_colors(&[Color::new(1), Color::new(2)]); // Differs by color 0

        let edge_fwd = Edge::new(0, color, face1, 1);
        let edge_rev = Edge::new(1, color, face2, 0);

        // Verify they reference each other
        assert_eq!(edge_fwd.reversed_id, edge_rev.id);
        assert_eq!(edge_rev.reversed_id, edge_fwd.id);

        // Verify they have the same color
        assert_eq!(edge_fwd.color, edge_rev.color);

        // Verify their face colors differ by exactly one color
        // (face1 has color 0, face2 doesn't)
        assert!(face1.contains(Color::new(0)));
        assert!(!face2.contains(Color::new(0)));
    }
}
