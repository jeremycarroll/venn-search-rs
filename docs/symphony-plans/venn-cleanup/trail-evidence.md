# Indexed trail safety evidence

SAFETY / [100-124](https://linear.app/1000lines/issue/100-124),
[PR #21](https://github.com/jeremycarroll/venn-search-rs/pull/21), implements
accepted D3/D4 against `main`, initially
`b5f48130a9ef5988db865b8df25d9da2b10c2507`. RECOVER is Done and
[PR #20](https://github.com/jeremycarroll/venn-search-rs/pull/20) is merged.
The accepted plan revision is `5ebf31261a9b651957cc82f6fed1059d3f2c3770`.

## API handoff

`SearchContext` privately owns a `TrailedState`. That owner privately holds both
`DynamicState` and `Trail`. Only `src/trail/mod.rs` records or replays entries;
there are no retained pointers. The public `Trail` export now provides metadata,
without standalone construction, mutation or replay. No mutable reference to
owned `DynamicState`, a face, an encoded word or a Vec escapes the owner.

| Previous access                               | Replacement                                                                                                  |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `ctx.state` reads                             | `ctx.state()`                                                                                                |
| `ctx.trail.len()`                             | `ctx.trail().len()`; metadata also exposes capacity and entry size                                           |
| `ctx.trail.checkpoint()/rewind_to()/freeze()` | `ctx.checkpoint()/rewind_to()/freeze()`                                                                      |
| Separate state/log arguments to propagation   | `let (memo, state) = ctx.parts_mut();` then the same function with `memo, state` and the remaining arguments |
| Direct retry-cycle assignment                 | `ctx.set_retry_cursor_untrailed(face, cycle)`                                                                |
| Output replacement                            | `ctx.replace_output(Option<Box<BufWriter<File>>>)`, returning the previous stream                            |
| Output writes with state reads                | `ctx.output_parts()` returns immutable MEMO/state/statistics and optional mutable writer                     |
| State replacement/reset                       | `ctx.reset_state()` replaces state/log/output together, retaining MEMO and statistics                        |

Existing constructors, degree/cycle setters and getters, root type exports,
EngineBuilder, Predicate and consuming search signatures remain. The historical
DynamicEdge import aliases remain. Standalone DynamicFace's unrestricted cursor
setter is removed; owned retry writes have an explicit untrailed name.
MEMO initialization and `context/memoized.rs` are unchanged.

The closed targets and public paired-owner mutations are:

| Target                  | Mutation method                      | Checked domain                                            |
| ----------------------- | ------------------------------------ | --------------------------------------------------------- |
| Face degree             | `set_face_degree`                    | Round below NCOLORS; degree remains an unrestricted u64   |
| Current cycle           | `set_face_cycle`, `reset_face_cycle` | Face below NFACES, cycle below NCYCLES                    |
| Possible-cycle word     | `set_face_possible_cycles`           | Valid CycleSet, bounded face and word                     |
| Cached cycle count      | Same method, inseparable from words  | Derived population count                                  |
| Edge connection         | `set_edge_connection`                | Face/color/vertex configured and packed bounds            |
| Next/previous dual face | `set_next_face`, `set_previous_face` | Both face IDs below NFACES                                |
| Pair crossing count     | `set_crossing_count`                 | Ordered pair i < j < NCOLORS                              |
| Vertex processed        | `mark_vertex_processed`              | Vertex below NPOINTS; first processing only               |
| Edge color count        | `increment_edge_color_count`         | Direction below 2, color below NCOLORS; checked increment |
| Color checked           | `mark_color_checked`                 | Color below NCOLORS; writes flag 1                        |

Every recorded write validates first and saves the old value before assignment.
Indices narrow to u16 only after checking. Optional indices retain 0/ID+1;
CurveLink retains bit 63 plus face/color/vertex fields, with assertions active
in release. `CycleSet::from_words` rejects unused high bits. Flags and completed
color masks can only be set through valid typed operations.

Checkpoint positions remain owner-local usize values: callers must use a position
from the same initialization that has not been discarded by rewind. Range checks
reject positions beyond the current log length. A freeze clamps rewinds at its
boundary. Reset invalidates all prior checkpoints. Whole-owner moves or replacement
cannot detach a log from its state; passing an unrelated position never selects
another owner's storage.

Recording and checkpoint acquisition are O(1); rewind is O(k) for k undone writes.
Cycle-set updates inspect O(w) words and record only changed words/counts. Their
entire required log capacity is checked first, so overflow cannot leave a cached
count inconsistent with its words. The 16,384-entry bound and repeated scalar
write recording remain intact.

`clear_completed_colors` and `add_completed_color` retain the temporary,
untrailed accumulator lifecycle. Retry cursors survive choice rewind, while the
trailed predicate-entry reset restores the prior cursor when that frame unwinds.
Output remains untrailed and follows the existing OpenClose lifecycle. The
FixedInnerFace test fixture now uses checked degree setters. Search control flow,
propagation order/depth/errors and the inactive disconnection call are preserved.

## Boundary checks

The existing integration checks retain independent contexts, nested checkpoints,
freeze, arrays and degree assertions. The previous unmoved raw-pointer test is
replaced with a moved-owner test that mutates all ten targets and compares the
complete restored state. Additional integration checks cover word/count rewind,
63/64 transitions where supported, cursor persistence, partial propagation failure,
paired reset and invalid release-domain inputs with unchanged state/log.

Module tests cover repeated-write overflow, multi-entry cycle-update overflow,
optional-index bounds, zero/highest CurveLink fields, configured crossing ordering
and rejected high CycleSet bits. Four compile-fail examples reject independent
state/log replacement and mutable Vec escapes; context/root examples exercise
the real paired API. Existing search counts 2/16/17/233 and all signature
assertions are preserved. O1 remains for final numeric acceptance.

## Validation and measurement status

Configured and effective mode: **remote**, from selected-base `.symphony.cfg.json`
and ticket V / explicit project human direction. Docker: **skipped — explicit
remote validation direction**. No image/digest is required. Rust tooling is
unavailable locally; Rust build, tests, doctests, Clippy and rustfmt are **unrun**.
Local source/scope inspection, `git diff --check`, `bash -n scripts/measure-cleanup.sh`
and Prettier 3.6.2 on this document are the available checks.

The six existing CI checks and all their original commands remain. The only
workflow addition is one tagged PR-only measurement step on the existing N=6 job:

```text
bash scripts/measure-cleanup.sh <event-base-main-SHA> <exact-event-PR-head-SHA>
```

The script fetches both immutable objects and creates disposable worktrees inside
that job workspace. It verifies clean source refs before injection. The standalone
ignored probe is copied from the exact head into both refs unchanged. A small
identical cfg(test)-only module is appended to each disposable trail source to
inspect private entry size/capacity. Its exact contents, diff and SHA-256, plus
the standalone probe hash, are printed. This is disclosed test instrumentation;
it is not baseline production code. Cleanup removes only those two worktrees and
their private scratch directory.

Both release/ncolors_6 executables are built before measurement. The required
Cargo invocation runs one warmup per ref; five measured executions per ref then
alternate baseline/head order serially. Each runs Initialize → InnerFace → Venn →
quiet counter → Fail and asserts 233. Timing begins after context/MEMO and engine
construction; compilation and file I/O are excluded. All samples and medians,
runner/image/toolchain, flags, source SHAs, entry sizes/capacities and storage bytes
are emitted to CI logs.

**Evidence gap:** the implementation's first runner execution is pending.
Baseline/head SHAs, observed layout/storage, samples/medians, run/attempt/job URLs
and any material regression investigation must be committed here after that run.
No speed or memory conclusion is claimed from source layout alone.

The two temporary files and only the tagged N=6 step carry
`VENN_CLEANUP_MEASUREMENT`. MEASURE may extend the probe/script; FINAL removes
those files and that step, retaining this evidence and every original CI command.
Ownership release to later tasks still requires fresh current-head CI/Cadence,
human acceptance and merge/Done.
