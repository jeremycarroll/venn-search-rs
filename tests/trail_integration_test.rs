// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Integration tests for the trail-based backtracking system.
//!
//! These tests verify that the trail system works correctly when integrated
//! with SearchContext, demonstrating the core backtracking behavior that
//! the search engine depends on.

use venn_search::SearchContext;

#[test]
fn test_search_context_simple_backtracking() {
    // This test demonstrates the basic trail workflow using SearchContext
    let mut ctx = SearchContext::new();

    // Initial state
    assert_eq!(ctx.get_face_degree(0), 0);

    // Checkpoint and modify using safe wrapper
    let checkpoint = ctx.checkpoint();
    ctx.set_face_degree(0, 100);

    assert_eq!(ctx.get_face_degree(0), 100);
    assert_eq!(ctx.trail().len(), 1);

    // Rewind restores old value automatically!
    ctx.rewind_to(checkpoint);
    assert_eq!(ctx.get_face_degree(0), 0);
    assert_eq!(ctx.trail().len(), 0);
}

#[test]
fn test_search_context_backtracking() {
    let mut ctx = SearchContext::new();

    // Create checkpoint
    let cp1 = ctx.checkpoint();
    assert_eq!(cp1, 0);

    // Make some changes using safe wrappers
    ctx.set_face_degree(0, 999);
    ctx.set_face_degree(1, 888);
    ctx.set_face_degree(2, 777);

    assert_eq!(ctx.get_face_degree(0), 999);
    assert_eq!(ctx.get_face_degree(1), 888);
    assert_eq!(ctx.get_face_degree(2), 777);
    assert_eq!(ctx.trail().len(), 3);

    // Nested checkpoint
    let cp2 = ctx.checkpoint();
    assert_eq!(cp2, 3);

    ctx.set_face_degree(0, 111);
    assert_eq!(ctx.trail().len(), 4);

    // Rewind to cp2
    ctx.rewind_to(cp2);
    assert_eq!(ctx.get_face_degree(0), 999); // Restored!
    assert_eq!(ctx.trail().len(), 3);

    // Rewind to cp1
    ctx.rewind_to(cp1);
    assert_eq!(ctx.get_face_degree(0), 0); // Restored to initial!
    assert_eq!(ctx.get_face_degree(1), 0);
    assert_eq!(ctx.trail().len(), 0);
}

#[test]
fn test_independent_search_contexts() {
    // This test verifies that multiple SearchContext instances can operate independently
    // which is critical for parallelization

    let mut ctx1 = SearchContext::new();
    let mut ctx2 = SearchContext::new();

    // Checkpoint in ctx1
    let cp1 = ctx1.checkpoint();

    // Modify ctx1
    ctx1.set_face_degree(0, 100);
    assert_eq!(ctx1.get_face_degree(0), 100);
    assert_eq!(ctx1.trail().len(), 1);

    // ctx2 should be completely unaffected
    assert_eq!(ctx2.get_face_degree(0), 0);
    assert_eq!(ctx2.trail().len(), 0);

    // Checkpoint in ctx2
    let _cp2 = ctx2.checkpoint();
    ctx2.set_face_degree(0, 200);

    // Both contexts have their own independent state
    assert_eq!(ctx1.get_face_degree(0), 100);
    assert_eq!(ctx2.get_face_degree(0), 200);
    assert_eq!(ctx1.trail().len(), 1);
    assert_eq!(ctx2.trail().len(), 1);

    // Rewind ctx1 doesn't affect ctx2
    ctx1.rewind_to(cp1);
    assert_eq!(ctx1.get_face_degree(0), 0);
    assert_eq!(ctx1.trail().len(), 0);
    assert_eq!(ctx2.get_face_degree(0), 200); // Still has its value
    assert_eq!(ctx2.trail().len(), 1); // Still has its trail entry
}

#[test]
fn test_trail_freeze() {
    let mut ctx = SearchContext::new();

    // Make some changes
    let cp1 = ctx.checkpoint();
    ctx.set_face_degree(0, 20);

    // Freeze the trail
    ctx.freeze();

    // Make more changes after freeze
    let cp2 = ctx.checkpoint();
    ctx.set_face_degree(0, 30);

    // Can rewind to cp2 (recent changes)
    ctx.rewind_to(cp2);
    assert_eq!(ctx.get_face_degree(0), 20);
    assert_eq!(ctx.trail().len(), 1);

    // Cannot rewind past freeze point
    ctx.rewind_to(cp1);
    assert_eq!(ctx.get_face_degree(0), 20); // Still 20 (blocked by freeze)
    assert_eq!(ctx.trail().len(), 1);
}

#[test]
fn test_array_operations() {
    use venn_search::geometry::constants::NCOLORS;

    let mut ctx = SearchContext::new();

    let checkpoint = ctx.checkpoint();

    // Set face degree 0
    ctx.set_face_degree(0, 10);

    // Set middle face degree (if NCOLORS >= 4)
    let middle_idx = NCOLORS / 2;
    if NCOLORS >= 4 {
        ctx.set_face_degree(middle_idx, 50);
    }

    // Set last face degree
    ctx.set_face_degree(NCOLORS - 1, 90);

    // Verify values
    assert_eq!(ctx.get_face_degree(0), 10);
    if NCOLORS >= 4 {
        assert_eq!(ctx.get_face_degree(middle_idx), 50);
    }
    assert_eq!(ctx.get_face_degree(NCOLORS - 1), 90);

    let expected_trail_len = if NCOLORS >= 4 { 3 } else { 2 };
    assert_eq!(ctx.trail().len(), expected_trail_len);

    // Rewind restores all array elements
    ctx.rewind_to(checkpoint);
    assert_eq!(ctx.get_face_degree(0), 0);
    if NCOLORS >= 4 {
        assert_eq!(ctx.get_face_degree(middle_idx), 0);
    }
    assert_eq!(ctx.get_face_degree(NCOLORS - 1), 0);
}

#[test]
fn test_deep_nesting() {
    use venn_search::geometry::constants::NCOLORS;

    let mut ctx = SearchContext::new();

    // Create NCOLORS nested checkpoints (one for each face degree)
    let mut checkpoints = Vec::new();
    for i in 0..NCOLORS {
        let cp = ctx.checkpoint();
        checkpoints.push(cp);
        ctx.set_face_degree(i, (i + 1) as u64 * 10);
    }

    assert_eq!(ctx.get_face_degree(NCOLORS - 1), NCOLORS as u64 * 10);
    assert_eq!(ctx.trail().len(), NCOLORS);

    // Rewind all the way back
    for i in (0..NCOLORS).rev() {
        ctx.rewind_to(checkpoints[i]);
        if i > 0 {
            assert_eq!(ctx.get_face_degree(i - 1), i as u64 * 10);
        } else {
            assert_eq!(ctx.get_face_degree(0), 0);
        }
    }

    assert_eq!(ctx.trail().len(), 0);
}

#[test]
fn test_memo_size_is_small() {
    // Verify that MemoizedData is small enough to copy efficiently
    let size = SearchContext::memo_size_bytes();

    println!("MemoizedData size: {} bytes", size);

    // MEMO data should be under 1MB for efficient copying
    // Current size is ~230 KB (confirmed in Phase 6)
    let ctx = SearchContext::new();
    let heap_size = ctx.estimate_memo_heap_size();
    let total = size + heap_size;
    println!(
        "MemoizedData total size: {} bytes ({:.2} KB)",
        total,
        total as f64 / 1024.0
    );

    assert!(total < 1024 * 1024, "MEMO data should be under 1MB");
}

#[test]
fn moved_owner_restores_all_ten_target_kinds() {
    use venn_search::geometry::constants::{NCOLORS, NCYCLES, NFACES, NPOINTS};
    use venn_search::geometry::{CurveLink, CycleSet, EdgeRef};

    let mut ctx = SearchContext::new();
    let original = format!("{:?}", ctx.state());
    let checkpoint = ctx.checkpoint();
    ctx.set_face_degree(NCOLORS - 1, 42);
    ctx.set_face_cycle(NFACES - 1, (NCYCLES - 1) as u64);
    ctx.set_face_possible_cycles(NFACES - 1, CycleSet::empty());
    let (_, owner) = ctx.parts_mut();
    let link = CurveLink::new(EdgeRef::new(NFACES - 1, NCOLORS - 1), NPOINTS - 1);
    owner.set_edge_connection(NFACES - 1, NCOLORS - 1, Some(link));
    owner.set_next_face(NFACES - 1, Some(0));
    owner.set_previous_face(NFACES - 1, Some(NFACES - 1));
    owner.set_crossing_count(0, NCOLORS - 1, 6);
    owner.mark_vertex_processed(NPOINTS - 1);
    owner.increment_edge_color_count(1, NCOLORS - 1);
    owner.mark_color_checked(NCOLORS - 1);

    assert_eq!(owner.state().faces.faces[NFACES - 1].edge_dynamic[NCOLORS - 1].get_to(), Some(link));
    assert_eq!(owner.state().faces.faces[NFACES - 1].next_face(), Some(0));
    assert_eq!(owner.state().faces.faces[NFACES - 1].previous_face(), Some(NFACES - 1));
    assert_eq!(owner.state().crossing_counts.get(0, NCOLORS - 1), 6);
    assert_eq!(owner.state().vertex_processed[NPOINTS - 1], 1);
    assert_eq!(owner.state().edge_color_counts[1][NCOLORS - 1], 1);
    assert_eq!(owner.state().colors_checked[NCOLORS - 1], 1);
    let mut moved = Box::new(ctx);
    moved.rewind_to(checkpoint);
    assert_eq!(format!("{:?}", moved.state()), original);
    assert!(moved.trail().is_empty());
}

#[test]
fn cycle_words_count_and_retry_cursor_have_distinct_lifetimes() {
    use venn_search::geometry::constants::NCYCLES;
    use venn_search::geometry::CycleSet;
    let mut ctx = SearchContext::new();
    let entry = ctx.checkpoint();
    ctx.reset_face_cycle(0);
    let choice = ctx.checkpoint();
    let original = *ctx.get_face_possible_cycles(0);
    let mut selected = CycleSet::empty();
    selected.insert(0);
    selected.insert((NCYCLES - 1) as u64);
    if NCYCLES > 64 {
        selected.insert(63);
        selected.insert(64);
    }
    ctx.set_retry_cursor_untrailed(0, Some(0));
    ctx.set_face_possible_cycles(0, selected);
    assert_eq!(*ctx.get_face_possible_cycles(0), selected);
    assert_eq!(ctx.get_face_cycle_count(0), selected.len() as u64);
    let unchanged = ctx.checkpoint();
    ctx.set_face_possible_cycles(0, selected);
    assert_eq!(ctx.checkpoint(), unchanged);
    ctx.set_face_cycle(0, (NCYCLES - 1) as u64);
    ctx.rewind_to(choice);
    assert_eq!(ctx.state().faces.faces[0].current_cycle(), Some(0));
    assert_eq!(*ctx.get_face_possible_cycles(0), original);
    assert_eq!(ctx.get_face_cycle_count(0), original.len() as u64);
    ctx.rewind_to(entry);
    assert_eq!(ctx.state().faces.faces[0].current_cycle(), None);
}

#[test]
fn failed_propagation_restores_partial_writes() {
    use venn_search::geometry::constants::{NCOLORS, NCYCLES, NFACES};
    use venn_search::propagation::{propagate_cycle_choice, PropagationFailure};
    let mut ctx = SearchContext::new();
    let (_, owner) = ctx.parts_mut();
    for i in 0..NCOLORS {
        for j in i + 1..NCOLORS {
            owner.set_crossing_count(i, j, 6);
        }
    }
    let before = format!("{:?}", ctx.state());
    let checkpoint = ctx.checkpoint();
    let (memo, owner) = ctx.parts_mut();
    let result = propagate_cycle_choice(memo, owner, NFACES - 1, (NCYCLES - 1) as u64, 0);
    assert!(matches!(result, Err(PropagationFailure::CrossingLimitExceeded { count: 7, .. })));
    assert!(ctx.checkpoint() > checkpoint);
    ctx.rewind_to(checkpoint);
    assert_eq!(format!("{:?}", ctx.state()), before);
}

#[test]
fn reset_discards_state_log_and_freeze_together() {
    let mut ctx = SearchContext::new();
    let original = format!("{:?}", ctx.state());
    ctx.set_face_degree(0, 42);
    ctx.freeze();
    ctx.parts_mut().1.add_completed_color(0);
    ctx.reset_state();
    assert_eq!(format!("{:?}", ctx.state()), original);
    assert_eq!(ctx.checkpoint(), 0);
    ctx.set_face_degree(0, 7);
    ctx.rewind_to(0);
    assert_eq!(ctx.get_face_degree(0), 0);
}

#[test]
fn invalid_inputs_are_rejected_before_state_or_log_changes() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use venn_search::geometry::constants::{NCOLORS, NCYCLES, NFACES, NPOINTS};
    use venn_search::geometry::{CurveLink, EdgeRef};
    use venn_search::TrailedState;
    let mut ctx = SearchContext::new();
    let (_, owner) = ctx.parts_mut();
    let original = format!("{:?}", owner.state());
    let invalid: &[fn(&mut TrailedState)] = &[
        |s| s.set_face_degree(NCOLORS, 1),
        |s| s.reset_face_cycle(NFACES),
        |s| s.set_face_cycle(0, NCYCLES as u64),
        |s| s.set_face_cycle(0, u64::MAX),
        |s| s.set_retry_cursor_untrailed(0, Some(NCYCLES as u64)),
        |s| s.set_next_face(0, Some(NFACES)),
        |s| s.set_previous_face(NFACES, Some(0)),
        |s| s.set_crossing_count(1, 0, 1),
        |s| s.set_crossing_count(0, 0, 1),
        |s| s.set_crossing_count(0, NCOLORS, 1),
        |s| s.mark_vertex_processed(NPOINTS),
        |s| s.increment_edge_color_count(2, 0),
        |s| s.increment_edge_color_count(0, NCOLORS),
        |s| s.mark_color_checked(NCOLORS),
        |s| s.add_completed_color(NCOLORS),
        |s| s.set_edge_connection(0, NCOLORS, None),
        |s| s.set_edge_connection(0, 0, Some(CurveLink::new(EdgeRef::new(NFACES, 0), 0))),
        |s| s.set_edge_connection(0, 0, Some(CurveLink::new(EdgeRef::new(0, NCOLORS), 0))),
        |s| s.set_edge_connection(0, 0, Some(CurveLink::new(EdgeRef::new(0, 0), NPOINTS))),
        |s| s.rewind_to(1),
    ];
    for operation in invalid {
        assert!(catch_unwind(AssertUnwindSafe(|| operation(owner))).is_err());
        assert_eq!(owner.checkpoint(), 0);
        assert_eq!(format!("{:?}", owner.state()), original);
    }
}

#[test]
fn temporary_accumulator_is_untrailed_and_cleared_explicitly() {
    let mut ctx = SearchContext::new();
    let (_, owner) = ctx.parts_mut();
    owner.add_completed_color(0);
    owner.set_face_degree(0, 42);
    owner.rewind_to(0);
    assert_eq!(owner.state().colors_completed_this_call, 1);
    owner.clear_completed_colors();
    assert_eq!(owner.state().colors_completed_this_call, 0);
    assert!(owner.trail().is_empty());
}
