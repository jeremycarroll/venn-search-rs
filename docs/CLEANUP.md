# Clean up items

- [ ] break up non-trivial mod.rs files
- [ ] rename symmetry/s6 as symmetric, add dihedral (from mod)
- [ ] move EdgeDynamic to state and fix DynamicFace vs EdgeDynamic
- [ ] all unsafe blocks for modifying trail need to be wrapped in methods within the trail

## Disconnected Curve Detection - Optimization Implemented but Disabled

**Status**: Completed but waiting for VennPredicate integration

### What's Implemented

1. **Infrastructure**:
   - `edge_color_counts: [[u64; NCOLORS]; 2]` - tracks clockwise/counterclockwise edges
   - `colors_checked: [u64; NCOLORS]` - permanent tracking (trail-tracked)
   - `colors_completed_this_call: u64` - temporary accumulator (not trail-tracked)
   - `DisconnectedCurve` error variant

2. **Logic**:
   - Corner checking marks colors as completed when loops close
   - `remove_completed_color_from_search()` - restricts unassigned faces to omit completed colors
   - If restriction fails → disconnected curve error (some edges form separate component)

3. **Why Disabled**:
   - Check runs at depth==0, which includes `setup_central_face` (too early)
   - C code only checks in `dynamicFaceBacktrackableChoice` (engine-level), not in `dynamicFaceChoice`
   - Need VennPredicate integration to call at correct time (after search choices, not during setup)

**Current Behavior**: Finds 7 solutions instead of 6 for test_55433 (one invalid with disconnected curve)

**To Enable**: When implementing VennPredicate, uncomment code at src/propagation/mod.rs:304-311 and call from engine after face choices

## Disconnected Curve Detection (In Progress - OLD)

### Current Status

Infrastructure added but check not enabled. Test `test_55433` finds 7 solutions instead of 6.
Solution #7 has a disconnected curve that should be rejected.

### Infrastructure Added

1. **`edge_color_counts: [u64; NCOLORS]`** in `DynamicState` (src/context/mod.rs:151)
   - Tracks edges set up per color
   - Incremented in `check_face_vertices` when edge->to pointers are set
   - Trail-tracked

2. **`colors_checked: [u64; NCOLORS]`** in `DynamicState` (src/context/mod.rs:159)
   - Tracks which colors have been checked
   - Prevents redundant checks
   - Trail-tracked

3. **`DisconnectedCurve` variant** in `PropagationFailure` (src/propagation/mod.rs:95)

### Implementation Needed

In `count_corners_on_curve` (src/propagation/mod.rs:731), check when loop completes:

```rust
if next_face_id == start_face_id {
    if state.colors_checked[color_idx] == 0 {
        let total_edges = state.edge_color_counts[color_idx] as usize;
        if edges_visited < total_edges {
            return Err(PropagationFailure::DisconnectedCurve { ... });
        }
        // Mark checked (trail-tracked)
        trail.record_and_set(..., 1);
    }
    return Ok(Some(corner_state.corner_count()));
}
```

### Key Insights

- A face can have edges of colors not contained in it
- Check only when closed loop forms (returning to start_face_id)
- Check only once per color (via colors_checked)
- Disconnection = closed loop visits fewer edges than total set up

### C Reference

- `c-reference/edge.c:35` - dynamicCheckForDisconnectedCurve
- `c-reference/venn.c:152` - dynamicFaceBacktrackableChoice