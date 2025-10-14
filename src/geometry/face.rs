// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Face type for representing regions in Venn diagrams.
//!
//! A face is a region in the diagram enclosed by edges of different colors.
//! Each face is identified by a set of colors (which curves bound it).
//!
//! # Phase 2 Skeleton
//!
//! This is a skeleton implementation providing the basic type structure.
//! Full initialization including edge connectivity and cycle constraints
//! will be implemented in Phase 3 as part of the Initialize predicate.

use crate::geometry::{ColorSet, CycleId, CycleSet, EdgeId};

/// Unique identifier for a face.
///
/// Face IDs correspond to the bitset of colors defining the face.
/// For NCOLORS=6, there are NFACES=64 possible faces (2^6).
pub type FaceId = usize;

/// A face (region) in the Venn diagram.
///
/// Each face is uniquely identified by its color set - the set of curves
/// that bound it. Faces with k colors correspond to the intersection of
/// k curves in the Venn diagram.
///
/// # Structure
///
/// - The **colors** field identifies which curves bound this face
/// - The **edges** array contains the edge IDs forming the boundary
/// - The **possible_cycles** tracks which facial cycles could bound this face
/// - The **adjacent_faces** array points to neighboring faces (one per color)
#[derive(Debug, Clone)]
pub struct Face {
    /// Unique identifier for this face (corresponds to color bitset value).
    pub id: FaceId,

    /// The set of colors defining this face.
    ///
    /// A face with colors {a, b, c} represents the region where curves
    /// a, b, and c all overlap (inside all three curves).
    pub colors: ColorSet,

    /// Edge IDs forming the boundary of this face, indexed by color.
    ///
    /// `edges[i]` is the edge of color i that bounds this face.
    /// Not all entries may be valid - only colors in the `colors` set
    /// and colors of adjacent faces have edges.
    pub edges: Vec<EdgeId>,

    /// IDs of adjacent faces, indexed by color.
    ///
    /// `adjacent_faces[i]` is the face you reach by crossing the edge
    /// of color i. Adjacent faces differ from this face by exactly one color.
    pub adjacent_faces: Vec<Option<FaceId>>,

    /// Set of possible facial cycles for this face.
    ///
    /// During search, this starts with all cycles containing the right colors,
    /// then gets constrained as edges are connected. When only one cycle remains,
    /// the face is fully determined.
    pub possible_cycles: CycleSet,
}

impl Face {
    /// Create a new face (used during initialization).
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier (typically the color set as a bitset value)
    /// * `colors` - The set of colors defining this face
    ///
    /// # Note
    ///
    /// Edge and adjacent face arrays are initialized with placeholder values
    /// and filled in during the full initialization phase.
    pub fn new(id: FaceId, colors: ColorSet) -> Self {
        let ncolors = crate::geometry::constants::NCOLORS;

        Self {
            id,
            colors,
            edges: vec![0; ncolors], // Placeholder edge IDs
            adjacent_faces: vec![None; ncolors],
            possible_cycles: CycleSet::full(), // Start with all cycles possible
        }
    }

    /// Get the number of colors defining this face.
    pub fn num_colors(&self) -> usize {
        self.colors.len()
    }

    /// Check if this is the central face (empty color set).
    ///
    /// The central face is the region outside all curves.
    pub fn is_central(&self) -> bool {
        self.colors.is_empty()
    }

    /// Check if this is a maximal face (all colors).
    ///
    /// The maximal face is the region inside all curves.
    pub fn is_maximal(&self) -> bool {
        self.colors.len() == crate::geometry::constants::NCOLORS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Color;

    #[test]
    fn test_face_creation() {
        let colors = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let face = Face::new(7, colors); // ID 7 = 0b111 for {a,b,c}

        assert_eq!(face.id, 7);
        assert_eq!(face.colors, colors);
        assert_eq!(face.num_colors(), 3);
    }

    #[test]
    fn test_central_face() {
        let central = Face::new(0, ColorSet::empty());
        assert!(central.is_central());
        assert!(!central.is_maximal());
        assert_eq!(central.num_colors(), 0);
    }

    #[test]
    fn test_maximal_face() {
        let all_colors = ColorSet::full();
        let maximal = Face::new(0, all_colors);

        assert!(maximal.is_maximal());
        assert!(!maximal.is_central());
        assert_eq!(maximal.num_colors(), crate::geometry::constants::NCOLORS);
    }

    #[test]
    fn test_face_with_different_sizes() {
        // 1-color face
        let face1 = Face::new(1, ColorSet::from_colors(&[Color::new(0)]));
        assert_eq!(face1.num_colors(), 1);

        // 2-color face
        let face2 = Face::new(3, ColorSet::from_colors(&[Color::new(0), Color::new(1)]));
        assert_eq!(face2.num_colors(), 2);

        // 3-color face
        let face3 = Face::new(7, ColorSet::from_colors(&[
            Color::new(0),
            Color::new(1),
            Color::new(2),
        ]));
        assert_eq!(face3.num_colors(), 3);
    }

    #[test]
    fn test_possible_cycles_initialized() {
        let colors = ColorSet::from_colors(&[Color::new(0), Color::new(1), Color::new(2)]);
        let face = Face::new(7, colors);

        // Should start with full set of possible cycles
        assert!(!face.possible_cycles.is_empty());
    }
}
