# Design Documentation

The goal of the program is to find all choices of facial cycle for each face such that
the overall result describes a planar graph that can be drawn with six triangles.

We cover both high level design and implementation details of wide scope.
Some other comments about lower level issues are in the code documentation.

## High-Level Design

The problem of finding diagrams of 6 Venn triangles is a search problem.
The search is divided into three parts:

1. Find a maximal sequence of 6 integers making a 5-face degree signature.
2. (the main search) Find 64 facial cycles defining a Venn diagram with this 5-face degree signature,
   which satisfies several necessary conditions to be drawable with triangles.
3. Find an edge to corner mapping for this Venn diagram,
   satisfying the condition that every pair of lines cross at most once.

The final step is to write the resulting Venn diagram, including its corners
into a [GraphML](http://graphml.graphdrawing.org/primer/graphml-primer.html) file.

We approach this in top-down fashion. Each of the three steps involves guessing, and we usually guess badly. That
branch of the search ends in failure and we backtrack to the previous
choice point, and make the next guess.

Success is very similar to failure. We get to the end of the search,
satisfying the criteria for this phase. We then proceed to the next phase of the search,
which is based on the results so far.
After executing the next phase, we backtrack, undoing the guesses we have made so far,
and proceed to the next guess.

### Searching for Venn Diagrams

In the main search, at each step we assign a specific facial cycle to a specific face.
At every step in the search we have a set of remaining possible facial cycles for each face.
If this set is empty for any face, then the search has failed and we backtrack to
the previous choice point.
If this set is a singleton for any face, then we make that choice
and compute all its consequences, which may result in failure, or
in a further assignment in a face with only one remaining facial cycle.

In the main loop, we first select the face with the fewest possible choices of facial cycle as the next face.
We choose
a facial cycle for that face. We will later backtrack and guess again
making all possible choices for the facial cycle for that chosen face.
With each choice, we compute has many consequences as we can, restricting the possible facial cycles for other faces.
The selection of which face to use for this iteration of the loop
is not backtrackable - the selected face does need to have a facial cycle: we have decided
to choose it now.

## Non-deterministic Engine, Backtracking, Memory and the Trail

Given that the problem is non-deterministic, with three separate non-deterministic subproblems,
we encode them all uniformly as top-down searches, and abandon the usual top-level control flow
to instead use a non-deterministic engine.
The engine executes a sequence of predicates. Each predicate can be evaluated to either succeed or fail
or create a choice point. Each choice point has a known number of choices. Each choice can either succeed
or fail, continuing to the next choice. When the choices are exhausted the predicate fails.

Success has two flavors: _success-same-predicate_ is a partial success that re-invokes the
current predicate with an incremented _round_ (starting at 0); _success-next-predicate_ is a full success,
indicating that the engine should move on to the next predicate. It is a runtime error if the final
predicate succeeds with _success-next-predicate_. A constant predicate the `FailPredicate` is provided to be the final entry in most programs.

On failure, if the current execution is a choice-point, the next choice (if any) is invoked. Otherwise,
the program backtracks to the previous predicate. If there are no previous predicates then the
program execution has completed, since the top-down search has been exhausted.

### Inspiration from Prolog

The engine design is inspired by Prolog's execution model, particularly the **Byrd box model**
for understanding control flow in logic programming:

- **Call port**: Entry to a predicate (our `try_pred`)
- **Exit port**: Success leaving predicate (our `Success` result)
- **Redo port**: Re-entry on backtracking (our `retry_pred`)
- **Fail port**: Failure leaving predicate (our `Failure` result)

This model provides a clean mental framework for understanding non-deterministic search:
each predicate is a "box" with four ports through which execution can flow. The trail system
handles state restoration when flowing backward through the Redo port.

**Deviation from pure Byrd box model**: Our `SuccessSame` result doesn't fit neatly into the
traditional 4-port model - it could be thought of as a fifth "re-call" port that loops back
to the Call port with an incremented round counter. In Prolog, similar functionality would be
achieved through recursive predicate calls, but our simplified implementation doesn't support
general recursion. Instead, `SuccessSame` provides a limited form of iteration within a single
predicate, useful for predicates that generate multiple solutions before moving to the next phase.

Like Prolog's choice points, our predicates maintain backtracking state. Unlike Prolog,
we use explicit trail-based backtracking rather than the WAM (Warren Abstract Machine) model,
giving us more control over what state is saved and restored.

**References**:
- Byrd, L. (1980). "Understanding the control flow of Prolog programs."
- See [notes on Byrd box model](https://github.com/dtonhofer/prolog_notes/blob/master/other_notes/about_byrd_box_model/README.md)

### Memory Management in Rust

The Rust implementation uses a two-tier memory model with clear ownership semantics:

#### Tier 1: MEMO Data (Immutable, Computed Once)

Precomputed lookup tables and constraints, computed during initialization:
- Facial cycle constraint tables
- Possible vertex configurations (480 entries for N=6)
- Edge and face relationship tables
- Cycle membership tables

**Strategy**: Owned by `SearchContext`, computed once in `InitializePredicate`, immutable thereafter.

#### Tier 2: DYNAMIC Data (Mutable, Per-Search)

State that changes during search, tracked on the trail for backtracking:
- `Trail` - records all state changes for O(1) restoration
- `DynamicFaces` - current facial cycle assignments
- `EdgeDynamic` - edge crossing counts
- Search statistics and counters

**Strategy**: Each `SearchContext` owns its mutable state, changes are tracked on the trail.

### Trail System

The trail system is critical for efficient backtracking:

**Purpose**: Record state changes to enable O(1) restoration during backtracking.

**Implementation**: `Vec<TrailEntry>` with checkpoint-based rewind.

**Key Features**:
- Type-safe: Different entry types for different data structures
- Checkpoint system: Save/restore trail index for nested scopes
- Sentinel values: Special u64 values encode Option<u64> for CycleSet
- Zero-copy restoration: Backtracking just replays trail entries in reverse

**Usage Pattern**:
```rust
// Save state before choice
let checkpoint = trail.save_checkpoint();

// Make changes (all tracked on trail)
faces.set_cycle(face_id, cycle_id, trail);

// On success: continue
// On failure: restore
trail.rewind_to_checkpoint(checkpoint);
```

See `src/trail/mod.rs` for implementation details.

## Search Engine

The engine (`src/engine/mod.rs`) implements the non-deterministic search framework.

### Predicate Trait

All search phases implement the `Predicate` trait:

```rust
pub trait Predicate {
    fn try_pred(&mut self, round: u64, ctx: &mut SearchContext) -> PredicateResult;
    fn retry_pred(&mut self, ctx: &mut SearchContext) -> PredicateResult;
}
```

- `try_pred`: Called on first entry and after each success-same-predicate
- `retry_pred`: Called when backtracking to this predicate
- `round`: Increments on each success-same-predicate (starts at 0)

**Default behavior**: For deterministic predicates (those without choice points), `retry_pred`
should return `Failure` immediately - there are no alternative choices to explore. The default
implementation panics to catch unimplemented predicates during development.

### Open/Close Predicate Pattern

For predicates that perform side effects (I/O, logging, statistics) without participating in
the search, we provide the `OpenClosePredicate` wrapper:

```rust
pub trait OpenClose {
    fn open(&mut self, ctx: &mut SearchContext) -> bool;
    fn close(&mut self, ctx: &mut SearchContext);
}
```

- `open`: Called when entering the predicate (forward execution). Returns `false` to fail immediately.
- `close`: Called when backtracking through the predicate (backward execution).

This pattern is useful for:
- Opening/closing output files around a search phase
- Logging entry/exit of search phases
- Recording statistics before/after operations
- Resource management that must happen in pairs

The `OpenClosePredicate` wrapper handles the `try_pred`/`retry_pred` mechanics, calling
`open` on forward pass and `close` on backtrack.

### PredicateResult Enum

```rust
pub enum PredicateResult {
    Success,           // Move to next predicate
    SuccessSame,       // Re-invoke try_pred with round+1
    Failure,           // Backtrack
}
```

### Engine Stack

The engine maintains a stack of predicate states:
- Current predicate index
- Current round number
- Choice point information

On backtrack from Failure, the engine pops until finding a predicate with remaining choices,
not just popping once.

### EngineBuilder

Predicates are composed using the builder pattern:

```rust
let engine = EngineBuilder::new()
    .add(Box::new(InitializePredicate))
    .add(Box::new(InnerFacePredicate))
    .add(Box::new(VennPredicate::new()))
    .terminal(Box::new(FailPredicate))
    .build();

engine.search(&mut ctx);
```

See `src/engine/mod.rs` for implementation.

## Three Phases & Four Predicates

The current implementation (Phase 7 complete) has these predicates:

### 1. InitializePredicate (`src/predicates/initialize.rs`)

**Purpose**: Deterministic initialization of MEMO data.

**Actions**:
- Initialize all geometric constants (NCOLORS, NFACES, etc.)
- Compute all possible facial cycles
- Build constraint lookup tables
- Initialize vertex configuration tables
- Set up face adjacency relationships

**Result**: Always `Success` (move to next predicate)

### 2. InnerFacePredicate (`src/predicates/innerface.rs`)

**Purpose**: Find maximal 5-face degree signatures.

**Algorithm**:
- Non-deterministically choose face degrees for the 6 five-faces
- Verify degrees sum to 27 (= 6-edges + 5-edges)
- Apply monotonicity constraints
- Verify signature is maximal under D₆ symmetry

**Result**:
- `Success` when found maximal signature → proceed to VennPredicate
- `Failure` when no more signatures → backtrack (search complete)

**Expected**: ~39 maximal signatures for N=6

**Status**: ✅ Complete

### 3. VennPredicate (`src/predicates/venn.rs`)

**Purpose**: Find valid facial cycle assignments (main search).

**Algorithm**:
1. Select face with fewest remaining cycle choices (fail-fast heuristic)
2. Non-deterministically assign a cycle to that face
3. Apply constraint propagation:
   - Edge adjacency: If faces F, F' are edge-adjacent at color j, and i,j,k is in cycle(F), then k,j,i must be in cycle(F')
   - Non-adjacency: If F and F' differ by one color j but aren't edge-adjacent, j appears in neither cycle
   - Vertex adjacency: If F, F' meet at vertex where colors i,j cross, and i,j is in cycle(F), then i,j is in cycle(F')
4. Repeat until all faces have cycles assigned
5. Validate solution (face cycles, vertex configurations)
6. Check solution canonicality under D₆ symmetry

**Result**:
- `Success` when found canonical solution → continue search (no next predicate yet)
- `SuccessSame` when backtracking after logging solution
- `Failure` when constraints violated or solution non-canonical

**Expected**: 233 canonical solutions for N=6 (plus ~14 equivocal duplicates that are filtered)

**Status**: ✅ Complete (Phase 7)

**Constraint Propagation**: See `src/propagation/` for detailed implementation

**Corner Detection**: ✅ Implemented and essential (`src/propagation/corner_detection.rs`).
The Carroll 2000 corner detection algorithm validates that each curve can be drawn with ≤3 corners.
This constraint is critical - without it, the search would produce many invalid solutions that cannot
be realized as triangles. The check runs during facial cycle assignment to fail early on non-realizable
diagrams. Note: This validates corner *requirements* (enabling the correct 233 solutions), but doesn't
yet assign specific corner positions to edges (Phase 8 enhancement)

### 4. FailPredicate (`src/engine/mod.rs`)

**Purpose**: Terminal predicate that always fails, forcing exhaustive search.

**Result**: Always `Failure`

## Future Phases

### Phase 8: Full Corner Assignment (Enhancement Needed)

**Current**: Corner detection validates that curves require ≤3 corners
**Needed**: Complete corner assignment and geometric realization

**Enhancements**:
- Implement `CornersPredicate` to assign specific corner positions
- Bidirectional traversal (forward and backward) to determine corner placement sub-paths
- Ensure ≥3 corners per curve (add arbitrary corners if needed)
- Validate crossing counts (≤6 per color pair for triangles)
- Integrate with PCO for line crossing order constraints

**Status**: 🚧 Partially implemented (validation only)

### Phase 9: GraphML Output

**Purpose**: Write solution to GraphML file with corner positions.

**Status**: 🚧 Not yet implemented

## Module Structure

### Core Modules

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `geometry/` | Type-safe geometric primitives | Color, Edge, Vertex, Face, Cycle |
| `geometry/constants.rs` | Compile-time constants | NCOLORS, NFACES, NEDGES, NVERTICES |
| `geometry/color.rs` | Edge labels and color sets | Color, ColorSet |
| `geometry/cycle.rs` | Facial cycles | Cycle, CycleSet |
| `geometry/edge.rs` | Directed edges | Edge, EdgeRef |
| `geometry/vertex.rs` | Oriented vertices | Vertex, VertexConfig |
| `geometry/face.rs` | Face regions | Face, FaceId |

### State Modules

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `trail/` | Backtracking support | Trail, TrailEntry, Checkpoint |
| `state/` | Mutable search state | DynamicFaces, DynamicEdge, Statistics |
| `state/faces.rs` | Face cycle assignments | DynamicFaces |
| `state/statistics.rs` | Performance counters | Statistics, Counters |

### MEMO Modules

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `memo/` | Immutable lookup tables | MemoizedData |
| `memo/cycles.rs` | Cycle constraint tables | CycleConstraints |
| `memo/vertices.rs` | Vertex configurations | VertexMemo |
| `memo/faces.rs` | Face relationship tables | FaceMemo |

### Algorithm Modules

| Module | Purpose | Key Files |
|--------|---------|-----------|
| `engine/` | Non-deterministic search | mod.rs, predicate.rs |
| `predicates/` | Search phases | initialize.rs, innerface.rs, venn.rs |
| `propagation/` | Constraint propagation | adjacency.rs, non_adjacency.rs, vertices.rs |
| `symmetry/` | Dihedral group D₆ | s6.rs (canonicality checking) |

### Context Module

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `context/` | Search context | SearchContext (owns MEMO + DYNAMIC state) |

## Type Safety Through Newtypes

Rust's type system prevents many errors at compile time:

```rust
// Each geometric concept has its own type
pub struct Color(u8);           // 0..NCOLORS-1
pub struct ColorSet(u64);       // Bitmask of colors
pub struct Cycle(u16);          // Index into cycle table
pub struct CycleSet(u128);      // Bitmask of possible cycles
pub struct Edge { ... };        // Directed edge with color
pub struct Vertex { ... };      // Oriented meeting point
pub struct Face(u8);            // 0..NFACES-1
```

These types prevent:
- Mixing up color indices with cycle indices
- Using face IDs where edge IDs are expected
- Bit manipulation errors

## Vertex Structure and Edge Organization

At each vertex, two curves intersect. One curve (the **primary**) crosses from inside the other curve
(the **secondary**) to outside it. This creates a natural orientation at the vertex.

Each vertex has 4 edges meeting at it: 2 of the primary color and 2 of the secondary color.
These edges are organized into 4 slots based on their orientation and face relationships:

**Edge slot mapping**:
- **Slot 0**: Primary color, clockwise edge, when secondary color contains the face
- **Slot 1**: Primary color, counterclockwise edge, when secondary color excludes the face
- **Slot 2**: Secondary color, counterclockwise edge, when primary color contains the face
- **Slot 3**: Secondary color, clockwise edge, when primary color excludes the face

This structure is critical for:
- Constraint propagation (vertex adjacency constraints)
- Determining which faces meet at each vertex
- Validating vertex configurations during search

The 480 possible vertex configurations (for N=6) are precomputed during initialization,
with each configuration specifying the valid edge arrangements for a given set of face colors.

## Memory Architecture for Parallelization

**Current Status**: Single-threaded, but architecture is parallelization-ready.

**Design Decision**: Two-tier memory model enables independent `SearchContext` instances.

**Parallelization Point** (future): After InnerFacePredicate finds each degree signature,
spawn independent thread for VennPredicate + CornersPredicate + GraphML output.

**Key Architectural Decisions**:
- No global mutable state (unlike the C implementation)
- Each `SearchContext` owns its MEMO and DYNAMIC data
- MEMO data could be Arc-shared across threads (future optimization)
- Enables Send + Sync for SearchContext (with appropriate marker traits)

**Expected Speedup**: 5-10x on modern multi-core systems (39 degree signatures, ~6-8 cores typical)

## MEMO vs DYNAMIC Annotations

Throughout the codebase, fields may be conceptually marked as MEMO or DYNAMIC:

- **MEMO**: Set during initialization, never changes. Example: `vertices: Vec<VertexConfig>` in MemoizedData
- **DYNAMIC**: Changes during search, tracked on trail. Example: `cycle: Option<Cycle>` in DynamicFace

This distinction is architectural documentation, not enforced by types (yet).

## Naming Conventions

The Rust implementation follows standard Rust conventions:

- **Modules**: `snake_case` (e.g., `geometry`, `state`)
- **Types**: `PascalCase` (e.g., `SearchContext`, `PredicateResult`)
- **Functions**: `snake_case` (e.g., `set_cycle`, `try_pred`)
- **Constants**: `UPPER_SNAKE_CASE` (e.g., `NCOLORS`, `NFACES`)
- **Lifetimes**: `'a`, `'ctx` (rarely needed due to ownership design)

## Error Handling

The implementation uses two approaches:

**1. Result types for search failures**:
Constraint propagation returns `Result<(), PropagationFailure>` to handle search failures gracefully:

```rust
pub enum PropagationFailure {
    NoMatchingCycles { face_id: usize, depth: usize },
    ConflictingConstraints { ... },
    CrossingLimitExceeded { ... },
    TooManyCorners { ... },
    DisconnectedCurve { ... },
    DepthExceeded { depth: usize },
}
```

These errors indicate search branches that should fail and backtrack, not program bugs.

**2. Panics for invariant violations**:
Used for conditions that should never occur if the algorithm is correct:

```rust
.expect("Face must have cycle assigned")
```

These indicate programming errors, not search failures.

## Testing Strategy

See **[TESTS.md](TESTS.md)** for complete test documentation.

**Test Organization**:
- Unit tests: Inline with modules (using `#[cfg(test)]`)
- Integration tests: `tests/` directory
- Feature flags: `ncolors_3`, `ncolors_5`, `ncolors_6` for different N values

**Test Approach**:
- Validate solution counts against known results
- Verify constraint propagation correctness
- Test trail backtracking mechanics
- Validate engine predicate execution

## Performance Considerations

**Current Performance** (Phase 7, N=6, release mode):
- Full search: ~3.5 seconds
- 233 solutions found
- ~39 inner face degree signatures

**Optimization Priorities**:
1. Trail operations (millions of calls) - already optimized
2. Predicate try/retry - hot loop
3. Constraint propagation - already optimized with bitsets
4. Memory allocations - minimized in hot paths

**Profiling**:
```bash
# Build with optimizations
cargo build --release

# Run with profiler (requires cargo-flamegraph)
cargo flamegraph --bin venn-search
```

**Key Optimizations Applied**:
- Trail uses unsafe for direct memory access (carefully encapsulated)
- CycleSet uses bit manipulation for set operations
- Constraint propagation uses precomputed lookup tables
- Fail-fast heuristic (choose face with fewest options)

## Corner Detection Algorithm

The Carroll 2000 corner detection algorithm is implemented in `src/propagation/corner_detection.rs`.

### Current Implementation (Phase 7)

**Purpose**: Validate that each curve can be drawn with ≤3 corners (triangles have 3 corners).

**Algorithm** (from [Carroll 2000]):

For each curve C, we start with its edge on the central face and walk around the curve.
We maintain two sets:
- **Out**: curves outside of which we currently lie
- **Passed**: curves we've recently crossed from inside to outside

Starting with both sets empty, at each vertex v where curve C meets curve C':
- If C' is in Out:
  - Remove C' from Out
  - If C' is also in Passed: Clear Passed and count a corner (there must be a corner between this and the previous vertex in the result set)
- Otherwise (C' not in Out):
  - Add C' to both Out and Passed

At the end of the walk, the count tells us the minimum corners required for this curve.

**When Called**: During `VennPredicate` constraint propagation, when assigning facial cycles to faces.
The check runs on each affected edge to fail early on non-realizable diagrams.

**Limitation**: This validates corner *requirements* but doesn't assign specific corner positions to edges.

### Future Enhancement (Phase 8+)

A full `CornersPredicate` would:
- Assign specific corners to edge positions (18 corners total for 6 triangles)
- Ensure crossing count validation (≤6 crossings per color pair)
- Integrate with PCO (Partial Cyclic Orders) for line crossing constraints
- Generate GraphML output with corner positions

## References

**Algorithm**:
- [Carroll 2000]: Carroll, Jeremy J. "Drawing Venn triangles." HP LABORATORIES TECHNICAL REPORT HPL-2000-73 (2000).

**Design Patterns**:
- Byrd, Lawrence. "Understanding the control flow of Prolog programs." in _Proceedings of the Logic Programming Workshop in Debrecen, Hungary_ (Sten Åke Tärnlund, editor). 1980.
  See [notes on Byrd box model](https://github.com/dtonhofer/prolog_notes/blob/master/other_notes/about_byrd_box_model/README.md).

**Related Documentation**:
- [MATH.md](MATH.md) - Mathematical foundations
- [TESTS.md](TESTS.md) - Test suite documentation
- [CLAUDE.md](../CLAUDE.md) - Development guide
- [CLEANUP.md](CLEANUP.md) - Code review and cleanup tasks
