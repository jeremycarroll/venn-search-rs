// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Cycle generation and global cycles array.
//!
//! This module generates all possible facial cycles for the current NCOLORS
//! and provides the global Cycles array used throughout the search.
//!
//! # Cycle Generation Algorithm
//!
//! Cycles are generated in a specific order to ensure deterministic behavior:
//! 1. Grouped by maximum color (from 2 to NCOLORS-1)
//! 2. Within each max color, ordered by length (3 to max+1)
//! 3. Within each length, ordered lexicographically
//!
//! This ordering ensures that cycles using fewer colors come first, which
//! helps with incremental search strategies.
//!
//! # Cycle Validity Rules
//!
//! A valid cycle must:
//! - Have length ≥ 3 (faces must be bounded by at least 3 curves)
//! - Start with the smallest color (canonical form)
//! - Contain no duplicate colors
//! - Contain the maximum color for its group
//!
//! # Example
//!
//! For NCOLORS=3, we generate 2 cycles:
//! - (abc) - length 3, max color 2
//! - (acb) - length 3, max color 2
//!
//! For NCOLORS=6, we generate 394 cycles total.

use crate::geometry::constants::{CYCLESET_LENGTH, NCOLORS, NCYCLES};
use crate::geometry::{Color, Cycle};

/// MEMO data for cycle-related lookup tables.
///
/// This structure contains precomputed constraint lookup tables for efficient
/// cycle-based constraint propagation during search.
///
/// # Memory Layout
///
/// All lookup tables are **stack-allocated** arrays (~16 KB total for NCOLORS=6):
/// - `cycle_pairs`: 6×6 × 7 u64s ≈ 2 KB
/// - `cycle_triples`: 6×6×6 × 7 u64s ≈ 12 KB
/// - `cycles_omitting_one_color`: 6 × 7 u64s ≈ 336 bytes
/// - `cycles_omitting_color_pair`: 6×6 × 7 u64s ≈ 2 KB
#[derive(Debug, Clone)]
pub struct CyclesMemo {
    /// Cycles containing edge (color i → color j).
    ///
    /// `cycle_pairs[i][j]` is a CycleSet of all cycles that contain the directed edge
    /// from color i to color j.
    ///
    /// Used during constraint propagation to quickly find cycles with specific edges.
    pub cycle_pairs: [[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS],

    /// Cycles containing triple sequence (i → j → k).
    ///
    /// `cycle_triples[i][j][k]` is a CycleSet of all cycles containing colors i, j, k
    /// in that consecutive order (wrapping around).
    ///
    /// Used for vertex constraint checking - which cycles can meet at a vertex.
    pub cycle_triples: [[[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS]; NCOLORS],

    /// Cycles NOT containing a specific color.
    ///
    /// `cycles_omitting_one_color[i]` is a CycleSet of all cycles that do not contain color i.
    ///
    /// Used to constrain faces that don't include certain curves.
    pub cycles_omitting_one_color: [[u64; CYCLESET_LENGTH]; NCOLORS],

    /// Cycles NOT containing directed edge (i → j) for unordered color pairs.
    ///
    /// **IMPORTANT**: Only upper triangle (i < j) is populated and accessed.
    ///
    /// `cycles_omitting_color_pair[i][j]` where i < j contains all cycles that do NOT
    /// contain the directed edge from color i to color j.
    ///
    /// **Design note**: Although this checks DIRECTED edges (i→j distinct from j→i),
    /// only the upper triangle is used. The search algorithm (venn.c:98-109) iterates
    /// `for i < j` and only accesses `[i][j]` entries. The lower triangle ([j][i] for j > i)
    /// is never populated or accessed, and attempting to access it is a bug.
    ///
    /// **Private**: Access via `get_cycles_omitting_color_pair(i, j)` which enforces i < j.
    ///
    /// Used for negative constraints during search to restrict non-vertex-adjacent faces.
    cycles_omitting_color_pair: [[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS],
}

/// Global array of all possible facial cycles.
///
/// This array contains all NCYCLES cycles, indexed by CycleId (0..NCYCLES-1).
/// Cycles are generated once during initialization and never change.
///
/// # Memory
///
/// - Size: NCYCLES × sizeof(Cycle) ≈ 12 KB for NCOLORS=6
/// - Stored on heap via Vec to avoid stack overflow
/// - Immutable after initialization (part of MEMO tier)
#[derive(Debug, Clone)]
pub struct CyclesArray {
    cycles: Vec<Cycle>,
}

impl CyclesArray {
    /// Generate all possible facial cycles.
    ///
    /// # Algorithm
    ///
    /// For each maximum color k (from 2 to NCOLORS-1):
    ///   For each length (from 3 to k+1):
    ///     Generate all sequences of that length
    ///     Filter to keep only valid cycles
    ///     Add valid cycles to array
    ///
    /// Valid cycles must:
    /// - Start with smallest color (canonical)
    /// - Contain no duplicates
    /// - Contain the maximum color k
    pub fn generate() -> Self {
        eprintln!("[CyclesArray] Generating {} cycles...", NCYCLES);

        let mut cycles = Vec::with_capacity(NCYCLES);

        // Generate cycles for each maximum color
        for max_color in 2..NCOLORS {
            for length in 3..=(max_color + 1) {
                generate_cycles_with_max_and_length(max_color, length, &mut cycles);
            }
        }

        assert_eq!(
            cycles.len(),
            NCYCLES,
            "Expected {} cycles, generated {}",
            NCYCLES,
            cycles.len()
        );

        eprintln!("[CyclesArray] Generated {} cycles.", cycles.len());

        Self { cycles }
    }

    /// Get a cycle by its ID.
    #[inline]
    pub fn get(&self, cycle_id: u64) -> &Cycle {
        &self.cycles[cycle_id as usize]
    }

    /// Get the number of cycles.
    #[inline]
    pub fn len(&self) -> usize {
        self.cycles.len()
    }

    /// Check if the array is empty (should never be true after initialization).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cycles.is_empty()
    }
}

/// Generate all valid cycles with a specific maximum color and length.
///
/// # Algorithm
///
/// 1. Start with sequence [max, max, max, ...] of given length
/// 2. Generate all sequences in reverse lexicographic order by decrementing
/// 3. Filter to keep only valid cycles
/// 4. Add valid cycles to the array
fn generate_cycles_with_max_and_length(max_color: usize, length: usize, cycles: &mut Vec<Cycle>) {
    let mut current = vec![max_color as u8; length];

    loop {
        // Check if this sequence is a valid cycle
        if is_cycle_valid(length, max_color as u8, &current) {
            // Convert to Color array and create Cycle
            let colors: Vec<Color> = current.iter().map(|&c| Color::new(c)).collect();
            cycles.push(Cycle::new(&colors));
        }

        // Find the rightmost position that can be decremented
        let mut pos = length - 1;
        let mut found = false;
        loop {
            if current[pos] > 0 {
                current[pos] -= 1;
                found = true;
                break;
            }
            if pos == 0 {
                break;
            }
            pos -= 1;
        }

        // If we couldn't decrement any position, we're done
        if !found {
            break;
        }

        // Reset all positions to the right to max_color
        for item in current[(pos + 1)..length].iter_mut() {
            *item = max_color as u8;
        }
    }
}

/// Check if a color sequence is a valid cycle.
///
/// A valid cycle must:
/// - Contain the maximum color
/// - Have no duplicate colors
/// - Start with the smallest color (canonical form)
fn is_cycle_valid(length: usize, max_color: u8, sequence: &[u8]) -> bool {
    let mut has_max = false;
    let mut used = [false; NCOLORS];

    for (i, &color) in sequence.iter().take(length).enumerate() {
        // Check if max color is present
        if color == max_color {
            has_max = true;
        }

        // Check for duplicates
        if used[color as usize] {
            return false;
        }
        used[color as usize] = true;

        // First element must be the smallest (canonical form)
        if i > 0 && color < sequence[0] {
            return false;
        }
    }

    has_max
}

impl CyclesMemo {
    /// Initialize all cycle MEMO data.
    ///
    /// This computes cycle constraint lookup tables from the generated cycles array
    /// and populates direction tables for each cycle.
    pub fn initialize(cycles: &mut CyclesArray) -> Self {
        eprintln!("[CyclesMemo] Computing cycle lookup tables...");

        let cycle_pairs = compute_cycle_pairs(cycles);
        let cycle_triples = compute_cycle_triples(cycles);
        let cycles_omitting_one_color = compute_cycles_omitting_one_color(cycles);
        let cycles_omitting_color_pair = compute_cycles_omitting_color_pair(cycles);

        eprintln!("[CyclesMemo] Initializing direction tables for all cycles...");

        // Initialize direction tables for each cycle using the computed lookup tables
        for cycle_id in 0..cycles.len() {
            cycles.cycles[cycle_id].init_direction_tables(&cycle_pairs, &cycle_triples);
        }

        eprintln!("[CyclesMemo] Cycle lookup tables complete.");

        Self {
            cycle_pairs,
            cycle_triples,
            cycles_omitting_one_color,
            cycles_omitting_color_pair,
        }
    }

    /// Get cycles omitting a color pair (upper triangle only).
    ///
    /// Returns the CycleSet of all cycles that do NOT contain the directed edge i→j.
    ///
    /// # Arguments
    ///
    /// * `i` - First color (must be less than `j`)
    /// * `j` - Second color (must be greater than `i`)
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `i >= j`. This catches bugs where the lower triangle
    /// is accessed (which is never populated). In release builds, the check is optimized away.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cycles_memo = CyclesMemo::initialize(&cycles);
    /// let omitting = cycles_memo.get_cycles_omitting_color_pair(2, 4); // OK: 2 < 4
    /// // let omitting = cycles_memo.get_cycles_omitting_color_pair(4, 2); // PANIC in debug!
    /// ```
    #[inline]
    pub fn get_cycles_omitting_color_pair(&self, i: usize, j: usize) -> &[u64; CYCLESET_LENGTH] {
        debug_assert!(
            i < j,
            "cycles_omitting_color_pair only valid for i < j (upper triangle), got i={}, j={}",
            i,
            j
        );
        &self.cycles_omitting_color_pair[i][j]
    }

    /// Get cycles omitting a single color.
    ///
    /// Returns the CycleSet of all cycles that do NOT contain the specified color.
    ///
    /// # Arguments
    ///
    /// * `color_idx` - The color index (0..NCOLORS-1)
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `color_idx >= NCOLORS`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cycles_memo = CyclesMemo::initialize(&cycles);
    /// let omitting = cycles_memo.get_cycles_omitting_one_color(3); // Cycles without color 3
    /// ```
    #[inline]
    pub fn get_cycles_omitting_one_color(&self, color_idx: usize) -> [u64; CYCLESET_LENGTH] {
        debug_assert!(
            color_idx < NCOLORS,
            "color_idx out of range: {} >= {}",
            color_idx,
            NCOLORS
        );
        self.cycles_omitting_one_color[color_idx]
    }
}

/// Compute cycle pairs lookup table.
///
/// For each color pair (i, j), find all cycles containing edge i→j.
fn compute_cycle_pairs(cycles: &CyclesArray) -> [[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS] {
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
    cycles: &CyclesArray,
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
fn compute_cycles_omitting_one_color(cycles: &CyclesArray) -> [[u64; CYCLESET_LENGTH]; NCOLORS] {
    let mut omitting = [[0u64; CYCLESET_LENGTH]; NCOLORS];

    for cycle_id in 0..cycles.len() as u64 {
        let cycle = cycles.get(cycle_id);
        let colorset = cycle.colorset();

        // For each color, if cycle doesn't contain it, add to omitting[color]
        for (color, omitting_set) in omitting.iter_mut().enumerate() {
            if !colorset.contains(Color::new(color as u8)) {
                let word_idx = (cycle_id / 64) as usize;
                let bit_idx = cycle_id % 64;
                omitting_set[word_idx] |= 1u64 << bit_idx;
            }
        }
    }

    omitting
}

/// Compute cycles omitting color pairs (upper triangle only).
///
/// For each unordered color pair (i, j) where i < j, finds all cycles that do NOT
/// contain the DIRECTED edge from color i to color j.
///
/// **IMPORTANT**: Only populates upper triangle (i < j). The lower triangle is left
/// as zeros and should never be accessed - that would be a bug in the search logic.
///
/// Matches C implementation in cycleset.c:initializeOmittingColorPairs() which also
/// only populates the upper triangle.
fn compute_cycles_omitting_color_pair(
    cycles: &CyclesArray,
) -> [[[u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS] {
    let mut omitting = [[[0u64; CYCLESET_LENGTH]; NCOLORS]; NCOLORS];

    for cycle_id in 0..cycles.len() as u64 {
        let cycle = cycles.get(cycle_id);

        // Only iterate upper triangle (i < j), matching C code behavior
        for (i, omitting_i) in omitting.iter_mut().enumerate() {
            for (j, omitting_i_j) in omitting_i.iter_mut().enumerate().skip(i + 1) {
                // Check if cycle contains the DIRECTED edge i→j
                // (Note: i→j is different from j→i, but we only check i→j where i < j)
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
                    omitting_i_j[word_idx] |= 1u64 << bit_idx;
                }
            }
        }
    }

    // Lower triangle (j < i) is left as zeros - never populated, never accessed
    omitting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_generation_count() {
        let cycles = CyclesArray::generate();

        // Check we generated the right number
        assert_eq!(cycles.len(), NCYCLES);

        // Verify specific counts for each NCOLORS
        match NCOLORS {
            3 => assert_eq!(cycles.len(), 2),
            4 => assert_eq!(cycles.len(), 14),
            5 => assert_eq!(cycles.len(), 74),
            6 => assert_eq!(cycles.len(), 394),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_cycles_non_empty() {
        let cycles = CyclesArray::generate();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_all_cycles_have_valid_length() {
        let cycles = CyclesArray::generate();

        for cycle_id in 0..cycles.len() as u64 {
            let cycle = cycles.get(cycle_id);
            assert!(cycle.len() >= 3, "Cycle {} has length < 3", cycle_id);
            assert!(
                cycle.len() <= NCOLORS,
                "Cycle {} has length > NCOLORS",
                cycle_id
            );
        }
    }

    #[test]
    fn test_cycles_start_with_smallest_color() {
        let cycles = CyclesArray::generate();

        for cycle_id in 0..cycles.len() as u64 {
            let cycle = cycles.get(cycle_id);
            let colors = cycle.colors();

            // First color should be smallest
            for (i, &color) in colors.iter().enumerate() {
                if i > 0 {
                    assert!(
                        color >= colors[0],
                        "Cycle {} not canonical: color at {} is smaller than first",
                        cycle_id,
                        i
                    );
                }
            }
        }
    }

    #[test]
    fn test_cycles_have_no_duplicates() {
        let cycles = CyclesArray::generate();

        for cycle_id in 0..cycles.len() as u64 {
            let cycle = cycles.get(cycle_id);
            let colors = cycle.colors();

            // Check for duplicates
            for i in 0..colors.len() {
                for j in (i + 1)..colors.len() {
                    assert_ne!(
                        colors[i], colors[j],
                        "Cycle {} has duplicate color",
                        cycle_id
                    );
                }
            }
        }
    }

    #[test]
    fn test_cycles_omitting_color_pair_accessor() {
        let mut cycles = CyclesArray::generate();
        let memo = CyclesMemo::initialize(&mut cycles);

        // Should work for upper triangle (i < j)
        for i in 0..NCOLORS {
            for j in (i + 1)..NCOLORS {
                let _omitting = memo.get_cycles_omitting_color_pair(i, j);
                // Just verify it doesn't panic
            }
        }
    }

    #[test]
    #[should_panic(expected = "cycles_omitting_color_pair only valid for i < j")]
    #[cfg(debug_assertions)]
    fn test_cycles_omitting_color_pair_lower_triangle_panics() {
        let mut cycles = CyclesArray::generate();
        let memo = CyclesMemo::initialize(&mut cycles);

        // Should panic in debug mode for lower triangle (i > j)
        let _omitting = memo.get_cycles_omitting_color_pair(3, 1);
    }

    #[test]
    #[should_panic(expected = "cycles_omitting_color_pair only valid for i < j")]
    #[cfg(debug_assertions)]
    fn test_cycles_omitting_color_pair_diagonal_panics() {
        let mut cycles = CyclesArray::generate();
        let memo = CyclesMemo::initialize(&mut cycles);

        // Should panic in debug mode for diagonal (i == j)
        let _omitting = memo.get_cycles_omitting_color_pair(2, 2);
    }
}
