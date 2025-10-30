// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Mutable search state (Tier 2: DYNAMIC).

use crate::geometry::constants::NCOLORS;
use crate::geometry::CrossingCounts;
use crate::state::DynamicFaces;
use std::fs::File;
use std::io::BufWriter;

use super::MemoizedData;

/// Mutable search state (Tier 2: DYNAMIC).
///
/// This data changes during search and is tracked on the trail for backtracking.
/// Each SearchContext owns its own mutable state.
///
/// # Memory Allocation
///
/// Like MEMO data, DYNAMIC state uses mixed stack/heap allocation:
/// - **Stack**: Small fixed-size arrays (e.g., `current_face_degrees: [u64; 6]`)
/// - **Heap**: Variable-size collections (e.g., Vecs for edge lists, cycle sets)
///
/// The trail records raw pointers to these locations for O(1) backtracking.
#[derive(Debug)]
pub struct DynamicState {
    /// Current face degree assignments (for InnerFacePredicate).
    ///
    /// During the InnerFacePredicate phase, this array stores the degree
    /// of each of the NCOLORS symmetric faces bordering the central face.
    ///
    /// Note: Stored as u64 to work with the trail system, even though values are small.
    pub current_face_degrees: [u64; NCOLORS],

    /// Per-face mutable state (Phase 7.1).
    ///
    /// Contains current_cycle, possible_cycles, and cycle_count for each face.
    pub faces: DynamicFaces,

    /// Crossing counts between color pairs (for corner detection).
    ///
    /// Tracks how many times each pair of colors crosses in the current solution.
    /// Used to enforce the triangle constraint (max 6 crossings per pair).
    ///
    /// All modifications must be trail-tracked.
    pub crossing_counts: CrossingCounts,

    /// Tracks which vertices have been processed for crossing counts.
    ///
    /// Array of u64 flags (0 = not processed, 1 = processed).
    /// Size 512 to accommodate up to 480 possible vertices (see VerticesMemo).
    ///
    /// When a vertex is first encountered during facial cycle assignment,
    /// we increment the crossing count for that vertex's color pair and
    /// mark the vertex as processed to avoid double-counting.
    ///
    /// All modifications must be trail-tracked.
    pub vertex_processed: Vec<u64>,

    /// Tracks the number of edges assigned for each color and direction.
    ///
    /// Index [0][color] = clockwise edges (face contains the color)
    /// Index [1][color] = counterclockwise edges (face doesn't contain the color)
    ///
    /// Used to detect disconnected curves during corner checking.
    /// When we traverse a curve, we only follow one direction (all clockwise or all counterclockwise).
    ///
    /// All modifications must be trail-tracked.
    pub edge_color_counts: [[u64; NCOLORS]; 2],

    /// Tracks which colors have been checked for disconnection.
    ///
    /// Once a color's curve forms a complete closed loop and passes the
    /// disconnection check, we mark it here to avoid checking again.
    ///
    /// All modifications must be trail-tracked.
    pub colors_checked: [u64; NCOLORS],

    /// Temporary accumulator for colors that completed during current propagation.
    ///
    /// Reset before each top-level propagate_cycle_choice, then checked after.
    /// NOT trail-tracked (temporary per-call state).
    pub colors_completed_this_call: u64,

    /// Current default output within backtracking context - can only have one.
    /// Replace with a vector, and a trailed index to provide better functionality.
    pub output: Option<Box<BufWriter<File>>>,
}

impl DynamicState {
    /// Create initial dynamic state from MEMO data.
    pub fn new(memo: &MemoizedData) -> Self {
        Self {
            current_face_degrees: [0; NCOLORS],
            faces: DynamicFaces::new(&memo.faces),
            crossing_counts: CrossingCounts::new(),
            vertex_processed: vec![0u64; 512], // 512 slots for up to 480 vertices
            edge_color_counts: [[0; NCOLORS]; 2],
            colors_checked: [0; NCOLORS],
            colors_completed_this_call: 0,
            output: None,
        }
    }
}
