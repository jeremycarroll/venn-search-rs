// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Face-related MEMO data structures.
//!
//! This module computes all immutable face-related data:
//! - Face degree expectations (binomial coefficients)
//! - Face adjacency tables
//! - Cycle-to-face relationship lookups (next/previous by cycle ID)
//! - Monotonicity constraints

use crate::geometry::constants::{CYCLESET_LENGTH, NCOLORS, NFACES};
use crate::geometry::{Color, ColorSet, CycleSet, Face, FaceId};

/// MEMO data for all faces in the diagram.
///
/// This structure is computed once during initialization and contains
/// all precomputed face-related lookup tables needed for efficient
/// constraint propagation.
///
/// # Memory Layout
///
/// - `faces`: **Heap-allocated** Vec (NFACES = 64 for NCOLORS=6)
///   - Reason: Variable size depending on NCOLORS; too large for stack
///   - Size: 64 × sizeof(Face) ≈ 5 KB
///
/// - `face_degree_by_color_count`: **Stack-allocated** array (7 elements for NCOLORS=6)
///   - Reason: Small fixed-size array (NCOLORS+1 elements); efficient on stack
///   - Size: 7 × 8 bytes = 56 bytes
///
/// - Cycle lookup tables: **Stack-allocated** arrays for O(1) constraint checks
///   - `pairs`: 6×6 × 7 u64s ≈ 2 KB
///   - `triples`: 6×6×6 × 7 u64s ≈ 12 KB
///   - `omitting_one`: 6 × 7 u64s ≈ 336 bytes
///   - `omitting_pair`: 6×6 × 7 u64s ≈ 2 KB
///   - Total: ≈16 KB
#[derive(Debug, Clone)]
pub struct FacesMemo {
    /// All faces in the diagram (NFACES = 2^NCOLORS faces).
    ///
    /// Indexed by face ID (0..NFACES), where face ID is the bitmask
    /// of colors bounding that face.
    ///
    /// **Heap-allocated** via Vec to handle variable NFACES (8 for N=3, 64 for N=6).
    pub faces: Vec<Face>,

    /// Expected cycle length for faces with k colors.
    ///
    /// `face_degree_by_color_count[k]` = C(NCOLORS, k) = number of ways to
    /// choose k items from NCOLORS items.
    ///
    /// This is used to validate that face degree signatures are feasible.
    ///
    /// For NCOLORS=6:
    /// - [0] = 1
    /// - [1] = 6
    /// - [2] = 15
    /// - [3] = 20
    /// - [4] = 15
    /// - [5] = 6
    /// - [6] = 1
    ///
    /// **Stack-allocated** array - small and fixed size (NCOLORS+1 ≤ 7 elements).
    pub face_degree_by_color_count: [u64; NCOLORS + 1],

    /// Cycles containing edge (color i → color j).
    ///
    /// `pairs[i][j]` is a CycleSet of all cycles that contain the directed edge
    /// from color i to color j.
    ///
    /// Used during constraint propagation to quickly find cycles with specific edges.
    ///
    /// **Stack-allocated** - fixed size (NCOLORS × NCOLORS × CYCLESET_LENGTH).
    pub pairs: [[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS],

    /// Cycles containing triple sequence (i → j → k).
    ///
    /// `triples[i][j][k]` is a CycleSet of all cycles containing colors i, j, k
    /// in that consecutive order (wrapping around).
    ///
    /// Used for vertex constraint checking - which cycles can meet at a vertex.
    ///
    /// **Stack-allocated** - fixed size (NCOLORS × NCOLORS × NCOLORS × CYCLESET_LENGTH).
    pub triples: [[[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS]; NCOLORS],

    /// Cycles NOT containing a specific color.
    ///
    /// `omitting_one[i]` is a CycleSet of all cycles that do not contain color i.
    ///
    /// Used to constrain faces that don't include certain curves.
    ///
    /// **Stack-allocated** - fixed size (NCOLORS × CYCLESET_LENGTH).
    pub omitting_one: [[u64; CYCLESET_LENGTH]; NCOLORS],

    /// Cycles NOT containing edge (i → j).
    ///
    /// `omitting_pair[i][j]` is a CycleSet of all cycles that do not contain
    /// the directed edge from color i to color j.
    ///
    /// Used for negative constraints during search.
    ///
    /// **Stack-allocated** - fixed size (NCOLORS × NCOLORS × CYCLESET_LENGTH).
    pub omitting_pair: [[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS],
}

impl FacesMemo {
    /// Initialize all face MEMO data.
    ///
    /// This computes:
    /// 1. Binomial coefficients for face degree validation
    /// 2. Cycle lookup tables (pairs, triples, omitting)
    /// 3. All NFACES faces with their color sets
    /// 4. Face adjacency relationships
    /// 5. Monotonicity constraints (which cycles are valid for which faces)
    /// 6. Next/previous face lookups by cycle ID
    ///
    /// # Arguments
    ///
    /// * `cycles` - The global array of all possible cycles
    pub fn initialize(cycles: &crate::memo::CyclesArray) -> Self {
        eprintln!("[FacesMemo] Computing binomial coefficients...");
        let face_degree_by_color_count = compute_binomial_coefficients();

        eprintln!("[FacesMemo] Computing cycle lookup tables...");
        let pairs = compute_cycle_pairs(cycles);
        let triples = compute_cycle_triples(cycles);
        let omitting_one = compute_omitting_one_color(cycles);
        let omitting_pair = compute_omitting_color_pairs(cycles);

        eprintln!("[FacesMemo] Creating {} faces...", NFACES);
        let mut faces = Vec::with_capacity(NFACES);
        for face_id in 0..NFACES {
            faces.push(create_face(face_id));
        }

        eprintln!("[FacesMemo] Applying monotonicity constraints...");
        apply_monotonicity_constraints(&mut faces, cycles);

        eprintln!("[FacesMemo] Initialization complete.");

        Self {
            faces,
            face_degree_by_color_count,
            pairs,
            triples,
            omitting_one,
            omitting_pair,
        }
    }

    /// Get a face by its ID.
    #[inline]
    pub fn get_face(&self, face_id: FaceId) -> &Face {
        &self.faces[face_id]
    }
}

/// Compute binomial coefficients C(NCOLORS, k) for k=0..NCOLORS.
///
/// Uses the recurrence relation:
/// C(n, k) = C(n, k-1) * (n - k + 1) / k
///
/// Starting with C(n, 0) = 1.
///
/// # Algorithm
///
/// Uses the recurrence relation C(n, k) = C(n, k-1) * (n - k + 1) / k:
fn compute_binomial_coefficients() -> [u64; NCOLORS + 1] {
    let mut coefficients = [0u64; NCOLORS + 1];
    coefficients[0] = 1;

    for i in 0..NCOLORS {
        coefficients[i + 1] = coefficients[i] * (NCOLORS - i) as u64 / (i + 1) as u64;
    }

    coefficients
}

/// Create a face with the given ID.
///
/// The face ID is interpreted as a bitmask of colors:
/// - Bit i set → color i bounds this face
/// - Face 0 = outer face (no colors, unbounded)
/// - Face NFACES-1 = inner face (all colors)
///
/// # Arguments
///
/// * `face_id` - The face identifier (0..NFACES)
///
/// # Returns
///
/// A Face with:
/// - ID set to face_id
/// - Colors set from bitmask
/// - Possible cycles initialized to all cycles with matching colors
/// - Adjacency tables empty (filled by monotonicity constraints)
fn create_face(face_id: FaceId) -> Face {
    // Convert face ID bitmask to ColorSet
    let mut colors = ColorSet::empty();
    for i in 0..NCOLORS {
        if (face_id & (1 << i)) != 0 {
            colors.insert(Color::new(i as u8));
        }
    }

    // Start with all possible cycles for this color count
    // (Will be filtered by monotonicity constraints)
    let possible_cycles = CycleSet::full();

    Face::new(face_id, colors, possible_cycles)
}

/// Compute cycle pairs lookup table.
///
/// For each color pair (i, j), find all cycles containing edge i→j.
fn compute_cycle_pairs(
    cycles: &crate::memo::CyclesArray,
) -> [[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS] {
    let mut pairs = [[[0u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS];

    for cycle_id in 0..cycles.len() as u64 {
        let cycle = cycles.get(cycle_id);

        // Check each consecutive pair in the cycle
        for i in 0..cycle.len() {
            let next_i = (i + 1) % cycle.len();
            let color_a = cycle.colors()[i];
            let color_b = cycle.colors()[next_i];

            // Add this cycle to the pairs[a][b] set
            let word_idx = (cycle_id / 64) as usize;
            let bit_idx = cycle_id % 64;
            pairs[color_a.value() as usize][color_b.value() as usize][word_idx] |= 1u64 << bit_idx;
        }
    }

    pairs
}

/// Compute cycle triples lookup table.
///
/// For each color triple (i, j, k), find all cycles containing sequence i→j→k.
fn compute_cycle_triples(
    cycles: &crate::memo::CyclesArray,
) -> [[[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS]; NCOLORS] {
    let mut triples = [[[[0u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS]; NCOLORS];

    for cycle_id in 0..cycles.len() as u64 {
        let cycle = cycles.get(cycle_id);

        // Check each consecutive triple in the cycle (including wrap-around)
        for i in 0..cycle.len() {
            let i1 = (i + 1) % cycle.len();
            let i2 = (i + 2) % cycle.len();
            let color_a = cycle.colors()[i];
            let color_b = cycle.colors()[i1];
            let color_c = cycle.colors()[i2];

            // Add this cycle to the triples[a][b][c] set
            let word_idx = (cycle_id / 64) as usize;
            let bit_idx = cycle_id % 64;
            triples[color_a.value() as usize][color_b.value() as usize]
                [color_c.value() as usize][word_idx] |= 1u64 << bit_idx;
        }
    }

    triples
}

/// Compute cycles omitting one color.
///
/// For each color i, find all cycles that do NOT contain color i.
fn compute_omitting_one_color(
    cycles: &crate::memo::CyclesArray,
) -> [[u64; CYCLESET_LENGTH]; NCOLORS] {
    let mut omitting = [[0u64; CYCLESET_LENGTH]; NCOLORS];

    for cycle_id in 0..cycles.len() as u64 {
        let cycle = cycles.get(cycle_id);
        let colorset = cycle.colorset();

        // For each color, if cycle doesn't contain it, add to omitting[color]
        for color in 0..NCOLORS {
            if !colorset.contains(Color::new(color as u8)) {
                let word_idx = (cycle_id / 64) as usize;
                let bit_idx = cycle_id % 64;
                omitting[color][word_idx] |= 1u64 << bit_idx;
            }
        }
    }

    omitting
}

/// Compute cycles omitting color pairs.
///
/// For each color pair (i, j), find all cycles that do NOT contain edge i→j.
fn compute_omitting_color_pairs(
    cycles: &crate::memo::CyclesArray,
) -> [[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS] {
    let mut omitting = [[[0u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS];

    for cycle_id in 0..cycles.len() as u64 {
        let cycle = cycles.get(cycle_id);

        // For each color pair (i, j), check if cycle contains edge i→j
        for i in 0..NCOLORS {
            for j in (i + 1)..NCOLORS {
                // Check if cycle contains edge i→j
                let mut has_edge = false;
                for idx in 0..cycle.len() {
                    let next_idx = (idx + 1) % cycle.len();
                    if cycle.colors()[idx] == Color::new(i as u8)
                        && cycle.colors()[next_idx] == Color::new(j as u8)
                    {
                        has_edge = true;
                        break;
                    }
                }

                if !has_edge {
                    let word_idx = (cycle_id / 64) as usize;
                    let bit_idx = cycle_id % 64;
                    omitting[i][j][word_idx] |= 1u64 << bit_idx;
                }
            }
        }
    }

    omitting
}

/// Apply monotonicity constraints to filter invalid cycles.
///
/// For each face, for each cycle:
/// 1. Check if cycle is valid for this face (has right colors, correct transitions)
/// 2. If valid, compute next/previous faces for this cycle
/// 3. If invalid, remove from possible_cycles
///
/// # Monotonicity
///
/// A monotone Venn diagram has the property that each facial cycle
/// crosses each curve at most once. This means:
/// - A cycle for face {a,b,c} must have colors from {a,b,c}
/// - The cycle must have exactly 2 edge transitions (in/out of face)
/// - The next and previous faces are determined by which edges transition
///
/// # TODO
///
/// This is a complex function that needs:
/// - Edge transition detection logic
/// - Next/previous face computation
///
/// For now, this is a skeleton that will be filled in later.
fn apply_monotonicity_constraints(_faces: &mut [Face], _cycles: &crate::memo::CyclesArray) {
    // TODO: Implement monotonicity constraint filtering
    // This requires:
    // 1. Check cycle colors match face colors
    // 2. Validate cycle has exactly two edge transitions (monotone property)
    // 3. Compute next/previous faces for each valid cycle
    // 4. Remove invalid cycles from possible_cycles

    eprintln!("[FacesMemo] WARNING: Monotonicity constraints not yet implemented (TODO)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binomial_coefficients() {
        let coeffs = compute_binomial_coefficients();

        // C(n, 0) = 1
        assert_eq!(coeffs[0], 1);

        // C(n, n) = 1
        assert_eq!(coeffs[NCOLORS], 1);

        // For NCOLORS=6:
        // C(6,1)=6, C(6,2)=15, C(6,3)=20, C(6,4)=15, C(6,5)=6
        #[cfg(any(
            feature = "ncolors_6",
            not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
        ))]
        {
            assert_eq!(coeffs[1], 6);
            assert_eq!(coeffs[2], 15);
            assert_eq!(coeffs[3], 20);
            assert_eq!(coeffs[4], 15);
            assert_eq!(coeffs[5], 6);
        }

        // For NCOLORS=3:
        // C(3,1)=3, C(3,2)=3
        #[cfg(feature = "ncolors_3")]
        {
            assert_eq!(coeffs[1], 3);
            assert_eq!(coeffs[2], 3);
        }
    }

    #[test]
    fn test_create_face_color_mapping() {
        // Face 0 = outer face (no colors)
        let face0 = create_face(0);
        assert_eq!(face0.id, 0);
        assert_eq!(face0.colors.len(), 0);

        // Face 1 = {color 0}
        let face1 = create_face(1);
        assert_eq!(face1.id, 1);
        assert_eq!(face1.colors.len(), 1);
        assert!(face1.colors.contains(Color::new(0)));

        // Face 3 = {color 0, color 1}
        let face3 = create_face(3);
        assert_eq!(face3.id, 3);
        assert_eq!(face3.colors.len(), 2);
        assert!(face3.colors.contains(Color::new(0)));
        assert!(face3.colors.contains(Color::new(1)));

        // Face NFACES-1 = inner face (all colors)
        let face_inner = create_face(NFACES - 1);
        assert_eq!(face_inner.id, NFACES - 1);
        assert_eq!(face_inner.colors.len(), NCOLORS);
    }

    #[test]
    fn test_faces_memo_initialization() {
        let cycles = crate::memo::CyclesArray::generate();
        let memo = FacesMemo::initialize(&cycles);

        // Should create exactly NFACES faces
        assert_eq!(memo.faces.len(), NFACES);

        // Outer face should exist
        let outer = memo.get_face(0);
        assert_eq!(outer.colors.len(), 0);

        // Inner face should exist
        let inner = memo.get_face(NFACES - 1);
        assert_eq!(inner.colors.len(), NCOLORS);

        // Binomial coefficients should be computed
        assert_eq!(memo.face_degree_by_color_count[0], 1);
        assert_eq!(memo.face_degree_by_color_count[NCOLORS], 1);
    }
}
