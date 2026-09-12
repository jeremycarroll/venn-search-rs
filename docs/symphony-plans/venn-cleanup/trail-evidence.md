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

### Observed paired run: 2026-09-12

[CI run 34722520057, attempt 1](https://github.com/jeremycarroll/venn-search-rs/actions/runs/34722520057/attempts/1),
[N=6 job 103630866316](https://github.com/jeremycarroll/venn-search-rs/actions/runs/34722520057/job/103630866316),
completed successfully, including tests, doctests and the paired measurement.
The event was `pull_request`, workflow `.github/workflows/ci.yml`, emitting
GitHub Actions App **15368**. The measured source refs were:

| Ref                                       | Immutable source SHA                       |
| ----------------------------------------- | ------------------------------------------ |
| Baseline `main`, including merged RECOVER | `b5f48130a9ef5988db865b8df25d9da2b10c2507` |
| SAFETY implementation                     | `2989908dfa8a8660e0f6bef8c91336ab43b16ec9` |

Exact step command:

```sh
bash scripts/measure-cleanup.sh b5f48130a9ef5988db865b8df25d9da2b10c2507 2989908dfa8a8660e0f6bef8c91336ab43b16ec9
```

Runner: `GitHub Actions 1000002308`, Linux/X64, image `ubuntu24`, version
`20260907.300.1`; host `runnervmlun5p`, kernel `6.17.0-1022-azure`.
Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`, commit
`48a229ceaefd4985c50990b14116b6d856af0985`, host
`x86_64-unknown-linux-gnu`, LLVM `22.1.8`; Cargo
`1.98.1 (797e8a9bc 2026-08-05)`. Both checkouts used `--release`,
`--features ncolors_6`, empty `RUSTFLAGS`, and the same Cargo profile
(`opt-level = 3`, `lto = true`, `codegen-units = 1`). No output files were written
by the quiet-counter workload.

Both standalone probes and both layout-test binaries were built before the first
warmup. The probe build command in each checkout was:

```sh
cargo test --release --features ncolors_6 --test cleanup_measurement --no-run --message-format=json
cargo test --release --features ncolors_6 --lib --no-run --message-format=json
```

The layout command was
`cargo test --release --features ncolors_6 --lib venn_cleanup_measurement_layout -- --ignored --nocapture --test-threads=1`.
Each warmup used the required
`cargo test --release --features ncolors_6 --test cleanup_measurement -- --ignored --nocapture --test-threads=1`.
Each measured command invoked that ref's already-built
`target/release/deps/cleanup_measurement-f8dff99e21725758 --ignored --nocapture --test-threads=1`
from its own disposable checkout. The full absolute paths are in the job log.
No other tests ran concurrently with these measurements.

The script verified each checkout's HEAD and clean Git status before injection.
The exact-head standalone probe was identical in both checkouts, SHA-256
`0433c3638105364be3bf60e86d1ffaca96654d94b58c060d38f384fab457f228`.
The appended layout module had SHA-256
`ad366d199141fcf72dfea8fdd77ee0aa1d7c4f35169c8c2c0bdeaadb2ff7e01e`.
Its complete contents, including a leading blank line, were:

```rust

#[cfg(test)]
mod venn_cleanup_measurement_layout {
    #[test]
    #[ignore = "VENN_CLEANUP_MEASUREMENT"]
    fn entry_layout() {
        let trail = super::Trail::new();
        println!("VENN_CLEANUP_MEASUREMENT entry_bytes={} capacity={} storage_bytes={}",
            std::mem::size_of::<super::TrailEntry>(), trail.entries.capacity(),
            std::mem::size_of::<super::TrailEntry>() * trail.entries.capacity());
    }
}
```

The log discloses the append-only `src/trail/mod.rs` diff for each ref. The
baseline also receives the standalone probe as a new test file; no baseline
production implementation is replaced. These are test-only measurement changes,
not committed baseline code. The later formatting/documentation correction does
not change the measured probe, layout injection or executable Rust behavior;
fresh CI is still required on that correction's head.

### Entry storage and full-search samples

| Measurement                     |                Baseline |           Indexed owner |
| ------------------------------- | ----------------------: | ----------------------: |
| Entry size, including alignment |                16 bytes |                16 bytes |
| Reserved capacity               |          16,384 entries |          16,384 entries |
| Reserved entry storage          | 262,144 bytes (256 KiB) | 262,144 bytes (256 KiB) |

These figures measure `size_of::<TrailEntry>() * entries.capacity()`, not total
process memory or allocator overhead. No trail-entry storage regression was
observed on this runner.

All twelve searches completed without suspension and asserted **233** results.
Times below are nanoseconds, starting after context/MEMO and engine construction.
Warmups are retained here but excluded from the medians.

| Repetition            | Execution order |       Baseline ns |  Indexed owner ns |
| --------------------- | --------------- | ----------------: | ----------------: |
| Warmup                | baseline, head  |     4,583,224,310 |     6,984,317,629 |
| 1                     | baseline, head  |     4,575,033,898 |     6,896,742,472 |
| 2                     | head, baseline  |     4,596,701,197 |     6,900,872,156 |
| 3                     | baseline, head  |     4,686,529,825 |     6,965,681,146 |
| 4                     | head, baseline  |     4,590,577,621 |     6,840,075,220 |
| 5                     | baseline, head  |     4,574,516,144 |     6,824,262,735 |
| Median, measured only |                 | **4,590,577,621** | **6,896,742,472** |

The median ratio is **1.502369**, a material **50.24%** slowdown, or about
**2.31 seconds** per full search on this runner.

### Regression investigation and disposition

The slowdown is present in every alternating pair (head/baseline ratios
1.507474, 1.501266, 1.486320, 1.490025 and 1.491800). Baseline samples span
4.57–4.69 s and head samples 6.82–6.97 s, with no overlap. Changing execution order
does not remove it; it cannot reasonably be presented as an isolated noisy sample.
The same input, probe hash, toolchain, release flags and 233-result assertion were
used for both refs. Construction, compilation and file I/O are outside the timed
region. The layout results rule out larger trail entries or reserved capacity as
the explanation for this measured cost.

Source comparison identified added work in the mutation boundary:

- The old recorder receives a precomputed address and rewind directly stores
  through it. The indexed owner resolves `Target` through `slot` while recording
  and again during rewind, including checked array/Vec access. Setters also
  validate configured indices and encodings in release builds.
- `set_face_possible_cycles` now counts changed words before writing, reserving
  capacity for the entire word/count update. At N=6 this adds a pass over seven
  words; the prior implementation could overflow partway through that update.
  Per-entry overflow checks remain. This preflight provides the tested atomic
  overflow behavior.
- Neither implementation clones the full search state for checkpoints. Both
  retain a bounded, preallocated log and record only changed cycle words/counts.
  Inspection of the migrated engine, Venn predicate and propagation callers
  found the same selection/cascade order and existing failure/depth conditions;
  the migration does not deliberately expand the search workload.

These are source-level explanations of additional work, not a sampled CPU
profile or a measured attribution of the 50.24% among individual operations.
The bounded P experiment measures their combined full-search cost. This evidence
does not establish performance on other machines or a process-memory bound.

Retain the simple indexed implementation under accepted D3: it closes the
movement/pairing hazard and keeps the required release checks and atomic overflow
contract. The observed time cost is a review-visible tradeoff, not a speedup or
a no-regression claim. No unsafe pointer API, unchecked access, changed search
semantics or speculative optimization is introduced to recover timing. MEASURE
can use these refs and samples as context for its separately scoped cursor study;
that later study is not evidence that this slowdown has been recovered.

### CI coverage and remaining handoff

At measured head `2989908dfa8a8660e0f6bef8c91336ab43b16ec9`, the four NCOLORS
jobs passed their release tests and doctests: N3 job `103630866281`, N4
`103630866344`, N5 `103630866248`, N6 `103630866316`. This includes the trail
integration/module tests, privacy doctests and unchanged search assertions.
Clippy job `103630866286` failed `doc_lazy_continuation` on the new paragraph in
`src/propagation/core.rs`; Format job `103630866227` reported formatting changes.
That run's overall conclusion is **failure**, despite successful measurement.

The follow-up separates the module paragraph and applies the runner's 13 unique
rustfmt hunks across nine Rust paths. Rust changes are limited to whitespace,
optional trailing commas and the blank documentation separator; assertions are
preserved. Rustfmt/Clippy remain unrun locally; applying the emitted formatting
diff is not a local Rustfmt pass. Acceptance still requires all six checks and
fresh Cadence review on the follow-up head, plus mandatory feedback closure.

The two temporary files and only the tagged N=6 step carry
`VENN_CLEANUP_MEASUREMENT`. MEASURE may extend the probe/script; FINAL removes
those files and that step, retaining this evidence and every original CI command.
Ownership release to later tasks still requires fresh current-head CI/Cadence,
human acceptance and merge/Done.
