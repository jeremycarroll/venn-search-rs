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
    let checkpoint = ctx.trail.checkpoint();
    ctx.set_face_degree(0, 100);

    assert_eq!(ctx.get_face_degree(0), 100);
    assert_eq!(ctx.trail.len(), 1);

    // Rewind restores old value automatically!
    ctx.trail.rewind_to(checkpoint);
    assert_eq!(ctx.get_face_degree(0), 0);
    assert_eq!(ctx.trail.len(), 0);
}

#[test]
fn test_search_context_backtracking() {
    let mut ctx = SearchContext::new();

    // Create checkpoint
    let cp1 = ctx.trail.checkpoint();
    assert_eq!(cp1, 0);

    // Make some changes using safe wrappers
    ctx.set_face_degree(0, 999);
    ctx.set_face_degree(1, 888);
    ctx.set_face_degree(2, 777);

    assert_eq!(ctx.get_face_degree(0), 999);
    assert_eq!(ctx.get_face_degree(1), 888);
    assert_eq!(ctx.get_face_degree(2), 777);
    assert_eq!(ctx.trail.len(), 3);

    // Nested checkpoint
    let cp2 = ctx.trail.checkpoint();
    assert_eq!(cp2, 3);

    ctx.set_face_degree(0, 111);
    assert_eq!(ctx.trail.len(), 4);

    // Rewind to cp2
    ctx.trail.rewind_to(cp2);
    assert_eq!(ctx.get_face_degree(0), 999); // Restored!
    assert_eq!(ctx.trail.len(), 3);

    // Rewind to cp1
    ctx.trail.rewind_to(cp1);
    assert_eq!(ctx.get_face_degree(0), 0); // Restored to initial!
    assert_eq!(ctx.get_face_degree(1), 0);
    assert_eq!(ctx.trail.len(), 0);
}

#[test]
fn test_independent_search_contexts() {
    // This test verifies that multiple SearchContext instances can operate independently
    // which is critical for parallelization

    let mut ctx1 = SearchContext::new();
    let mut ctx2 = SearchContext::new();

    // Checkpoint in ctx1
    let cp1 = ctx1.trail.checkpoint();

    // Modify ctx1
    ctx1.set_face_degree(0, 100);
    assert_eq!(ctx1.get_face_degree(0), 100);
    assert_eq!(ctx1.trail.len(), 1);

    // ctx2 should be completely unaffected
    assert_eq!(ctx2.get_face_degree(0), 0);
    assert_eq!(ctx2.trail.len(), 0);

    // Checkpoint in ctx2
    let _cp2 = ctx2.trail.checkpoint();
    ctx2.set_face_degree(0, 200);

    // Both contexts have their own independent state
    assert_eq!(ctx1.get_face_degree(0), 100);
    assert_eq!(ctx2.get_face_degree(0), 200);
    assert_eq!(ctx1.trail.len(), 1);
    assert_eq!(ctx2.trail.len(), 1);

    // Rewind ctx1 doesn't affect ctx2
    ctx1.trail.rewind_to(cp1);
    assert_eq!(ctx1.get_face_degree(0), 0);
    assert_eq!(ctx1.trail.len(), 0);
    assert_eq!(ctx2.get_face_degree(0), 200); // Still has its value
    assert_eq!(ctx2.trail.len(), 1); // Still has its trail entry
}

#[test]
fn test_trail_freeze() {
    let mut ctx = SearchContext::new();

    // Make some changes
    let cp1 = ctx.trail.checkpoint();
    ctx.set_face_degree(0, 20);

    // Freeze the trail
    ctx.trail.freeze();

    // Make more changes after freeze
    let cp2 = ctx.trail.checkpoint();
    ctx.set_face_degree(0, 30);

    // Can rewind to cp2 (recent changes)
    ctx.trail.rewind_to(cp2);
    assert_eq!(ctx.get_face_degree(0), 20);
    assert_eq!(ctx.trail.len(), 1);

    // Cannot rewind past freeze point
    ctx.trail.rewind_to(cp1);
    assert_eq!(ctx.get_face_degree(0), 20); // Still 20 (blocked by freeze)
    assert_eq!(ctx.trail.len(), 1);
}

#[test]
fn test_array_operations() {
    use venn_search::geometry::constants::NCOLORS;

    let mut ctx = SearchContext::new();

    let checkpoint = ctx.trail.checkpoint();

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
    assert_eq!(ctx.trail.len(), expected_trail_len);

    // Rewind restores all array elements
    ctx.trail.rewind_to(checkpoint);
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
        let cp = ctx.trail.checkpoint();
        checkpoints.push(cp);
        ctx.set_face_degree(i, (i + 1) as u64 * 10);
    }

    assert_eq!(ctx.get_face_degree(NCOLORS - 1), NCOLORS as u64 * 10);
    assert_eq!(ctx.trail.len(), NCOLORS);

    // Rewind all the way back
    for i in (0..NCOLORS).rev() {
        ctx.trail.rewind_to(checkpoints[i]);
        if i > 0 {
            assert_eq!(ctx.get_face_degree(i - 1), i as u64 * 10);
        } else {
            assert_eq!(ctx.get_face_degree(0), 0);
        }
    }

    assert_eq!(ctx.trail.len(), 0);
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
fn test_raw_pointer_safety() {
    // This test demonstrates that our design prevents dangling pointers
    let mut ctx = SearchContext::new();

    let checkpoint = ctx.trail.checkpoint();
    ctx.set_face_degree(0, 42);

    // Even though we're using raw pointers internally,
    // the ownership model prevents dangling pointers:
    // - ctx owns both trail and state
    // - trail can only be rewound while ctx exists
    // - pointers in trail are guaranteed valid

    ctx.trail.rewind_to(checkpoint);
    assert_eq!(ctx.get_face_degree(0), 0);

    // If ctx is dropped here, both trail and state drop together
    // No possibility of dangling pointers!
}
