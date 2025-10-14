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
    assert_eq!(ctx.state.example_value, 0);

    // Checkpoint and modify using safe wrapper
    ctx.trail.checkpoint();
    ctx.set_example_value(100);

    assert_eq!(ctx.state.example_value, 100);
    assert_eq!(ctx.trail.len(), 1);

    // Rewind restores old value automatically!
    ctx.trail.rewind();
    assert_eq!(ctx.state.example_value, 0);
    assert_eq!(ctx.trail.len(), 0);
}

#[test]
fn test_search_context_backtracking() {
    let mut ctx = SearchContext::new();

    // Create checkpoint
    let cp1 = ctx.trail.checkpoint();
    assert_eq!(cp1, 0);

    // Make some changes using safe wrappers
    ctx.set_example_value(999);
    ctx.set_array_element(0, 888);
    ctx.set_array_element(1, 777);

    assert_eq!(ctx.state.example_value, 999);
    assert_eq!(ctx.state.example_array[0], 888);
    assert_eq!(ctx.state.example_array[1], 777);
    assert_eq!(ctx.trail.len(), 3);

    // Nested checkpoint
    let cp2 = ctx.trail.checkpoint();
    assert_eq!(cp2, 3);

    ctx.set_example_value(111);
    assert_eq!(ctx.trail.len(), 4);

    // Rewind inner checkpoint
    assert!(ctx.trail.rewind());
    assert_eq!(ctx.state.example_value, 999); // Restored!
    assert_eq!(ctx.trail.len(), 3);

    // Rewind outer checkpoint
    assert!(ctx.trail.rewind());
    assert_eq!(ctx.state.example_value, 0); // Restored to initial!
    assert_eq!(ctx.state.example_array[0], 0);
    assert_eq!(ctx.trail.len(), 0);
}

#[test]
fn test_independent_search_contexts() {
    // This test verifies that multiple SearchContext instances can operate independently
    // which is critical for parallelization

    let mut ctx1 = SearchContext::new();
    let mut ctx2 = SearchContext::new();

    // Checkpoint in ctx1
    ctx1.trail.checkpoint();

    // Modify ctx1
    ctx1.set_example_value(100);
    assert_eq!(ctx1.state.example_value, 100);
    assert_eq!(ctx1.trail.len(), 1);

    // ctx2 should be completely unaffected
    assert_eq!(ctx2.state.example_value, 0);
    assert_eq!(ctx2.trail.len(), 0);
    assert_eq!(ctx2.trail.checkpoint_depth(), 0);

    // Checkpoint in ctx2
    ctx2.trail.checkpoint();
    ctx2.set_example_value(200);

    // Both contexts have their own independent state
    assert_eq!(ctx1.state.example_value, 100);
    assert_eq!(ctx2.state.example_value, 200);
    assert_eq!(ctx1.trail.len(), 1);
    assert_eq!(ctx2.trail.len(), 1);

    // Rewind ctx1 doesn't affect ctx2
    ctx1.trail.rewind();
    assert_eq!(ctx1.state.example_value, 0);
    assert_eq!(ctx1.trail.len(), 0);
    assert_eq!(ctx2.state.example_value, 200); // Still has its value
    assert_eq!(ctx2.trail.len(), 1); // Still has its trail entry
}

#[test]
fn test_trail_freeze() {
    let mut ctx = SearchContext::new();

    // Make some changes
    ctx.trail.checkpoint();
    ctx.set_example_value(20);

    // Freeze the trail
    ctx.trail.freeze();

    // Make more changes after freeze
    ctx.trail.checkpoint();
    ctx.set_example_value(30);

    // Can rewind recent changes
    assert!(ctx.trail.rewind());
    assert_eq!(ctx.state.example_value, 20);
    assert_eq!(ctx.trail.len(), 1);

    // But cannot rewind past freeze point
    assert!(!ctx.trail.rewind());
    assert_eq!(ctx.state.example_value, 20); // Still 20
    assert_eq!(ctx.trail.len(), 1);
}

#[test]
fn test_trail_maybe_set() {
    let mut ctx = SearchContext::new();
    ctx.trail.checkpoint();

    // Setting same value doesn't record in trail
    assert!(!ctx.maybe_set_example_value(0));
    assert_eq!(ctx.trail.len(), 0);

    // Setting different value records in trail
    assert!(ctx.maybe_set_example_value(100));
    assert_eq!(ctx.trail.len(), 1);
    assert_eq!(ctx.state.example_value, 100);

    // Setting same value again doesn't record
    assert!(!ctx.maybe_set_example_value(100));
    assert_eq!(ctx.trail.len(), 1);

    // Rewind restores
    ctx.trail.rewind();
    assert_eq!(ctx.state.example_value, 0);
}

#[test]
fn test_array_operations() {
    let mut ctx = SearchContext::new();

    ctx.trail.checkpoint();
    ctx.set_array_element(0, 10);
    ctx.set_array_element(5, 50);
    ctx.set_array_element(9, 90);

    assert_eq!(ctx.state.example_array[0], 10);
    assert_eq!(ctx.state.example_array[5], 50);
    assert_eq!(ctx.state.example_array[9], 90);
    assert_eq!(ctx.trail.len(), 3);

    // Rewind restores all array elements
    ctx.trail.rewind();
    assert_eq!(ctx.state.example_array[0], 0);
    assert_eq!(ctx.state.example_array[5], 0);
    assert_eq!(ctx.state.example_array[9], 0);
}

#[test]
fn test_deep_nesting() {
    let mut ctx = SearchContext::new();

    // Create 10 nested checkpoints
    for i in 0..10 {
        ctx.trail.checkpoint();
        ctx.set_example_value(i as u64);
    }

    assert_eq!(ctx.state.example_value, 9);
    assert_eq!(ctx.trail.len(), 10);
    assert_eq!(ctx.trail.checkpoint_depth(), 10);

    // Rewind all the way back
    for i in (0..10).rev() {
        assert!(ctx.trail.rewind());
        if i > 0 {
            assert_eq!(ctx.state.example_value, (i - 1) as u64);
        } else {
            assert_eq!(ctx.state.example_value, 0);
        }
    }

    assert_eq!(ctx.trail.len(), 0);
    assert_eq!(ctx.trail.checkpoint_depth(), 0);
}

#[test]
fn test_memo_size_is_small() {
    // Verify that MemoizedData is small enough to copy efficiently
    let size = SearchContext::memo_size_bytes();

    // For now it's just a placeholder (0 bytes)
    println!("MemoizedData size: {} bytes", size);

    // Once we add real MEMO data, we'll want to verify it stays reasonable
    // Target: < 1MB for efficient copying
    // If > 1MB, we should use &'static references instead
}

#[test]
fn test_raw_pointer_safety() {
    // This test demonstrates that our design prevents dangling pointers
    let mut ctx = SearchContext::new();

    ctx.trail.checkpoint();
    ctx.set_example_value(42);

    // Even though we're using raw pointers internally,
    // the ownership model prevents dangling pointers:
    // - ctx owns both trail and state
    // - trail can only be rewound while ctx exists
    // - pointers in trail are guaranteed valid

    ctx.trail.rewind();
    assert_eq!(ctx.state.example_value, 0);

    // If ctx is dropped here, both trail and state drop together
    // No possibility of dangling pointers!
}
