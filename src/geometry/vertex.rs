// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Vertex type for representing crossing points between curves.
//!
//! A vertex represents the intersection point where two curves cross.
//! Each vertex has four incoming edges (two from each curve) and maintains
//! information about which curves cross and their orientation.

use crate::geometry::{Color, ColorSet, EdgeId};

/// Unique identifier for a vertex.
///
/// Vertices are numbered sequentially during initialization.
/// For NCOLORS=6, there are 480 possible vertices (NPOINTS constant).
pub type VertexId = usize;

/// A vertex at the crossing of two curves.
///
/// When two curves cross, they create a vertex with four incoming edges:
/// two edges from each curve approaching the vertex.
///
/// # Orientation
///
/// The vertex has two distinguished colors:
/// - **Primary**: The curve that crosses from inside the secondary to outside
/// - **Secondary**: The curve that crosses from outside the primary to inside
///
/// This orientation determines the order of incoming edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vertex {
    /// Unique identifier for this vertex.
    pub id: VertexId,

    /// The primary color (curve that crosses from inside secondary to outside).
    pub primary: Color,

    /// The secondary color (curve that crosses from outside primary to inside).
    pub secondary: Color,

    /// Bitset of the two colors that cross at this vertex.
    pub colors: ColorSet,

    /// The four edges that enter this vertex.
    ///
    /// If vertex is between crossing of curve A (primary) and curve B (secondary):
    /// - `incoming_edges[0]`: Color A edge running into the vertex
    /// - `incoming_edges[1]`: Counterclockwise color A edge into the vertex
    /// - `incoming_edges[2]`: Color B edge running into the vertex
    /// - `incoming_edges[3]`: Counterclockwise color B edge into the vertex
    ///
    /// The outgoing edges are found by following the reversed edge IDs.
    pub incoming_edges: [EdgeId; 4],
}

impl Vertex {
    /// Create a new vertex (used during initialization).
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this vertex
    /// * `primary` - Primary color (crosses from inside secondary to outside)
    /// * `secondary` - Secondary color (crosses from outside primary to inside)
    /// * `incoming_edges` - Array of 4 edge IDs entering this vertex
    ///
    /// # Note
    ///
    /// Vertices are created during initialization after edges are set up.
    /// The incoming edge array follows the convention documented in the struct.
    pub fn new(
        id: VertexId,
        primary: Color,
        secondary: Color,
        incoming_edges: [EdgeId; 4],
    ) -> Self {
        let mut colors = ColorSet::empty();
        colors.insert(primary);
        colors.insert(secondary);

        Self {
            id,
            primary,
            secondary,
            colors,
            incoming_edges,
        }
    }

    /// Get the two colors that cross at this vertex.
    ///
    /// Returns (primary, secondary) color tuple.
    pub fn crossing_colors(&self) -> (Color, Color) {
        (self.primary, self.secondary)
    }

    /// Check if a given color crosses at this vertex.
    pub fn has_color(&self, color: Color) -> bool {
        self.colors.contains(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_creation() {
        let primary = Color::new(0); // Color 'a'
        let secondary = Color::new(1); // Color 'b'
        let incoming = [10, 11, 12, 13]; // Placeholder edge IDs

        let vertex = Vertex::new(0, primary, secondary, incoming);

        assert_eq!(vertex.id, 0);
        assert_eq!(vertex.primary, primary);
        assert_eq!(vertex.secondary, secondary);
        assert_eq!(vertex.incoming_edges, incoming);
    }

    #[test]
    #[cfg(ncolors_min_5)]
    fn test_crossing_colors() {
        let primary = Color::new(2);
        let secondary = Color::new(4);
        let vertex = Vertex::new(5, primary, secondary, [0, 1, 2, 3]);

        let (p, s) = vertex.crossing_colors();
        assert_eq!(p, primary);
        assert_eq!(s, secondary);
    }

    #[test]
    fn test_has_color() {
        let primary = Color::new(0);
        let secondary = Color::new(2);
        let vertex = Vertex::new(0, primary, secondary, [0, 1, 2, 3]);

        assert!(vertex.has_color(Color::new(0)));
        assert!(!vertex.has_color(Color::new(1)));
        assert!(vertex.has_color(Color::new(2)));
    }

    #[test]
    #[cfg(ncolors_min_4)]
    fn test_colors_bitset() {
        let primary = Color::new(1);
        let secondary = Color::new(3);
        let vertex = Vertex::new(0, primary, secondary, [0, 1, 2, 3]);

        // ColorSet should contain exactly the two colors
        assert_eq!(vertex.colors.len(), 2);
        assert!(vertex.colors.contains(primary));
        assert!(vertex.colors.contains(secondary));

        // Should not contain other colors
        assert!(!vertex.colors.contains(Color::new(0)));
        assert!(!vertex.colors.contains(Color::new(2)));
    }
}
