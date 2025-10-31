// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Immutable precomputed data (Tier 1: MEMO).

use crate::memo::{CyclesArray, CyclesMemo, FacesMemo, VerticesMemo};

/// Immutable precomputed data (Tier 1: MEMO).
///
/// This data is computed once during initialization and never changes during search.
/// It can be shared across multiple SearchContext instances (via copy or reference).
///
/// # Size Estimation
///
/// Measured size (Phase 6, NCOLORS=6):
/// - Stack: ~16 KB (CyclesMemo lookup tables) + 88 bytes (Vec/Box headers + arrays)
/// - Heap: ~214 KB
///   - CyclesArray: ~12 KB (394 Cycle structs in Vec)
///   - FacesMemo: ~55 KB (5 KB Face structs + 50 KB next/previous arrays)
///   - VerticesMemo: ~147 KB (64×6×6 Option<Vertex> array in Box)
/// - **Total: ~230 KB**
///
/// Future additions may increase size:
/// - Edge relationship tables
/// - PCO/Chirotope structures
/// - Expected final size: ~250-300 KB
///
/// **Decision: Copy strategy confirmed** - At <1MB, copying per SearchContext
/// provides excellent cache locality while enabling parallelization.
#[derive(Debug, Clone)]
pub struct MemoizedData {
    /// All possible facial cycles (NCYCLES = 394 for NCOLORS=6)
    pub cycles: CyclesArray,

    /// Cycle-related MEMO data (lookup tables for constraint propagation)
    pub cycles_memo: CyclesMemo,

    /// All face-related MEMO data (binomial coefficients, adjacency, etc.)
    pub faces: FacesMemo,

    /// All vertex-related MEMO data (crossing point configurations)
    pub vertices: VerticesMemo,
    // TODO: Add more MEMO fields in later phases:
    // - Edge relationship tables
    // - PCO/Chirotope structures
}

impl MemoizedData {
    /// Initialize all MEMO data structures.
    ///
    /// Computes all immutable precomputed data needed for the search.
    /// This is called once at SearchContext creation.
    pub fn new() -> Self {
        eprintln!("[MemoizedData] Initializing all MEMO structures...");

        let mut cycles = CyclesArray::generate();
        let cycles_memo = CyclesMemo::initialize(&mut cycles);
        let mut faces = FacesMemo::initialize(&cycles);
        let vertices = VerticesMemo::initialize();

        // Phase 3: Link edges to vertices for corner detection
        faces.populate_vertex_links(&vertices);

        eprintln!(
            "[MemoizedData] Initialization complete ({} cycles, {} faces, {} possible vertices)",
            cycles.len(),
            faces.faces.len(),
            vertices.vertices.len()
        );

        Self {
            cycles,
            cycles_memo,
            faces,
            vertices,
        }
    }
}

impl Default for MemoizedData {
    fn default() -> Self {
        Self::new()
    }
}
