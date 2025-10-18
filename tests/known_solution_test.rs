// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Known solution test - validates constraint propagation using Carroll 2000 solution.
//!
//! This test pre-seeds facial cycle assignments from the published Carroll 2000
//! solution and verifies that constraint propagation works correctly.
//!
//! The test data is organized by face color count (3, 4, 5, 6 colors) and
//! tracks how many assignments are "forced" by propagation vs "guessed".

#[cfg(feature = "ncolors_6")]
use venn_search::context::SearchContext;
#[cfg(feature = "ncolors_6")]
use venn_search::geometry::{Color, ColorSet};
#[cfg(feature = "ncolors_6")]
use venn_search::propagation;
#[cfg(feature = "ncolors_6")]
use std::cell::RefCell;

/// Known solution data from Carroll 2000.
/// Format: (face_colors_str, cycle_colors_str)
#[cfg(feature = "ncolors_6")]
const KNOWN_SOLUTION_DATA: &[(&str, &str)] = &[
    // Faces with 3 colors
    ("ac", "aed"),
    ("abc", "cfe"),
    ("bd", "afb"),
    ("cd", "abd"),
    ("ae", "bde"),
    ("ace", "ade"),
    ("abce", "cef"),
    ("de", "bdf"),
    ("ade", "bed"),
    ("bde", "bfd"),
    ("cde", "adb"),
    ("af", "ace"),
    ("bf", "afc"),
    ("abf", "acf"),
    ("cf", "acd"),
    ("bdf", "abf"),
    ("cdf", "adc"),
    ("bcdf", "adf"),
    ("aef", "aec"),
    ("bef", "cfd"),
    ("abef", "cdf"),
    ("cef", "adc"),
    ("bcef", "cdf"),
    ("adef", "bce"),
    // Faces with 4 colors
    ("a", "abed"),
    ("b", "abcf"),
    ("bc", "bdfc"),
    ("d", "abfd"),
    ("ad", "adeb"),
    ("abd", "abef"),
    ("acd", "adeb"),
    ("bcd", "afdb"),
    ("abcd", "abef"),
    ("be", "bdfc"),
    ("bce", "bcfd"),
    ("abde", "bdfe"),
    ("acde", "abed"),
    ("bcde", "abdf"),
    ("abcde", "afeb"),
    ("acf", "adec"),
    ("bcf", "acfd"),
    ("adf", "abec"),
    ("abdf", "afeb"),
    ("acdf", "aced"),
    ("abcdf", "afed"),
    ("acef", "aced"),
    ("abcef", "cfed"),
    ("def", "bfdc"),
    ("bdef", "bcdf"),
    ("cdef", "abcd"),
    // Faces with 5 colors
    ("ab", "afceb"),
    ("c", "adbce"),
    ("e", "bcefd"),
    ("abe", "becfd"),
    ("ce", "aecbd"),
    ("f", "aefdc"),
    ("abcf", "adefc"),
    ("df", "acdfb"),
    ("ef", "acdfe"),
    ("abdef", "befdc"),
    ("acdef", "adecb"),
    ("bcdef", "afdcb"),
    // Faces with 6 colors
    ("", "adfecb"),
    ("abcdef", "abcdef"),
];

/// Convert a color string like "abc" to a ColorSet.
#[cfg(feature = "ncolors_6")]
fn parse_colors(s: &str) -> ColorSet {
    let mut colorset = ColorSet::empty();
    for ch in s.chars() {
        let color_idx = (ch as u8) - b'a';
        colorset.insert(Color::new(color_idx));
    }
    colorset
}

/// Convert a ColorSet to a face ID (bitmask).
#[cfg(feature = "ncolors_6")]
fn colorset_to_face_id(colorset: ColorSet) -> usize {
    colorset.bits() as usize
}

/// Find a cycle ID by matching its color sequence.
#[cfg(feature = "ncolors_6")]
fn find_cycle_id(ctx: &SearchContext, cycle_colors_str: &str) -> Option<u64> {
    use venn_search::geometry::constants::NCYCLES;

    let target_colors: Vec<Color> = cycle_colors_str
        .chars()
        .map(|ch| Color::new((ch as u8) - b'a'))
        .collect();

    // Search all cycles for matching color sequence
    for cycle_id in 0..NCYCLES as u64 {
        let cycle = ctx.memo.cycles.get(cycle_id);
        let cycle_colors = cycle.colors();

        if cycle_colors.len() != target_colors.len() {
            continue;
        }

        // Check if colors match (accounting for rotation)
        for start_offset in 0..cycle_colors.len() {
            let mut matches = true;
            for i in 0..cycle_colors.len() {
                let cycle_idx = (start_offset + i) % cycle_colors.len();
                if cycle_colors[cycle_idx] != target_colors[i] {
                    matches = false;
                    break;
                }
            }
            if matches {
                return Some(cycle_id);
            }
        }
    }

    None
}

#[test]
#[cfg(feature = "ncolors_6")]
fn test_known_solution_in_order() {
    eprintln!("\n=== Testing Known Solution (In Order) ===");
    eprintln!("Pre-seeding Carroll 2000 solution and tracking propagation");

    let mut ctx = SearchContext::new();
    let mut assignments_made = 0;
    let mut forced_by_propagation = 0;

    // Track which faces we've manually assigned
    let manually_assigned = RefCell::new(std::collections::HashSet::new());

    for (face_colors_str, cycle_colors_str) in KNOWN_SOLUTION_DATA {
        let face_colorset = parse_colors(face_colors_str);
        let face_id = colorset_to_face_id(face_colorset);

        // Check if face is already assigned (forced by propagation)
        let current_cycle = ctx.state.faces.faces[face_id].current_cycle();
        if current_cycle.is_some() {
            eprintln!("Face {} already assigned (forced by propagation)", face_id);
            forced_by_propagation += 1;
            continue;
        }

        // Find the cycle ID
        let cycle_id = find_cycle_id(&ctx, cycle_colors_str)
            .unwrap_or_else(|| panic!("Could not find cycle for colors '{}'", cycle_colors_str));

        // Check that this cycle is in the face's possible set
        let possible_cycles = &ctx.state.faces.faces[face_id].possible_cycles;
        assert!(
            possible_cycles.contains(cycle_id),
            "Face {} cannot have cycle {} (colors '{}')",
            face_id,
            cycle_id,
            cycle_colors_str
        );

        // Record this as a manual assignment
        manually_assigned.borrow_mut().insert(face_id);

        // Set the cycle directly (simulating VennPredicate choice)
        ctx.state.faces.faces[face_id].set_current_cycle(Some(cycle_id));

        // Propagate constraints
        let result = propagation::propagate_cycle_choice(
            &ctx.memo,
            &mut ctx.state,
            &mut ctx.trail,
            face_id,
            cycle_id,
            0,
        );

        if let Err(failure) = result {
            eprintln!("Propagation failed at face {}: {:?}", face_id, failure);
            eprintln!("Face colors: {}", face_colors_str);
            eprintln!("Cycle colors: {}", cycle_colors_str);
            panic!("Known solution should not fail propagation!");
        }

        assignments_made += 1;
        eprintln!(
            "Assigned face {} (colors '{}') cycle {} ('{}')",
            face_id, face_colors_str, cycle_id, cycle_colors_str
        );
    }

    eprintln!("\n=== Results ===");
    eprintln!("Total faces in known solution: {}", KNOWN_SOLUTION_DATA.len());
    eprintln!("Manual assignments made: {}", assignments_made);
    eprintln!("Forced by propagation: {}", forced_by_propagation);
    eprintln!("Propagation effectiveness: {:.1}%",
        100.0 * forced_by_propagation as f64 / KNOWN_SOLUTION_DATA.len() as f64);

    // Verify all 64 faces are assigned
    use venn_search::geometry::constants::NFACES;
    let mut unassigned_count = 0;
    for face_id in 0..NFACES {
        if ctx.state.faces.faces[face_id].current_cycle().is_none() {
            unassigned_count += 1;
            eprintln!("WARNING: Face {} is unassigned!", face_id);
        }
    }

    assert_eq!(unassigned_count, 0, "All faces should be assigned");
    eprintln!("\n✓ All 64 faces successfully assigned from known solution");
}
