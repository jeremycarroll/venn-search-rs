## Summary

Implements Phase 1 of the Rust migration: **Memory Architecture & Trail System** - the foundation for the Venn triangle search engine.

This PR establishes the core backtracking system that all search predicates depend on, using a raw pointer-based trail design that matches the C implementation's efficiency while leveraging Rust's ownership system for memory safety.

## Key Features

### Two-Tier Memory Model

**Tier 1 (MEMO)**: Immutable precomputed data
- Shared across parallel searches (via copy or reference)
- Will contain facial cycle lookup tables, vertex configurations
- Currently placeholder, to be filled in Phase 2-3

**Tier 2 (DYNAMIC)**: Mutable search state
- Trail system for O(1) backtracking
- Per-search state (faces, edge counts, etc.)
- Each `SearchContext` owns its own trail and state

### Raw Pointer Trail System

- **128-bit trail entries**: 64-bit `*mut u64` pointer + 64-bit old value
- **Zero overhead**: Matches C implementation exactly, no abstraction cost
- **Automatic restoration**: `rewind()` walks trail backwards, restores all values
- **Memory safety**: Rust ownership ensures pointers remain valid (trail and state owned together)
- **u64-only**: No support for smaller types, optimized for 64-bit hardware

### Safe Wrapper API

`SearchContext` provides safe methods that wrap unsafe trail operations:
```rust
let mut ctx = SearchContext::new();
ctx.trail.checkpoint();
ctx.set_example_value(42);  // Safe wrapper, unsafe inside
ctx.trail.rewind();          // Automatic restoration!
```

Each wrapper method:
- Creates pointer into `ctx.state`
- Calls unsafe trail operation
- Explicit unsafe block (auditable, maintainable)
- ~10-15 total methods expected for full implementation

## Architecture

```
SearchContext {
    memo: MemoizedData,    // Tier 1: Immutable, shared
    trail: Trail,          // Tier 2: Backtracking system
    state: DynamicState,   // Tier 2: Mutable search state
}
```

**Parallelization strategy** (future):
1. Single-threaded initialization computes MEMO data
2. Single-threaded InnerFacePredicate finds ~10-20 degree signatures
3. Each signature spawns independent parallel search with its own `SearchContext`
4. Expected speedup: 5-10x on modern multi-core systems

## Module Structure

```
src/
├── lib.rs           - Library root with crate docs
├── memo/            - Tier 1: MEMO data (placeholder)
├── context/         - SearchContext combining both tiers
├── trail/           - Tier 2: Trail system (complete)
├── state/           - Tier 2: DYNAMIC state (placeholder)
└── geometry/        - Geometric types (Color skeleton)
```

## Testing

**29 tests passing** (20 unit + 9 integration):
- Trail operations: checkpoint, rewind, freeze, overflow
- SearchContext: creation, independence, nested operations
- Array operations: tracking multiple values
- Deep nesting: 10 levels of checkpoints

## Documentation

- Comprehensive doc comments with examples
- Generated docs via `cargo doc --open`
- Architecture explained in `docs/PHASE1_COMPLETE.md`
- Design decisions and performance notes in `CLAUDE.md`

## Performance

Matches C implementation characteristics:
- `checkpoint()`: O(1) - just push index
- `set value`: O(1) - record in Vec
- `rewind()`: O(n) entries to restore
- Memory: 16,384 max entries × 16 bytes = 256 KB

Expected overhead: <5% vs C (Vec vs array, minimal)

## Design Decisions

### Why Raw Pointers?

1. **Zero overhead**: Same memory layout as C (128 bits per entry)
2. **Natural array support**: Can point into Vec elements, bitmaps
3. **Automatic restoration**: Write old value directly via pointer
4. **Type safety**: Rust ownership prevents dangling pointers

### Why Not Trailed<T> Wrapper?

Initial implementation used `Trailed<T>` with ID-based tracking. Refactored to raw pointers because:
- ❌ Extra indirection (ID → HashMap → value)
- ❌ Couldn't handle arrays naturally
- ❌ Abstraction overhead (register, lookup)
- ✅ Raw pointers match C exactly
- ✅ Simpler design (-75 lines of code)

### Safety Invariant

**Invariant**: All pointers in trail must point into `SearchContext.state`

**Enforcement**:
- `SearchContext` owns both trail and state (same lifetime)
- Wrapper methods only create pointers into `self.state`
- Trail operations are `pub(crate)` (not public API)
- Each unsafe block is small, explicit, auditable

## Migration Status

**✅ Complete:**
- Trail system with checkpoint/rewind
- SearchContext combining MEMO + DYNAMIC
- Safe wrapper API pattern
- Comprehensive test suite

**⬜ Next (Phase 2):**
- Color and ColorSet types
- Cycle type (edge sequences)
- Basic geometric constants

## Testing Commands

```bash
cargo test              # Run all tests (29 passing)
cargo clippy            # Lint (clean)
cargo doc --open        # View documentation
cargo build --release   # Production build
```

## References

- C implementation: `c-reference/` directory (tag v1.1-pco)
- Carroll 2000 paper: "Drawing Venn triangles" (algorithm description)
- Full Phase 1 summary: `docs/PHASE1_COMPLETE.md`
