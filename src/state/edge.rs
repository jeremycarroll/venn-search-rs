// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! DYNAMIC (mutable, trail-tracked) edge state.
//!
//! This module contains EdgeDynamic and related encoding functions for
//! trail-tracked edge vertex connections. EdgeDynamic is paired with
//! EdgeMemo from geometry::edge (which contains immutable precomputed data).

use crate::geometry::edge::{CurveLink, EdgeRef};

/// Sentinel bit used to distinguish None from Some in CurveLink encoding.
///
/// When bit 63 is set, the value represents Some(CurveLink). When clear (value is 0),
/// it represents None. This is a standard tagged pointer pattern.
const CURVELINK_SOME_BIT: u64 = 1 << 63;

/// DYNAMIC (mutable, trail-tracked) edge data.
///
/// DynamicEdge contains runtime state that changes during search:
/// - Current vertex connection (set during vertex configuration)
///
/// Each DynamicFace has NCOLORS DynamicEdge structures in its edge_dynamic array.
/// All modifications must be trail-tracked for backtracking.
#[derive(Debug, Clone, Copy)]
pub struct DynamicEdge {
    /// Encoded vertex connection (set during search, trail-tracked)
    ///
    /// This is one of the possible connections from EdgeMemo.possibly_to,
    /// selected when the edge's endpoint vertex is determined during search.
    ///
    /// **Encoding**: Uses u64 for trail compatibility:
    /// - 0 = None
    /// - bit 63 set = Some(CurveLink) with data in lower bits:
    ///   - bits 0-5: face_id (6 bits, supports 0-63)
    ///   - bits 6-8: color_idx (3 bits, supports 0-7)
    ///   - bits 9-17: vertex_id (9 bits, supports 0-511)
    ///   - bit 63: sentinel bit (always 1 for Some)
    ///
    /// Use `get_to()` and `encode_to()` accessor methods to work with Option<CurveLink>.
    pub to_encoded: u64,
}

impl DynamicEdge {
    /// Create a new DynamicEdge with no vertex connection.
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
    /// let encoded = DynamicEdge::encode_to(Some(link));
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
/// - bit 63 set = Some(CurveLink) with data in lower bits:
///   - bits 0-5: face_id (6 bits, supports 0-63)
///   - bits 6-8: color_idx (3 bits, supports 0-7)
///   - bits 9-17: vertex_id (9 bits, supports 0-511)
///   - bit 63: sentinel bit (always 1 for Some)
#[inline]
fn encode_curve_link(link: Option<CurveLink>) -> u64 {
    match link {
        None => 0,
        Some(l) => {
            let face_id = l.next.face_id as u64;
            let color_idx = l.next.color_idx as u64;
            let vertex_id = l.vertex_id as u64;

            // Validate bounds to prevent data corruption
            debug_assert!(
                face_id < 64,
                "face_id {} exceeds 6-bit limit (0-63)",
                face_id
            );
            debug_assert!(
                color_idx < 8,
                "color_idx {} exceeds 3-bit limit (0-7)",
                color_idx
            );
            debug_assert!(
                vertex_id < 512,
                "vertex_id {} exceeds 9-bit limit (0-511)",
                vertex_id
            );

            // Pack into u64: face_id (6 bits) | color_idx (3 bits) | vertex_id (9 bits)
            let encoded = (face_id & 0x3F)           // bits 0-5
                | ((color_idx & 0x7) << 6)           // bits 6-8
                | ((vertex_id & 0x1FF) << 9); // bits 9-17

            // Set sentinel bit to mark as Some
            encoded | CURVELINK_SOME_BIT
        }
    }
}

/// Decode u64 to Option<CurveLink>.
#[inline]
fn decode_curve_link(encoded: u64) -> Option<CurveLink> {
    // Check sentinel bit to distinguish None from Some
    if encoded & CURVELINK_SOME_BIT == 0 {
        None
    } else {
        // Mask out sentinel bit to get data
        let value = encoded & !CURVELINK_SOME_BIT;

        // Unpack fields
        let face_id = (value & 0x3F) as usize; // bits 0-5
        let color_idx = ((value >> 6) & 0x7) as usize; // bits 6-8
        let vertex_id = ((value >> 9) & 0x1FF) as usize; // bits 9-17

        Some(CurveLink::new(EdgeRef::new(face_id, color_idx), vertex_id))
    }
}

impl Default for DynamicEdge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_dynamic_default() {
        let edge = DynamicEdge::new();
        assert!(edge.get_to().is_none());
        assert_eq!(edge.to_encoded, 0);

        let edge2 = DynamicEdge::default();
        assert!(edge2.get_to().is_none());
        assert_eq!(edge2.to_encoded, 0);
    }

    #[test]
    fn test_edge_dynamic_encode_decode() {
        // Test None encoding
        let encoded_none = DynamicEdge::encode_to(None);
        assert_eq!(encoded_none, 0);
        assert_eq!(decode_curve_link(encoded_none), None);

        // Test Some(CurveLink) encoding
        let link = CurveLink::new(EdgeRef::new(42, 3), 123);
        let encoded = DynamicEdge::encode_to(Some(link));
        assert_ne!(encoded, 0);

        let decoded = decode_curve_link(encoded);
        assert_eq!(decoded, Some(link));

        // Test with DynamicEdge accessor
        let mut edge = DynamicEdge::new();
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
        let encoded = DynamicEdge::encode_to(Some(link));
        let decoded = decode_curve_link(encoded);

        assert_eq!(decoded, Some(link));
        assert_eq!(decoded.unwrap().next.face_id, max_face);
        assert_eq!(decoded.unwrap().next.color_idx, max_color);
        assert_eq!(decoded.unwrap().vertex_id, max_vertex);
    }
}
