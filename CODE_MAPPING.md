# C to Rust Code Mapping - Dynamic Search Components

This document maps the C reference implementation (v1.1-pco) to the Rust rewrite, focusing on **dynamic/trail-tracked components** of the search.

**Goal**: Exactly copy the mathematical logic from C, with only implementation details differing (memory safety, ownership, etc.).

---

## 1. Trail System (Backtracking)

| C Code | Rust Code | Description |
|--------|-----------|-------------|
| [trail.h](https://github.com/roll/venntriangles/blob/v1.1-pco/trail.h) | [src/trail/mod.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/trail/mod.rs) | Trail system for O(1) backtracking |

**C Implementation**:
```c
// trail.h
typedef struct {
    void *address;
    uint64_t oldValue;
} TrailEntry;
```

**Rust Implementation**:
```rust
// src/trail/mod.rs
pub struct TrailEntry {
    address: NonNull<u64>,
    old_value: u64,
}
```

**Key Difference**: Rust uses `NonNull<u64>` for type safety; C uses `void*`.

---

## 2. Search Engine

| C Code | Rust Code | Description |
|--------|-----------|-------------|
| [engine.c](https://github.com/roll/venntriangles/blob/v1.1-pco/engine.c) | [src/engine/mod.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/engine/mod.rs) | Non-deterministic search engine |
| [nondeterminism.c](https://github.com/roll/venntriangles/blob/v1.1-pco/nondeterminism.c) | [src/engine/mod.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/engine/mod.rs) | Choice point management |

**C Flow**: `tryPred()` → `retryPred()` loop with backtracking
**Rust Flow**: `try_pred()` → `retry_pred()` loop with backtracking

**Key Difference**: Rust uses trait-based predicates; C uses function pointers.

---

## 3. Dynamic Face State (Trail-Tracked)

| C Code | Rust Code | Trail-Tracked Fields |
|--------|-----------|----------------------|
| [dynamicface.h](https://github.com/roll/venntriangles/blob/v1.1-pco/dynamicface.h) | [src/state/faces.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/state/faces.rs) | `current_cycle`, `possible_cycles`, `cycle_count`, `edge_dynamic[]`, `next_face`, `previous_face` |

**C Structure** (dynamicface.h):
```c
typedef struct {
    // Trail-tracked fields
    uint64_t current_cycle_encoded;     // 0=unassigned, n+1=cycle n
    CycleSet possible_cycles;            // Bitset of valid cycles
    uint64_t cycle_count;                // Cached count
    EdgeDynamic edge_dynamic[NCOLORS];   // Per-edge dynamic state
    uint64_t next_face_id_encoded;       // Next in dual graph cycle
    uint64_t previous_face_id_encoded;   // Previous in dual graph cycle
} DynamicFace;
```

**Rust Structure** (src/state/faces.rs):
```rust
pub struct DynamicFace {
    pub(crate) current_cycle_encoded: u64,
    pub possible_cycles: CycleSet,
    pub cycle_count: u64,
    pub edge_dynamic: [EdgeDynamic; NCOLORS],
    pub(crate) next_face_id_encoded: u64,
    pub(crate) previous_face_id_encoded: u64,
}
```

**Encoding**: Both use `0=None`, `n+1=Some(n)` for trail compatibility.

---

## 4. Edge Dynamic State (Trail-Tracked)

| C Code | Rust Code | Trail-Tracked Fields |
|--------|-----------|----------------------|
| [edge.h](https://github.com/roll/venntriangles/blob/v1.1-pco/edge.h) (DYNAMIC section) | [src/geometry/edge.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/geometry/edge.rs) | `to_encoded` (edge→vertex pointer) |

**C Structure** (edge.h):
```c
struct edge {
    // ... MEMO fields ...

    // DYNAMIC (trail-tracked):
    uint64_t to_encoded;  // Vertex connection (sentinel encoding)
};
```

**Rust Structure** (src/geometry/edge.rs):
```rust
pub struct EdgeDynamic {
    pub(crate) to_encoded: u64,  // Encodes Option<CurveLink>
}
```

**Encoding**: Sentinel value (specific ID) means "not set"; non-sentinel means vertex assigned.

---

## 5. Crossing Counts (Trail-Tracked)

| C Code | Rust Code | Description |
|--------|-----------|-------------|
| [triangles.c](https://github.com/roll/venntriangles/blob/v1.1-pco/triangles.c) / [triangles.h](https://github.com/roll/venntriangles/blob/v1.1-pco/triangles.h) | [src/geometry/corner.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/geometry/corner.rs) | Triangle constraint: max 6 crossings per color pair |

**C Global**:
```c
// triangles.h
uint64_t crossing_counts[NCOLORS][NCOLORS];  // Upper triangle only
```

**Rust Structure**:
```rust
// src/geometry/corner.rs
pub struct CrossingCounts {
    counts: [[u64; NCOLORS]; NCOLORS],  // Upper triangle only
}
```

**Trail-tracked**: Yes, incremented during `check_face_vertices()`, restored on backtrack.

---

## 6. Vertex Processing (Trail-Tracked)

| C Code | Rust Code | Description |
|--------|-----------|-------------|
| [failure.c](https://github.com/roll/venntriangles/blob/v1.1-pco/failure.c) (`dynamicCheckFacePoints`) | [src/propagation/mod.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/propagation/mod.rs) (`check_face_vertices`) | Sets edge→vertex pointers, counts crossings |

**C Logic** (failure.c lines ~400-500):
1. For each edge in assigned cycle
2. Look up vertex from `possibly_to[][]`
3. Set `edge->to` pointer (trail-tracked)
4. If vertex not yet processed:
   - Increment `crossing_counts[i][j]` (trail-tracked)
   - Mark vertex processed (trail-tracked)
   - Check if exceeds MAX_CROSSINGS_PER_PAIR (6)

**Rust Logic** (src/propagation/mod.rs lines 367-469):
- **Identical algorithm**, line-for-line translation

---

## 7. Constraint Propagation

| C Code | Rust Code | Key Functions |
|--------|-----------|---------------|
| [failure.c](https://github.com/roll/venntriangles/blob/v1.1-pco/failure.c) | [src/propagation/mod.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/propagation/mod.rs) | Propagation logic |

### 7.1 Main Propagation Entry Point

| C Function | Rust Function | Description |
|------------|---------------|-------------|
| `failure.c::failurePropagate()` | `propagate_cycle_choice()` | Entry point after assigning cycle to face |

### 7.2 Propagation Steps (Both Implementations)

1. **Set face to singleton cycle** (trail-tracked)
2. **Check vertices** → `dynamicCheckFacePoints()` / `check_face_vertices()`
3. **Set next/previous face pointers** (trail-tracked)
4. **Edge adjacency** → `propagateEdgeAdjacency()` / `propagate_edge_adjacency()`
5. **Non-adjacent faces** → `propagateNonAdjacentFaces()` / `propagate_non_adjacent_faces()`
6. **Non-vertex-adjacent** → `propagateNonVertexAdjacentFaces()` / `propagate_non_vertex_adjacent_faces()`

| C Function | Rust Function | Uses Lookup Table |
|------------|---------------|-------------------|
| `propagateEdgeAdjacency()` | `propagate_edge_adjacency()` | `cycle->same_direction[]`, `cycle->opposite_direction[]` |
| `propagateNonAdjacentFaces()` | `propagate_non_adjacent_faces()` | `cycles_omitting_one_color[color]` |
| `propagateNonVertexAdjacentFaces()` | `propagate_non_vertex_adjacent_faces()` | `cycles_omitting_color_pair[i][j]` (upper triangle) |

### 7.3 Cascading Restriction

| C Function | Rust Function | Description |
|------------|---------------|-------------|
| `failure.c::restrictFaceCycles()` | `restrict_face_cycles()` | Intersect possible cycles, auto-assign if singleton |

**Key Behavior** (Both):
- If intersection → empty set: **Failure** (backtrack)
- If intersection → singleton: **Auto-assign** + recursive `propagate_cycle_choice()` (CASCADE)
- Otherwise: Update possible_cycles (trail-tracked)

---

## 8. S6 Symmetry Checking

| C Code | Rust Code | Description |
|--------|-----------|-------------|
| [s6.c](https://github.com/roll/venntriangles/blob/v1.1-pco/s6.c) | [src/symmetry/s6.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/symmetry/s6.rs) | Dihedral symmetry checking |

### 8.1 Degree Signature Check (Inner Face)

| C Function | Rust Function | When Called |
|------------|---------------|-------------|
| `s6.c::isCanonicalUnderDihedralGroup()` (6-element array) | `check_symmetry()` (6-element array) | InnerFacePredicate (degree signatures) |

**Both**: Check if `[d0, d1, d2, d3, d4, d5]` is lexicographically maximal under D₆.

### 8.2 Full Solution Check (Venn)

| C Function | Rust Function | When Called |
|------------|---------------|-------------|
| `s6.c::s6FacesSymmetryType()` (64-element array) | `check_solution_canonicality()` (64-element array) | **VennPredicate round 0** ⚠️ |

**C Code** (venn.c, `vennTryPred` function):
```c
// Around line 150-200 in venn.c
if (round == 0) {
    // ... setup_central_face ...

    // S6 check BEFORE starting search
    SymmetryType symmetry = s6FacesSymmetryType();
    if (symmetry == NON_CANONICAL) {
        return FAILURE;  // Prune this branch
    }
}
```

**Rust Code** (src/predicates/venn.rs, `try_pred` method):
```rust
// Currently MISSING or MISPLACED
if round == 0 {
    // ... setup_central_face ...

    // TODO: Add S6 check HERE
    match check_solution_canonicality(&ctx.state, &ctx.memo) {
        SymmetryType::NonCanonical => return PredicateResult::Failure,
        _ => {}, // Continue
    }
}
```

**⚠️ CURRENT ISSUE**: Rust implementation calls S6 check at wrong time or has logic error.

---

## 9. VennPredicate Main Logic

| C Code | Rust Code | Description |
|--------|-----------|-------------|
| [venn.c](https://github.com/roll/venntriangles/blob/v1.1-pco/venn.c) | [src/predicates/venn.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/predicates/venn.rs) | Main Venn search |

### 9.1 try_pred / tryPred

**C Logic** (venn.c):
```c
PredicateResult vennTryPred(int round) {
    if (round == 0) {
        // Setup central face (if needed)
        // S6 symmetry check
    }

    // Find unassigned face with min cycle_count
    face = chooseNextFace();
    if (!face) return SUCCESS;  // All assigned

    // Reset current_cycle to 0 (trail-tracked)
    // Return Choices(cycle_count)
}
```

**Rust Logic** (src/predicates/venn.rs):
```rust
fn try_pred(&mut self, ctx: &mut SearchContext, round: usize) -> PredicateResult {
    if round == 0 {
        // Setup central face (if needed)
        // S6 symmetry check ⚠️
    }

    // Find unassigned face with min cycle_count
    let face_id = choose_next_face(ctx);
    match face_id {
        None => PredicateResult::Success,  // All assigned
        Some(face_id) => {
            // Reset current_cycle to None (trail-tracked)
            // Return Choices(cycle_count)
        }
    }
}
```

### 9.2 retry_pred / retryPred

**C Logic** (venn.c):
```c
PredicateResult vennRetryPred(int round, int choice) {
    face = facesInOrder[round];

    // Choose next cycle after current_cycle (iterator)
    cycle = chooseNextCycle(face);

    // Set current_cycle directly (NOT trail-tracked)
    face->current_cycle = cycle;

    // Propagate constraints
    if (failurePropagate(face, cycle, 0) == FAILURE) {
        return FAILURE;
    }

    return SUCCESS_SAME_PREDICATE;
}
```

**Rust Logic** (src/predicates/venn.rs):
```rust
fn retry_pred(&mut self, ctx: &mut SearchContext, round: usize, _choice: usize) -> PredicateResult {
    let face_id = self.faces_in_order[round];

    // Choose next cycle after current_cycle (iterator)
    let next_cycle = choose_next_cycle(ctx, face_id, current_cycle);

    // Set current_cycle directly (NOT trail-tracked)
    ctx.state.faces.faces[face_id].set_current_cycle(Some(next_cycle));

    // Propagate constraints
    if propagate_cycle_choice(...).is_err() {
        return PredicateResult::Failure;
    }

    PredicateResult::SuccessSamePredicate
}
```

---

## 10. InnerFacePredicate

| C Code | Rust Code | Description |
|--------|-----------|-------------|
| [innerface.c](https://github.com/roll/venntriangles/blob/v1.1-pco/innerface.c) | [src/predicates/innerface.rs](https://github.com/jeremycarroll/venn-search-rs/blob/cornering/src/predicates/innerface.rs) | Degree signature search |

**Both Implementations**:
1. Enumerate partitions of `TOTAL_CENTRAL_NEIGHBOR_DEGREE` (27 for N=6)
2. Check each partition is canonical under D₆ symmetry
3. Call `setup_central_face()` with degree signature
4. Propagate to see if configuration is viable

---

## 11. Setup Central Face

| C Function | Rust Function | Description |
|------------|---------------|-------------|
| `venn.c::setupCentralFace()` | `setup_central_face()` | Initialize inner face + neighbors |

**Both Implementations**:
```
For i in 0..NCOLORS:
    face_id = ~(1 << i) & (NFACES-1)
    Restrict face to cycles of length face_degrees[i]

Set inner face (NFACES-1) to canonical cycle
Propagate constraints
```

---

## 12. Face Cycle Validation

| C Function | Rust Function | Description |
|------------|---------------|-------------|
| `venn.c::validateFaceCycles()` | `validate_face_cycles()` | Final validation check |

**Both**: Verify faces with M colors form single cycle of length C(NCOLORS, M) in dual graph.

---

## Current Debugging Focus

### Issue: N=6 Finding 15,087 Solutions (Expected: 233)

**Hypothesis**: S6 symmetry check not working correctly

**C Code Location**: `venn.c::vennTryPred()` around line 150-200
**Rust Code Location**: `src/predicates/venn.rs::try_pred()` around line 55-87

**Action Items**:
1. ✅ Add `check_solution_canonicality()` function
2. ⚠️ **Verify it's called at round 0 (not at solution end)**
3. ⚠️ **Verify colorset_permute logic matches C**
4. ⚠️ **Verify SEQUENCE_ORDER matches C's initializeS6()**

---

## Testing Strategy

To verify mathematical equivalence:
1. Compare intermediate states (trail dumps)
2. Compare propagation step counts
3. Compare solution counts per degree signature
4. Verify SEQUENCE_ORDER constant matches C initialization

---

## Status Legend
- ✅ **Mathematically Equivalent**: Logic verified against C
- ⚠️ **Under Investigation**: May have subtle bug
- 🚧 **Partial**: Structure correct, logic incomplete
- ⬜ **Not Started**
