// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! DYNAMIC (mutable, trail-tracked) edge state.
//!
//! This module contains [`DynamicEdge`] and related encoding functions for
//! trail-tracked edge vertex connections. DynamicEdge is paired with
//! [`crate::geometry::EdgeMemo`] (which contains immutable precomputed data).

use crate::geometry::constants::{NCOLORS, NFACES, NPOINTS};
use crate::geometry::edge::{CurveLink, EdgeRef};

/// Sentinel bit used to distinguish None from Some in CurveLink encoding.
///
/// When bit 63 is set, the value represents Some(CurveLink). When clear (value is 0),
/// it represents None. This tags an indexed connection, not a memory address.
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
    pub(crate) to_encoded: u64,
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

    /// Encode a connection after checking both configured and packed-field bounds.
    /// Search mutations use `TrailedState::set_edge_connection` to record old values.
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
            assert!(
                face_id < NFACES as u64 && face_id < 64,
                "face_id {} outside configured or 6-bit domain",
                face_id
            );
            assert!(
                color_idx < NCOLORS as u64 && color_idx < 8,
                "color_idx {} outside configured or 3-bit domain",
                color_idx
            );
            assert!(
                vertex_id < NPOINTS as u64 && vertex_id < 512,
                "vertex_id {} outside configured or 9-bit domain",
                vertex_id
            );

            // Pack into u64: face_id (6 bits) | color_idx (3 bits) | vertex_id (9 bits)
            let encoded = face_id | (color_idx << 6) | (vertex_id << 9);

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
        let link = CurveLink::new(EdgeRef::new(NFACES - 1, NCOLORS - 1), NPOINTS - 1);
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
    fn rejects_configured_and_packed_field_overflow() {
        for link in [
            CurveLink::new(EdgeRef::new(NFACES, 0), 0),
            CurveLink::new(EdgeRef::new(0, NCOLORS), 0),
            CurveLink::new(EdgeRef::new(0, 0), NPOINTS),
            CurveLink::new(EdgeRef::new(64, 0), 0),
            CurveLink::new(EdgeRef::new(0, 8), 0),
            CurveLink::new(EdgeRef::new(0, 0), 512),
            CurveLink::new(EdgeRef::new(usize::MAX, 0), 0),
        ] {
            assert!(std::panic::catch_unwind(|| DynamicEdge::encode_to(Some(link))).is_err());
        }
    }

    #[test]
    fn zero_fields_are_distinct_from_none() {
        let link = CurveLink::new(EdgeRef::new(0, 0), 0);
        let encoded = DynamicEdge::encode_to(Some(link));
        assert_eq!(encoded, CURVELINK_SOME_BIT);
        assert_eq!(decode_curve_link(encoded), Some(link));
        assert_eq!(decode_curve_link(0), None);
    }

    #[test]
    fn test_curve_link_encoding_bounds() {
        // Test maximum values for each field
        let max_face = NFACES - 1; // 6 bits (0-63)
        let max_color = NCOLORS - 1; // 3 bits (0-7, we use 0-5)
        let max_vertex = NPOINTS - 1; // 9 bits (0-511, we use 0-479)

        let link = CurveLink::new(EdgeRef::new(max_face, max_color), max_vertex);
        let encoded = DynamicEdge::encode_to(Some(link));
        let decoded = decode_curve_link(encoded);

        assert_eq!(decoded, Some(link));
        assert_eq!(decoded.unwrap().next.face_id, max_face);
        assert_eq!(decoded.unwrap().next.color_idx, max_color);
        assert_eq!(decoded.unwrap().vertex_id, max_vertex);
    }
}
