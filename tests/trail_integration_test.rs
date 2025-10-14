// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Integration tests for the trail-based backtracking system.
//!
//! These tests verify that the trail system works correctly when integrated
//! with SearchContext, demonstrating the core backtracking behavior that
//! the search engine depends on.

use venn_search::{SearchContext, Trail};

#[test]
fn test_trail_simple_backtracking() {
    let mut trail = Trail::new();
    let mut registry = venn_search::trail::TrailedRegistry::new();

    let mut x = registry.register(10u64);
    let mut y = registry.register(20u64);

    // Initial state
    assert_eq!(x.get(), 10);
    assert_eq!(y.get(), 20);

    // Checkpoint and modify
    trail.checkpoint();
    x.set(&mut trail, 100);
    y.set(&mut trail, 200);

    assert_eq!(x.get(), 100);
    assert_eq!(y.get(), 200);
    assert_eq!(trail.len(), 2);

    // Rewind restores old values
    trail.rewind();
    // Note: Trail doesn't automatically restore values - that's done by the engine
    // For now we're just testing the trail recording mechanism
    assert_eq!(trail.len(), 0);
}

#[test]
fn test_search_context_backtracking() {
    let mut ctx = SearchContext::new();

    // Register some tracked values
    let mut value1 = ctx.registry.register(42u64);
    let mut value2 = ctx.registry.register(100u64);
    let mut flag = ctx.registry.register(false);

    // Create checkpoint
    let cp1 = ctx.trail.checkpoint();
    assert_eq!(cp1, 0);

    // Make some changes
    value1.set(&mut ctx.trail, 999);
    value2.set(&mut ctx.trail, 888);
    flag.set(&mut ctx.trail, true);

    assert_eq!(value1.get(), 999);
    assert_eq!(value2.get(), 888);
    assert_eq!(flag.get(), true);
    assert_eq!(ctx.trail.len(), 3);

    // Nested checkpoint
    let cp2 = ctx.trail.checkpoint();
    assert_eq!(cp2, 3);

    value1.set(&mut ctx.trail, 777);
    assert_eq!(ctx.trail.len(), 4);

    // Rewind inner checkpoint
    assert!(ctx.trail.rewind());
    assert_eq!(ctx.trail.len(), 3);

    // Rewind outer checkpoint
    assert!(ctx.trail.rewind());
    assert_eq!(ctx.trail.len(), 0);
}

#[test]
fn test_independent_search_contexts() {
    // This test verifies that multiple SearchContext instances can operate independently
    // which is critical for parallelization

    let mut ctx1 = SearchContext::new();
    let mut ctx2 = SearchContext::new();

    // Register values in each context
    let mut v1 = ctx1.registry.register(10u64);
    let mut v2 = ctx2.registry.register(20u64);

    // Checkpoint in ctx1
    ctx1.trail.checkpoint();

    // Modify ctx1
    v1.set(&mut ctx1.trail, 100);
    assert_eq!(v1.get(), 100);
    assert_eq!(ctx1.trail.len(), 1);

    // ctx2 should be completely unaffected
    assert_eq!(v2.get(), 20);
    assert_eq!(ctx2.trail.len(), 0);
    assert_eq!(ctx2.trail.checkpoint_depth(), 0);

    // Checkpoint in ctx2
    ctx2.trail.checkpoint();
    v2.set(&mut ctx2.trail, 200);

    // Both contexts have their own independent state
    assert_eq!(v1.get(), 100);
    assert_eq!(v2.get(), 200);
    assert_eq!(ctx1.trail.len(), 1);
    assert_eq!(ctx2.trail.len(), 1);

    // Rewind ctx1 doesn't affect ctx2
    ctx1.trail.rewind();
    assert_eq!(ctx1.trail.len(), 0);
    assert_eq!(ctx2.trail.len(), 1); // Still has its trail entry
}

#[test]
fn test_trail_freeze() {
    let mut ctx = SearchContext::new();
    let mut value = ctx.registry.register(10u64);

    // Make some changes
    ctx.trail.checkpoint();
    value.set(&mut ctx.trail, 20);

    // Freeze the trail
    ctx.trail.freeze();

    // Make more changes after freeze
    ctx.trail.checkpoint();
    value.set(&mut ctx.trail, 30);

    // Can rewind recent changes
    assert!(ctx.trail.rewind());
    assert_eq!(ctx.trail.len(), 1);

    // But cannot rewind past freeze point
    assert!(!ctx.trail.rewind());
    assert_eq!(ctx.trail.len(), 1);
}

#[test]
fn test_trail_maybe_set() {
    let mut ctx = SearchContext::new();
    let mut value = ctx.registry.register(42u64);

    ctx.trail.checkpoint();

    // Setting same value doesn't record in trail
    assert!(!value.maybe_set(&mut ctx.trail, 42));
    assert_eq!(ctx.trail.len(), 0);

    // Setting different value records in trail
    assert!(value.maybe_set(&mut ctx.trail, 100));
    assert_eq!(ctx.trail.len(), 1);
    assert_eq!(value.get(), 100);

    // Setting same value again doesn't record
    assert!(!value.maybe_set(&mut ctx.trail, 100));
    assert_eq!(ctx.trail.len(), 1);
}

#[test]
fn test_multiple_types() {
    let mut ctx = SearchContext::new();

    let mut u8_val = ctx.registry.register(10u8);
    let mut u16_val = ctx.registry.register(1000u16);
    let mut u32_val = ctx.registry.register(100000u32);
    let mut u64_val = ctx.registry.register(10000000u64);
    let mut bool_val = ctx.registry.register(false);
    let mut usize_val = ctx.registry.register(42usize);

    ctx.trail.checkpoint();

    u8_val.set(&mut ctx.trail, 20);
    u16_val.set(&mut ctx.trail, 2000);
    u32_val.set(&mut ctx.trail, 200000);
    u64_val.set(&mut ctx.trail, 20000000);
    bool_val.set(&mut ctx.trail, true);
    usize_val.set(&mut ctx.trail, 84);

    assert_eq!(u8_val.get(), 20);
    assert_eq!(u16_val.get(), 2000);
    assert_eq!(u32_val.get(), 200000);
    assert_eq!(u64_val.get(), 20000000);
    assert_eq!(bool_val.get(), true);
    assert_eq!(usize_val.get(), 84);

    assert_eq!(ctx.trail.len(), 6);
}

#[test]
fn test_deep_nesting() {
    let mut ctx = SearchContext::new();
    let mut counter = ctx.registry.register(0u64);

    // Create 10 nested checkpoints
    for i in 0..10 {
        ctx.trail.checkpoint();
        counter.set(&mut ctx.trail, i);
    }

    assert_eq!(counter.get(), 9);
    assert_eq!(ctx.trail.len(), 10);
    assert_eq!(ctx.trail.checkpoint_depth(), 10);

    // Rewind all the way back
    for _ in 0..10 {
        assert!(ctx.trail.rewind());
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
