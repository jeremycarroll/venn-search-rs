// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Edge system for representing curve segments in Venn diagrams.
//!
//! Edges are first-class objects with MEMO (precomputed) and DYNAMIC (runtime) parts:
//! - **EdgeMemo**: Immutable precomputed edge data (color, reversed edge, possible vertices)
//! - **EdgeDynamic**: Mutable runtime state (current vertex connection) - trail-tracked
//!
//! Each face has NCOLORS edges (one per color), stored in the edges array in FaceMemo.
//! Edges are referenced via EdgeRef (face_id, color_idx) which provides stable references
//! during initialization while respecting Rust borrowing rules.
//!
//! # Vertex Connections
//!
//! Edges connect to vertices via CurveLink structures:
//! - **possibly_to**: Precomputed array of all possible vertex connections (MEMO)
//! - **to**: Current vertex connection set during search (DYNAMIC, trail-tracked)
//!
//! This structure matches the C implementation's separation of `struct edge` into
//! MEMO and DYNAMIC parts.

use crate::geometry::constants::NCOLORS;
use crate::geometry::{Color, ColorSet};

/// Reference to an edge by its location in the face/color grid.
///
/// EdgeRef provides stable references during MEMO initialization while respecting
/// Rust's borrowing rules. Each edge is uniquely identified by the face it belongs
/// to and the color index within that face's edges array.
///
/// This is necessary because during initialization we need to create cross-references
/// between edges in different faces before all EdgeMemo structures are complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeRef {
    /// Face ID (0..NFACES-1)
    pub face_id: usize,
    /// Color index within face's edges array (0..NCOLORS-1)
    pub color_idx: usize,
}

impl EdgeRef {
    /// Create a new edge reference.
    pub fn new(face_id: usize, color_idx: usize) -> Self {
        Self { face_id, color_idx }
    }
}

/// Connection from an edge to a vertex.
///
/// A CurveLink connects an edge to a vertex via the "next" edge that continues
/// around the vertex. This structure is used in both:
/// - **possibly_to**: All possible vertex connections (MEMO)
/// - **to**: Current vertex connection during search (DYNAMIC)
///
/// Matches C `struct curveLink`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurveLink {
    /// The next edge around the vertex (in counterclockwise order)
    pub next: EdgeRef,
    /// The vertex this edge connects to
    pub vertex_id: usize,
}

impl CurveLink {
    /// Create a new curve link.
    pub fn new(next: EdgeRef, vertex_id: usize) -> Self {
        Self { next, vertex_id }
    }
}

/// MEMO (precomputed, immutable) edge data.
///
/// EdgeMemo contains all precomputed edge information:
/// - Color and face identification
/// - Reversed edge reference
/// - All possible vertex connections
///
/// Each face has NCOLORS EdgeMemo structures in its edges array.
/// EdgeMemo is part of the MEMO tier and never changes after initialization.
///
/// Matches MEMO fields of C `struct edge`.
#[derive(Debug, Clone, Copy)]
pub struct EdgeMemo {
    /// The color (curve) this edge belongs to
    pub color: Color,

    /// The set of colors defining the face this edge bounds
    pub face_colors: ColorSet,

    /// Reference to the reversed edge (same color, adjacent face)
    pub reversed: EdgeRef,

    /// All possible vertex connections (one per color in face)
    ///
    /// During initialization, we precompute all possible ways this edge could
    /// connect to a vertex. During search, one of these is selected and placed
    /// in EdgeDynamic.to.
    ///
    /// Matches C `struct curveLink possiblyTo[NCOLORS]`.
    pub possibly_to: [Option<CurveLink>; NCOLORS],
}

impl EdgeMemo {
    /// Create a new EdgeMemo with no possible connections.
    ///
    /// Use `set_possibly_to` to populate possible vertex connections after
    /// vertices are initialized.
    pub fn new(color: Color, face_colors: ColorSet, reversed: EdgeRef) -> Self {
        Self {
            color,
            face_colors,
            reversed,
            possibly_to: [None; NCOLORS],
        }
    }

    /// Set a possible vertex connection at the given index.
    pub fn set_possibly_to(&mut self, idx: usize, link: Option<CurveLink>) {
        self.possibly_to[idx] = link;
    }

    /// Check if this edge is clockwise around its face.
    ///
    /// An edge is clockwise if its color is a member of the face's color set.
    pub fn is_clockwise(&self) -> bool {
        self.face_colors.contains(self.color)
    }
}

/// DYNAMIC (mutable, trail-tracked) edge data.
///
/// EdgeDynamic contains runtime state that changes during search:
/// - Current vertex connection (set by dynamicCheckFacePoints)
///
/// Each DynamicFace has NCOLORS EdgeDynamic structures in its edge_dynamic array.
/// All modifications must be trail-tracked for backtracking.
///
/// Matches DYNAMIC fields of C `struct edge`.
#[derive(Debug, Clone, Copy)]
pub struct EdgeDynamic {
    /// Encoded vertex connection (set during search, trail-tracked)
    ///
    /// This is one of the possible connections from EdgeMemo.possibly_to,
    /// selected when the edge's endpoint vertex is determined during search.
    ///
    /// **Encoding**: Uses u64 for trail compatibility:
    /// - 0 = None
    /// - non-zero = Some(CurveLink) encoded as:
    ///   - bits 0-5: face_id (6 bits, supports 0-63)
    ///   - bits 6-8: color_idx (3 bits, supports 0-7)
    ///   - bits 9-17: vertex_id (9 bits, supports 0-511)
    ///   - Offset by 1 so 0 is reserved for None
    ///
    /// Use `get_to()` and `set_to()` accessor methods to work with Option<CurveLink>.
    ///
    /// Matches C `DYNAMIC CURVELINK to`.
    pub to_encoded: u64,
}

impl EdgeDynamic {
    /// Create a new EdgeDynamic with no vertex connection.
    pub fn new() -> Self {
        Self { to_encoded: 0 }
    }

    /// Get the current vertex connection.
    #[inline]
    pub fn get_to(&self) -> Option<CurveLink> {
        decode_curve_link(self.to_encoded)
    }

    /// Set the current vertex connection (returns encoded value for trail tracking).
    ///
    /// This method is used with trail tracking like:
    /// ```ignore
    /// let encoded = EdgeDynamic::encode_to(Some(link));
    /// trail.record_and_set(ptr_to_to_encoded, encoded);
    /// ```
    #[inline]
    pub fn encode_to(link: Option<CurveLink>) -> u64 {
        encode_curve_link(link)
    }
}

/// Encode Option<CurveLink> as u64 for trail tracking.
///
/// Encoding:
/// - 0 = None
/// - non-zero = Some(CurveLink) encoded as:
///   - bits 0-5: face_id (6 bits, supports 0-63)
///   - bits 6-8: color_idx (3 bits, supports 0-7)
///   - bits 9-17: vertex_id (9 bits, supports 0-511)
///   - Offset by 1 so 0 is reserved for None
#[inline]
fn encode_curve_link(link: Option<CurveLink>) -> u64 {
    match link {
        None => 0,
        Some(l) => {
            let face_id = l.next.face_id as u64;
            let color_idx = l.next.color_idx as u64;
            let vertex_id = l.vertex_id as u64;

            // Pack into u64: face_id (6 bits) | color_idx (3 bits) | vertex_id (9 bits)
            let encoded = (face_id & 0x3F)           // bits 0-5
                | ((color_idx & 0x7) << 6)           // bits 6-8
                | ((vertex_id & 0x1FF) << 9); // bits 9-17

            // Offset by 1 so 0 is None
            encoded + 1
        }
    }
}

/// Decode u64 to Option<CurveLink>.
#[inline]
fn decode_curve_link(encoded: u64) -> Option<CurveLink> {
    if encoded == 0 {
        None
    } else {
        // Subtract offset
        let value = encoded - 1;

        // Unpack fields
        let face_id = (value & 0x3F) as usize; // bits 0-5
        let color_idx = ((value >> 6) & 0x7) as usize; // bits 6-8
        let vertex_id = ((value >> 9) & 0x1FF) as usize; // bits 9-17

        Some(CurveLink::new(EdgeRef::new(face_id, color_idx), vertex_id))
    }
}

impl Default for EdgeDynamic {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_ref_creation() {
        let edge_ref = EdgeRef::new(5, 2);
        assert_eq!(edge_ref.face_id, 5);
        assert_eq!(edge_ref.color_idx, 2);
    }

    #[test]
    fn test_edge_ref_equality() {
        let ref1 = EdgeRef::new(5, 2);
        let ref2 = EdgeRef::new(5, 2);
        let ref3 = EdgeRef::new(5, 3);

        assert_eq!(ref1, ref2);
        assert_ne!(ref1, ref3);
    }

    #[test]
    fn test_curve_link_creation() {
        let next_edge = EdgeRef::new(10, 1);
        let link = CurveLink::new(next_edge, 42);

        assert_eq!(link.next, next_edge);
        assert_eq!(link.vertex_id, 42);
    }

    #[test]
    fn test_edge_memo_creation() {
        let color = Color::new(0);
        let face_colors = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let reversed = EdgeRef::new(7, 0);

        let edge = EdgeMemo::new(color, face_colors, reversed);

        assert_eq!(edge.color, color);
        assert_eq!(edge.face_colors, face_colors);
        assert_eq!(edge.reversed, reversed);
        assert!(edge.possibly_to.iter().all(|x| x.is_none()));
    }

    #[test]
    fn test_edge_memo_is_clockwise() {
        let color_a = Color::new(0);
        let reversed = EdgeRef::new(0, 0);

        // Edge with color 'a' on face {a, b, c} is clockwise
        let face_abc = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let edge1 = EdgeMemo::new(color_a, face_abc, reversed);
        assert!(edge1.is_clockwise());

        // Edge with color 'a' on face {b, c} is not clockwise
        let face_bc = ColorSet::from_colors(&[Color::new(1), Color::new(2)]);
        let edge2 = EdgeMemo::new(color_a, face_bc, reversed);
        assert!(!edge2.is_clockwise());
    }

    #[test]
    fn test_edge_memo_set_possibly_to() {
        let color = Color::new(0);
        let face_colors = ColorSet::from_colors(&[Color::new(0), Color::new(1)]);
        let reversed = EdgeRef::new(1, 0);

        let mut edge = EdgeMemo::new(color, face_colors, reversed);

        let link = CurveLink::new(EdgeRef::new(2, 1), 10);
        edge.set_possibly_to(0, Some(link));

        assert_eq!(edge.possibly_to[0], Some(link));
        assert_eq!(edge.possibly_to[1], None);
    }

    #[test]
    fn test_edge_dynamic_default() {
        let edge = EdgeDynamic::new();
        assert!(edge.get_to().is_none());
        assert_eq!(edge.to_encoded, 0);

        let edge2 = EdgeDynamic::default();
        assert!(edge2.get_to().is_none());
        assert_eq!(edge2.to_encoded, 0);
    }

    #[test]
    fn test_edge_dynamic_encode_decode() {
        // Test None encoding
        let encoded_none = EdgeDynamic::encode_to(None);
        assert_eq!(encoded_none, 0);
        assert_eq!(decode_curve_link(encoded_none), None);

        // Test Some(CurveLink) encoding
        let link = CurveLink::new(EdgeRef::new(42, 3), 123);
        let encoded = EdgeDynamic::encode_to(Some(link));
        assert_ne!(encoded, 0);

        let decoded = decode_curve_link(encoded);
        assert_eq!(decoded, Some(link));

        // Test with EdgeDynamic accessor
        let mut edge = EdgeDynamic::new();
        edge.to_encoded = encoded;
        assert_eq!(edge.get_to(), Some(link));
    }

    #[test]
    fn test_curve_link_encoding_bounds() {
        // Test maximum values for each field
        let max_face = 63; // 6 bits (0-63)
        let max_color = 5; // 3 bits (0-7, we use 0-5)
        let max_vertex = 479; // 9 bits (0-511, we use 0-479)

        let link = CurveLink::new(EdgeRef::new(max_face, max_color), max_vertex);
        let encoded = EdgeDynamic::encode_to(Some(link));
        let decoded = decode_curve_link(encoded);

        assert_eq!(decoded, Some(link));
        assert_eq!(decoded.unwrap().next.face_id, max_face);
        assert_eq!(decoded.unwrap().next.color_idx, max_color);
        assert_eq!(decoded.unwrap().vertex_id, max_vertex);
    }

    #[test]
    fn test_edge_pair_reversal() {
        // Create a pair of reversed edges
        let color = Color::new(0);
        let face1 = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let face2 = ColorSet::from_colors(&[Color::new(1), Color::new(2)]);

        let ref1 = EdgeRef::new(5, 0);
        let ref2 = EdgeRef::new(3, 0);

        let edge_fwd = EdgeMemo::new(color, face1, ref2);
        let edge_rev = EdgeMemo::new(color, face2, ref1);

        // Verify they reference each other (edges at face 5 and face 3)
        assert_eq!(edge_fwd.reversed, ref2);
        assert_eq!(edge_rev.reversed, ref1);

        // Verify they have the same color
        assert_eq!(edge_fwd.color, edge_rev.color);

        // Verify their face colors differ by exactly one color
        assert!(face1.contains(Color::new(0)));
        assert!(!face2.contains(Color::new(0)));
    }
}
