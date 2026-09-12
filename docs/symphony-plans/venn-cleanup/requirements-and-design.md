# Venn cleanup: requirements and design

## Document contract

| Field                             | Value                                                                                                                                                                                     |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Project                           | [Venn cleanup for the next stage](https://linear.app/1000lines/project/venn-cleanup-for-the-next-stage-75b1ed33f42c)                                                                      |
| Design issue                      | [100-110](https://linear.app/1000lines/issue/100-110/create-requirements-and-design-doc)                                                                                                  |
| Repository / selected base        | `jeremycarroll/venn-search-rs` / `main`                                                                                                                                                   |
| Project code / color / human lead | `venn-cleanup` / `red` / Jeremy Carroll (`jeremycarroll`)                                                                                                                                 |
| Source snapshot                   | 2026-09-12; main `2af35ba8dc839c84bd3bd7f0bdef593b4160fe89`; PR #14 `c3b25341d1028f40a5c7bb3eb431b84730193735`                                                                            |
| Status                            | Proposed design for review; O1 is a material counting-contract discrepancy                                                                                                                |
| Next artifact                     | [100-111](https://linear.app/1000lines/issue/100-111) produces `fan-out-plan.md` after design acceptance; [100-112](https://linear.app/1000lines/issue/100-112) performs accepted fan-out |

This document defines outcomes, decisions, acceptance checks and ownership constraints.
It is not a fan-out plan: there are no implementation tickets, branch manifests,
execution rounds or generated relation payloads here. The brief's A–L candidates
are evaluated below, not approved as twelve tickets. Stable R, D, L, V and O
identifiers let the planning ticket cite requirements without inventing scope.

## Goal and boundaries

Finish useful cleanup from PR #14 so a future researcher can understand and safely
extend the successful search. Preserve the algorithm, trail-based restoration,
canonicality and observable behavior while making ownership, propagation, tests
and contributor documentation accurate. Prepare a clear boundary for later
linear-programming work, without designing a solver interface.

In scope: recover valuable unmerged cleanup with attribution; close the trail
safety boundary; clarify genuinely complex code; consolidate demonstrably
duplicated fixtures; fill specific regression gaps; correct misleading docs;
measure the identified cycle-selection opportunity; reconcile the final ledger.

Out of scope:

- LP solver implementation or dependency selection; corner assignment,
  stretchability/realization, PCO/Chirotope features and new mathematical claims.
- New CLI or GraphML formats, parallel search, generic predicate combinators,
  static-dispatch conversion, global MEMO caches or speculative sharing.
- Rewriting the search or changing its accepted solution set as incidental cleanup.
- Broad formatting/naming sweeps, arbitrary file-size quotas or a speedup quota.
- New Symphony infrastructure, a second Rust CI pipeline, deployment, a local
  Cargo installation or a Docker prerequisite.

Existing corner-count and crossing-limit pruning are part of the search to
preserve. They do not establish geometric realizability or implement the future
corner-assignment stage. The later mathematics project starts from documented
facial-cycle/edge/vertex data and the existing library entry points; this project
adds no speculative LP types, adapters or output format.

## Verified starting point

### Main versus PR #14

PR #14 is **open and unmerged** at the snapshot. Its seven commits contain 501
insertions and 561 deletions across 18 paths relative to its merge base, largely
relocations. Submitted reviews, inline comments, conversation comments and
thread-aware review results were all empty when read on 2026-09-12. Its successful
2025 CI is historical evidence, not acceptance for a recovery against today's main.

| Observation           | Main                                                                                                              | PR #14                                                 | Design consequence                                                                          |
| --------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| Debug globals/output  | `symmetry/s6.rs` has FIRST_CALL, DEBUG_FIRST and CALL_COUNT; innerface and disconnection contain temporary output | Removed                                                | Recover the removals; retain useful startup and structured failure diagnostics              |
| Symmetry organization | `s6.rs`, with group constants in `mod.rs`                                                                         | `canonical.rs`, constants/macro moved, exports updated | Recover canonical naming; preserve old public import paths through compatibility re-exports |
| Dynamic edge          | `geometry::EdgeDynamic` alongside MEMO types                                                                      | `state::DynamicEdge`                                   | Recover location/name; retain a compatibility alias for the old type/path                   |
| Context               | `mod.rs`: 482 lines                                                                                               | `mod.rs`: 315; `dynamic.rs`: 104; `memoized.rs`: 83    | Recover split; the commit-message/checklist line counts are not exact current measurements  |
| Propagation           | `mod.rs`: 127 lines, already split by constraint                                                                  | Same split                                             | Retire the proposed 800-line-module split                                                   |
| MEMO                  | `mod.rs`: 59 lines; cycles/faces/vertices: 577/671/740                                                            | Same split                                             | Inspect concrete constructors; no new module split just to satisfy the checklist            |
| Recent main fixes     | Onboarding/remote CI and Clippy fixes in `advanced_test.rs` and `tests/common/mod.rs`                             | Predates them                                          | Preserve main; do not replace files wholesale from the stale branch                         |

The checked “Break up large mod.rs files” parent in PR #14 does not mean all
children were implemented there. Its propagation/memo children say deferred,
although both splits already exist on main. Its other checked parents describe
unmerged work.

### Counting evidence: a discrepancy to keep visible

The commissioned brief and README/CLAUDE claim N=3/4/5/6 counts **2/3/23/233**.
The executable source does not support that entire claim:

| Configuration | Executable evidence on pinned main                                                                                                              | Baseline evidence                                  |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| N=3           | `venn3_test::test_venn3` and `venn_integration_test::test_venn_search_ncolors_3_baseline` assert 2                                              | Main CI passed                                     |
| N=4           | `venn_integration_test::test_venn_search_ncolors_4` asserts **16**                                                                              | Main CI log explicitly records this test passing   |
| N=5           | `venn_integration_test::test_venn_search_ncolors_5` asserts **17**; dedicated signatures test 6, 2, 4 and 5 solutions, plus rejected signatures | Main CI log explicitly records these tests passing |
| N=6           | `venn6_test::test_all` asserts 233 solutions and 39 inner-face signatures                                                                       | Main CI passed                                     |

Evidence: [main CI run 34714460922](https://github.com/jeremycarroll/venn-search-rs/actions/runs/34714460922),
push event at the pinned main SHA. PR #14 changes no count assertions.
Do not reinterpret these numbers as isomorphism classes versus labellings without
evidence. Do not change an assertion to 3/23 or call those counts verified.

O1 records the human decision needed if 3/23 is intended as a behavior requirement.
The conservative proposed contract is to preserve the executable baseline and
correct unsupported documentation. Independent recovery, safety design and
readability work do not depend on explaining the mathematical discrepancy.

### Safety, control flow and coverage findings

- `SearchContext` publicly exposes movable `state` and `trail`. Its safe
  setters store addresses of inline arrays and Vec elements. Public replacement,
  movement or Vec mutation can invalidate recorded pointers; a caller can also
  supply unrelated state/trail instances to safe propagation functions.
  `NonNull` proves non-nullness, not lifetime. The existing
  `test_raw_pointer_safety` only rewinds an unmoved context and proves no such
  general guarantee. This is a source-level safety finding, not a claim that a
  dynamic memory checker was run.
- `set_face_possible_cycles` duplicates word/count mutation in context and core.
  Raw trail writes occur in context, core, setup, vertices and disconnection.
  `CrossingCounts::get_mut_ptr` is another part of that boundary.
- `SearchEngine::search` clears stack and counters each time it is entered,
  including after it returns `Some(engine)` on Suspend. Existing tests exercise
  suspension, but do not establish continuation from the suspended instruction.
  Preserve actual re-entry behavior and explain it; a new continuation protocol
  would require a separate behavior decision.
- `propagation/vertices.rs` has a commented-out call to `edge_curve_checks`.
  Disconnection helpers remain implemented but are not activated by that call.
  The completed-color block exists in core, but its accumulator is populated by
  the disabled path. “TODO” removal must not claim that path is active.
- N=4 count coverage exists; a dedicated N=4 structural/monotonicity check does
  not. MEMO already tests clockwise orientation, incoming slots, primary/secondary
  selection, unique vertex IDs and incoming edge colors. Do not re-port those
  same tests under new names. `tests/debug_dihedral.rs` prints all groups but
  has no assertions.
- `CycleSetIter` scans bits, and `choose_next_cycle` restarts the iterator at
  the beginning then finds a value greater than the cursor. The optimization
  opportunity is real; its performance impact has not been measured here.

## Requirements and acceptance criteria

| ID  | Required outcome                   | Acceptance                                                                                                                                                                                                                         |
| --- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Preserve and reconcile prior work  | Every L row below has a final disposition and evidence. Valuable PR #14 changes are accounted for with original authorship/provenance; no unmerged work is represented as delivered                                                |
| R2  | Preserve successful search         | Existing four feature suites, canonicality, known-solution and signature assertions pass unchanged unless an explicit reviewed behavior decision supersedes one. Resolve O1 before claiming the brief's numeric contract satisfied |
| R3  | Make trail mutation safe           | D3's owned state/log contract is enforced by APIs; no safe call can replay retained pointers into moved/freed/unrelated state. Validate movement, independent contexts, rewind/freeze and encoding boundaries                      |
| R4  | Clarify propagation and engine     | Named helpers expose actual cascade/vertex/control-flow steps; preserve error variants, depth handling, corner rules, retry cursor, failure and suspension/re-entry behavior                                                       |
| R5  | Preserve module and API boundaries | Keep MEMO immutable during search and DYNAMIC per context. Recover existing splits, preserve benign import compatibility, and document the narrow safety-related API migration                                                     |
| R6  | Add useful regression evidence     | Add missing N=4 structure and D5 action assertions; retain existing geometry coverage. Fixtures use isolated output resources. New tests target behavior or safety risks, not line rearrangements                                  |
| R7  | Teach actual implementation        | Contributor docs explain initialization/data flow, trail costs/encodings, adding a predicate, NCOLORS matrix, verified counts and the future mathematics boundary. Runnable examples use real APIs                                 |
| R8  | Measure before optimizing          | Comparable runner evidence determines whether the cursor improvement is retained. Correctness precedes timing; a documented no-change result satisfies this requirement                                                            |
| R9  | Keep changes reviewable            | Small coherent PRs, normally about 100–400 substantive changed lines. Explain larger mechanical recovery/API migrations; assign one writer per file and explicit handoffs                                                          |
| R10 | Close with evidence                | All required current-head CI checks and current-head Cadence review, closed mandatory feedback, human acceptance, final CLEANUP ledger with PR links and explicit future deferrals. No deployment requirement                      |

## Design decisions

These are the proposed design's binding defaults once human-reviewed. A later
plan can choose ticket boundaries and helper names without revisiting product
scope. Any behavior change outside them must be recorded as a design amendment.

### D1: recover PR #14 before dependent code cleanup

Prefer reuse of the existing PR where practical. Re-read its head, commits and all
feedback immediately before recovery; compare current main to avoid duplicating
anything newly merged. Do not merge, close, force-push or overwrite Jeremy's old
branch as an unattended recovery shortcut.

If the old PR cannot provide a clean, current-main-based task diff, prepare
replacement recovery PRs from main. Apply the relevant commit changes selectively,
retain Jeremy Carroll and the existing co-author attribution where patches are
reused, and link the original commit SHAs in each recovery PR. Leave merge/closure
of #14 to Jeremy. Do not carry its final checked checklist wholesale into main.

Recovery source commits, in order:

| Commit                                     | Content                                                                    |
| ------------------------------------------ | -------------------------------------------------------------------------- |
| `e8a11022f7f8a9fd6ba17d30df6aca6578ec05b9` | Temporary debug/static-mut removal                                         |
| `2043d0abf750e2f1eaf5137e02b82266c9b1e14b` | Symmetry module and dynamic-edge relocation                                |
| `ef9132690f76c3be1c3d5d8b82aae08798d553bd` | Context split                                                              |
| `ed3cbfddce0bd31add93c382485393dedde74892` | Symmetry formatting                                                        |
| `01ccad32a9304a0c3bad3a1f49366b81a286cd6d` | Integration-test import repair                                             |
| `5179807c4e13fb46c202bd3ace47d124ced773af` | DynamicEdge naming                                                         |
| `c3b25341d1028f40a5c7bb3eb431b84730193735` | Historical checklist updates; reconcile rather than copy completion claims |

Retain MEMO initialization summaries, test failure details and
`PropagationFailure` payloads. The removed disconnection warning carries the
same counts as its returned error; callers must still expose useful failures.
Do not introduce a logging dependency.

### D2: retain the current modular architecture

Keep `memo/{cycles,faces,vertices}.rs` and existing propagation submodules.
After recovery, `context/memoized.rs` owns initialization and
`context/dynamic.rs` owns the mutable-state definition. `geometry/edge.rs`
keeps EdgeMemo/EdgeRef/CurveLink; `state/edge.rs` owns DynamicEdge and its encoding.

Keep separate degree-signature and full-solution canonicality functions together
in `symmetry/canonical.rs`; they share group definitions. Add cheap compatibility
re-exports for `symmetry::s6`, `geometry::EdgeDynamic` and
`geometry::edge::EdgeDynamic`. These aliases do not duplicate implementations.
Preserve existing root exports, SearchContext constructors, EngineBuilder,
Predicate and the consuming `search(&mut SearchContext) -> Option<Self>` entry point.

Review MEMO constructors only at concrete readability seams:
vertex parameter calculation duplicated across two initialization passes,
face-to-vertex partner-edge calculation and monotonicity/adjacency construction.
Keep cycle numbering, table layout, per-context ownership and exports stable.
The initialized placeholder edges in the vertex constructor are construction
steps, not unfinished product stubs.

### D3: use an owned, indexed undo log to close the trail boundary

Choose a small closed set of field/index undo entries, replayed against the
owner's state, instead of retaining arbitrary `NonNull<u64>` addresses. This
preserves recording old values and reverse replay while removing the need for
address stability. It also avoids introducing a generic mutation framework.

A private owner within SearchContext holds both DynamicState and its Trail.
Public callers can read state and inspect log length/checkpoints through methods,
but cannot replace either half or obtain unrestricted mutable state/Vec access.
Propagation mutators receive a borrow of this paired owner, plus immutable MEMO,
rather than separately supplied `&mut DynamicState` and `&mut Trail`.
Internal visibility must keep unchecked mutations within the owning module.

The closed target inventory is:

| Target                  | Address by logical index | Mutation semantics                                                                       |
| ----------------------- | ------------------------ | ---------------------------------------------------------------------------------------- |
| Face degree             | round                    | Record old u64 before assignment                                                         |
| Current cycle           | face                     | Forced assignment/reset trailed; retry cursor uses a specifically named untrailed setter |
| Possible-cycle word     | face, word               | Record only changed words                                                                |
| Cached cycle count      | face                     | Update together with possible-cycle changes                                              |
| Edge connection         | face, color              | Record checked encoded CurveLink                                                         |
| Next/previous dual face | face, direction          | Record checked optional face ID                                                          |
| Pair crossing count     | ordered color pair       | Validate bounds and ordering before recording                                            |
| Vertex processed        | vertex ID                | Trail first processing                                                                   |
| Edge color count        | direction, color         | Trail each increment                                                                     |
| Color checked           | color                    | Trail completion flag                                                                    |

Temporary `colors_completed_this_call`, statistics, output lifecycle and the
intentional retry cursor are not ordinary undo-log writes. Expose narrow methods
for them rather than a general mutable-state escape. Preserve their current
lifecycles; adding output streams or new search phases is deferred.

Each setter validates its indices/encoding before mutation, records the old value,
then writes the new value. Rewind pops entries in reverse against the same private
owner. Keep freeze semantics and the existing 16,384-entry overflow contract.
Keep checkpoint origin/range requirements explicit; do not expose a way to attach
one owner's log to another owner. Reset/reinitialization must clear or replace
state and log together.

The narrow compatibility exception is necessary: public mutable state/trail field
access, freely paired propagation mutation arguments and raw-pointer helpers
cannot remain safe APIs with their present contract. Supply read accessors,
context checkpoint/rewind/freeze methods and the paired propagation entry points;
migrate all repository callers in the same coherent safety change. Do not retain
a safe legacy shim that can separate the pair. Document this migration for users.

Acceptance must cover moved owner followed by rewind, two independently mutated
contexts, repeated writes to the same field, nested checkpoints, freeze,
partial propagation failure, cached-count restoration and intentional untrailed
cursor persistence. Privacy/compile-fail examples should demonstrate that callers
cannot swap state and trail or resize trailed storage through a mutable escape.
A normal rewind test alone is insufficient.

Alternatives considered:

| Alternative                               | Decision and rationale                                                                                                                                          |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Wrap existing pointer writes in methods   | Rejected: does not constrain movement, Vec reallocation, replacement, pairing or aliasing                                                                       |
| Pin/box existing public state             | Rejected as a standalone fix: exposed replacement/mutable access still defeats the contract; correct pinning also needs a complete lifetime and aliasing design |
| Private stable storage plus pointer trail | Viable alternative with a larger unsafe proof burden; not selected for this cleanup                                                                             |
| Closed indexed undo entries               | Selected: auditable safe indexing and owned pairing; may change entry size and dispatch cost, which must be measured                                            |
| Clone whole state at each choice          | Rejected: abandons incremental trail restoration and adds avoidable copying                                                                                     |

The Rust documentation explains that [NonNull may still dangle](https://doc.rust-lang.org/std/ptr/struct.NonNull.html)
and that [pinning requires an address-stability contract](https://doc.rust-lang.org/std/pin/).
The repository-specific conclusions above follow from the exposed fields and
call sites; no speed or memory improvement is claimed for the selected design.

Checkpoint acquisition and individual word updates are O(1); recording is O(1)
within the preallocated bounded log. Rewind is **O(k)** for k entries undone.
Changing a CycleSet costs O(w) for w words inspected, with only changed words
recorded. Replace unconditional “O(1) backtracking” and ownership-equals-pinning
claims wherever the module/document owner encounters them.

### D4: centralize encoding without changing valid values

Retain the existing wire-free internal encodings:

| Value              | Encoding                                                       | Required checks                                                                                       |
| ------------------ | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Optional CycleId   | 0 = None; id + 1 = Some(id)                                    | CycleId below NCYCLES; no overflow                                                                    |
| Optional face ID   | 0 = None; id + 1 = Some(id)                                    | ID below NFACES                                                                                       |
| Optional CurveLink | 0 = None; bit 63 = Some; face bits 0–5, color 6–8, vertex 9–17 | Validate configured-domain and packed-field bounds before packing; do not silently mask invalid input |
| Flags / masks      | 0/1 processed flag; color masks in u64                         | Only valid vertices/colors/directions accepted                                                        |

Put helpers with their state types, share the optional-index rule where genuinely
duplicated, and keep packed words private to mutation/encoding code. Use ordinary
assertions or checked constructors for invalid public inputs in release builds;
debug assertions followed by truncation are insufficient. Round-trip None, zero,
highest valid IDs, changed words across 63/64, and rejected out-of-range values.

### D5: clarify existing behavior before changing control flow

For vertex propagation, separate existing-link handling, first vertex processing,
incoming-edge linking/counting and corner validation. Preserve the existing
monotonicity-related skip behavior and the central-face corner-check exemption.
Do not turn the skipped link conflict into a new failure without behavioral evidence.

For restriction/setup, expose assigned-face validation, intersection/empty
failure, cached update, forced assignment and cascade as readable steps.
Keep depth-limit checks and operation order. Replace stale completed-color and
“7 instead of 6” comments with an accurate account of the existing inactive path.

For disconnection, preserve the disabled production call and state why it remains
deferred. Retain the helper code for now; clarify traversal assumptions and add
bounded termination for malformed/non-returning link cycles if the helper is
retained as callable code. Dedicated fixtures must distinguish open paths,
closed loops and disconnected/malformed structures, without enabling pruning.

For engine work, keep boxed predicates and the existing choice-point helper.
Simplify dispatch only where it reduces duplicate handling while preserving
try/retry restrictions, terminal enforcement, counters and OpenClose behavior.
Add a characterization test for repeated search after Suspend before refactoring;
document its restart/re-entry behavior. This project does not silently promise
instruction-level continuation that the current implementation lacks.

### D6: tests and documentation have explicit scope

Retain tests with distinct predicates or diagnostics; extract only identical
setup/run/count plumbing between venn5/venn6 and related integration fixtures.
A helper must express expected counts and pipeline differences directly.

Tests currently create files named from the shared “solution” prefix. Consolidated
helpers must give each test an isolated output directory/prefix and clean up only
their files, preserving output lifecycle coverage. Do not create shared writable
external resources when later tasks run independently.

Add N=4 structural checks to the existing N=4 integration location: every assigned
cycle belongs to its MEMO set, monotonicity constraints hold and dual face cycles
validate. Creating `venn4_test.rs` solely to fill a filename gap is unnecessary.
Replace the print-only dihedral test with assertions covering all ten D5
permutations, uniqueness and signature maximization; keep existing D3/D6 tests.
Add geometry tests only for gaps at changed linking/encoding boundaries, not for
already-covered primary/secondary or incoming-slot logic.

Contributor docs need a real predicate example that compiles in doc tests,
including failure/choice/terminal behavior and the owned mutation API. Use Mermaid
or existing in-repo illustrations for one MEMO data-flow view and useful
vertex/edge test cases. Do not introduce an image generator or documentation
framework. Distinguish historical research output/timing in RESULTS from current
Rust capabilities. `src/main.rs` currently prints “Hello, world!”; correct CLI
claims rather than implement a CLI.

### D7: bound performance work to evidence

After correctness and relevant file handoffs, compare the existing cursor scan
with a small cursor-aware/word-aware selection change in
`geometry/cycle_set.rs` and `predicates/venn.rs`. Preserve ascending order,
empty/exhausted behavior, valid-ID bounds and partial last-word handling.
A word scan can skip empty words and start strictly after the cursor; tests must
cover 63/64 transitions, the highest valid ID and repeated retries.

Use comparable GitHub runner executions: same runner class, Rust version,
release flags, feature, workload and output handling; record both commit SHAs,
warmup/repetition protocol and all samples/median. Separate MEMO initialization,
test file I/O and compilation from the selected measurement. Compare total
search time as well as the targeted operation. Existing CI establishes correctness;
job duration alone is not a microbenchmark. Use existing CI/test workload tooling,
not a duplicate workflow.

No optimization lands on the strength of the historic 3.5-second figure.
Retain it only if correct, simple and supported by the comparison; otherwise
record a no-change finding and preserve the existing implementation. Measure
trail representation cost as part of the safety change's evidence; a material
regression requires investigation before acceptance, not an unreviewed return
to an unsound API.

## Exhaustive historical cleanup disposition

Statuses: **present** means observed on main; **recover** means useful only in
unmerged PR #14; **action** means accepted cleanup outcome; **obsolete** means
the old proposed task no longer applies; **defer** means future work, not done.
Mixed parents are expanded below. Owner names describe responsibilities for the
planning ticket, not approved implementation nodes.

### Specific cleanup items and subitems

| ID  | Historical item/subitem                    | Disposition                | Evidence, rationale and owner                                                                                         |
| --- | ------------------------------------------ | -------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| L01 | Remove debug output (parent)               | recover                    | D1; recovery owner                                                                                                    |
| L02 | s6 DEBUG_FIRST/logging                     | recover                    | PR #14 also removes FIRST_CALL/CALL_COUNT and static_mut allowance                                                    |
| L03 | InnerFace debug output                     | recover                    | Active/commented eprintln removals in #14                                                                             |
| L04 | Other temporary debug code                 | recover + action           | Recover curve debug removals; module owners remove dead debug comments; keep useful diagnostics                       |
| L05 | Break up large mod.rs (parent)             | mixed                      | L06–L08 determine completion, not parent's checkbox                                                                   |
| L06 | Split ~800-line propagation/mod.rs         | obsolete; present          | Actual 127-line dispatcher and existing constraint files                                                              |
| L07 | Split memo/mod.rs                          | obsolete; present          | Actual 59-line exports module; review concrete constructors under D2 instead                                          |
| L08 | Separate MemoizedData/DynamicState         | recover                    | Context split in #14; retain re-exports                                                                               |
| L09 | Reorganize symmetry (parent)               | recover                    | L10–L12 plus compatibility in D2                                                                                      |
| L10 | Rename s6 to dihedral/canonical            | recover                    | Select canonical as already implemented in #14                                                                        |
| L11 | Move dihedral constants/macro              | recover                    | Located in canonical at #14 head                                                                                      |
| L12 | Separate degree and solution checking      | present + recover          | Already distinct functions; recover module naming, no extra split                                                     |
| L13 | Move EdgeDynamic (parent)                  | recover                    | L14–L16                                                                                                               |
| L14 | Separate from static geometry data         | recover                    | EdgeMemo/EdgeRef/CurveLink stay in geometry                                                                           |
| L15 | Place dynamic edge with trail state        | recover                    | state/edge.rs in #14                                                                                                  |
| L16 | DynamicFace/EdgeDynamic naming mismatch    | recover                    | DynamicEdge rename, compatibility alias; fix leftover EdgeDynamic module prose in #14                                 |
| L17 | Documentation review (parent)              | action                     | D6, docs owner plus module-local writers                                                                              |
| L18 | Module purpose/relationships               | present + action           | Existing module docs need actual names/data-flow corrections                                                          |
| L19 | Option/u64 trail encodings                 | present + action           | Existing field docs; centralize/validate with D4                                                                      |
| L20 | MEMO versus DYNAMIC                        | present + action           | Existing separation; correct initialization, copying and safety claims                                                |
| L21 | Complex-function examples                  | action                     | Runnable predicate/mutation example and concrete propagation cases                                                    |
| L22 | Simplify complex functions (parent)        | action                     | D5; avoid line-count refactors                                                                                        |
| L23 | check_face_vertices helpers                | action                     | Linking/counting/corner phases in 230-line file                                                                       |
| L24 | restrict_face_cycles cascade               | action                     | core.rs; preserve assigned/empty/singleton/depth semantics                                                            |
| L25 | Long engine methods                        | present + action           | Choice-point helper already exists; characterize re-entry then simplify dispatch                                      |
| L26 | Naming consistency (parent)                | action                     | Module-owned only; no overlapping global sweep                                                                        |
| L27 | snake_case/PascalCase mix                  | obsolete as blanket task   | Rust functions/types legitimately use different conventions; fix only concrete misleading names                       |
| L28 | Unclear s6 abbreviation                    | recover                    | D2 canonical name and compatibility path                                                                              |
| L29 | Short-lived variable names                 | action where ambiguous     | Preserve clear loop/index names; module owner explains geometric roles                                                |
| L30 | Trail API improvements (parent)            | action                     | D3/D4 are the safety acceptance contract                                                                              |
| L31 | Wrap all unsafe trail modifications safely | action, revised design     | Indexed undo removes stored pointers rather than hiding them                                                          |
| L32 | Unsafe code exposed in callers             | action                     | Atomic caller migration under sole safety owner                                                                       |
| L33 | Encapsulate sentinels                      | action                     | D4; no public writable packed words                                                                                   |
| L34 | Remove commented-out code (parent)         | action + defer             | Remove stale development fragments; preserve explicit future ledger                                                   |
| L35 | Remaining TODOs/comments                   | action                     | Initialize is already done in context; Venn is not a skeleton; retired 7/6 and completed-color claims need correction |
| L36 | Implement, explain deferral or remove      | action                     | Disabled edge_curve_checks stays deferred; future MEMO/PCO/output wishes move to visible deferrals                    |
| L37 | Test organization (parent)                 | action                     | D6; retain distinct behavior and diagnostic coverage                                                                  |
| L38 | common has one helper / more utilities     | present + action           | FixedInnerFacePredicate exists; share only proven duplicated plumbing                                                 |
| L39 | venn3/5/6 duplication                      | action                     | Preserve different feature/pipeline behavior and give outputs isolated paths                                          |
| L40 | Performance opportunities (parent)         | action + defer             | D7 gates any retained optimization                                                                                    |
| L41 | Eliminate hot-path allocations             | defer absent evidence      | Profile first; no general allocation rewrite                                                                          |
| L42 | Word-level CycleSet iteration              | action, conditional change | Existing bit scan and cursor TODO verified; a measured no-change result is acceptable                                 |
| L43 | Profile bottlenecks first                  | action                     | Comparable runner evidence, no historical timing claims                                                               |
| L44 | More idiomatic Rust (parent)               | action where useful        | Per-module readability, no mechanical combinator conversion                                                           |
| L45 | C-style loops                              | action where clearer       | Keep indexing when it expresses geometry/table correspondence                                                         |
| L46 | Iterator combinators                       | action where clearer       | No generic rewrite or performance presumption                                                                         |

### Architecture, documentation and testing gaps

| ID  | Historical item/subitem                                 | Disposition                                     | Evidence, rationale and owner                                                                               |
| --- | ------------------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| L47 | Stronger trail entry typing                             | action                                          | Closed field/index target inventory in D3                                                                   |
| L48 | Encapsulate all unsafe operations                       | action                                          | Remove retained pointer operations; private state/log ownership                                             |
| L49 | Document trail invariants                               | action                                          | Pairing, replay order, freeze, cursor exceptions, O(k) rewind                                               |
| L50 | Static predicate dispatch                               | defer                                           | No demonstrated bottleneck; retain Box<dyn Predicate>                                                       |
| L51 | Reusable predicate combinators                          | defer                                           | Existing builder/OpenClose suffice; no new use case                                                         |
| L52 | Better state management between predicates              | action, bounded                                 | D3 pairing and D5 re-entry documentation; no new engine protocol                                            |
| L53 | MEMO static references                                  | defer                                           | Preserve per-context owned MEMO                                                                             |
| L54 | Lazy table initialization                               | defer                                           | Existing eager initialization works; no measured need                                                       |
| L55 | Arc sharing for parallelization                         | defer                                           | Parallel search is outside commissioned scope                                                               |
| L56 | Architecture overview in lib.rs                         | present + action                                | Overview exists but claims unimplemented Corners/parallel output and wrong signature counts                 |
| L57 | Explain trail backtracking                              | present + action                                | Existing explanation has wrong O(1)/safety/API claims                                                       |
| L58 | Visual MEMO data flow                                   | action                                          | In-repo Mermaid/illustration tied to actual initialization and consumers                                    |
| L59 | Key-operation performance characteristics               | action                                          | Accurate asymptotic costs; measurements carry ref/environment                                               |
| L60 | Adding predicates example                               | present + action                                | Existing engine examples; add correct mutation/terminal example under D6                                    |
| L61 | Existing trail/backtracking coverage                    | present, incomplete safety proof                | Keep eight integration tests and unit coverage; replace overclaimed raw-pointer test with boundary evidence |
| L62 | Existing engine coverage                                | present, incomplete re-entry proof              | Keep ten integration tests; characterize repeated search after Suspend                                      |
| L63 | Full counts N3/N5/N6 and claimed N4 coverage            | present + disputed docs                         | Executable 2/16/17/233; O1 tracks brief 2/3/23/233                                                          |
| L64 | Canonicality and Carroll known solution                 | present                                         | Keep symmetry assertions and known_solution_test                                                            |
| L65 | Vertex configurations at each face type                 | present + targeted action                       | MEMO incoming-edge tests exist; only add missing changed-link cases                                         |
| L66 | Primary/secondary and clockwise orientation             | present                                         | memo/vertices.rs and geometry/edge.rs tests already cover these                                             |
| L67 | Internal validation only indirect / low-priority action | obsolete as blanket claim                       | Both direct MEMO and count tests exist; dynamic propagation gaps get focused tests                          |
| L68 | All ten D5 operations                                   | action                                          | Replace print-only debug_dihedral with group-action assertions                                              |
| L69 | Signature maximization across labellings                | action                                          | D5 permutation fixture; do not alter canonicality rule                                                      |
| L70 | Isomorphism count coverage / low priority               | present + action                                | Counts alone insufficient for action-specific regressions; preserve existing assertions                     |
| L71 | Visual face/vertex/edge test diagrams                   | action                                          | Explain selected cases using existing illustrations or Mermaid                                              |
| L72 | Visual teaching priority                                | action                                          | D6, docs owner; no diagram for every test                                                                   |
| L73 | Missing dedicated venn4 file/monotonicity               | present + action                                | N4 count test exists; add structural assertions there, no filename-only ticket                              |
| L74 | N4 is low priority/not production                       | revised                                         | N4 remains a required CI job and structural regression target                                               |
| L75 | proptest geometric properties                           | defer dependency; action on finite invariants   | Small finite domains can be covered with ordinary loops; no new crate without a demonstrated need           |
| L76 | Deep-tree stress tests                                  | defer                                           | Keep existing deep-nesting tests and bounded overflow check; no stress project                              |
| L77 | Memory/leak tests during backtracking                   | action on changed safety; defer broad profiling | D3 movement/owner tests required; optional memory tooling is supplementary evidence                         |
| L78 | Parallel execution tests                                | defer                                           | Parallel search is not implemented in this project                                                          |
| L79 | Port C vertex validation                                | present + targeted action                       | Existing orientation/slot/ID tests cover much of it; port only a proven gap after reading its source        |
| L80 | Visual diagrams for Rust test cases                     | action                                          | Same outcome as L71, one docs owner                                                                         |

### Assessment, metrics and suggested sequencing

| ID  | Historical claim/suggestion                                                | Disposition                       | Evidence and consequence                                                                                                      |
| --- | -------------------------------------------------------------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| L81 | Core search/canonicality working                                           | present                           | Existing source and baseline CI; maintain R2                                                                                  |
| L82 | 233 solutions and competitive 3.5s versus 5s                               | present count; historical timing  | No current comparable C/Rust measurement; D7                                                                                  |
| L83 | Trail, MEMO/DYNAMIC, predicate architecture                                | present + action                  | Preserve structure; repair unsafe ownership contract                                                                          |
| L84 | Comprehensive N3/4/5/6 tests / compile-time type safety                    | present with limits               | Feature jobs exist; O1 and D3 show why those claims are not complete proof                                                    |
| L85 | Clarity, debug, docs, naming and complexity assessment                     | action                            | Covered by L01–L46 and named module owners                                                                                    |
| L86 | Estimated component/total line counts                                      | obsolete                          | Replace with a pinned inventory if useful; do not use stale ~5000/~800 metrics as scope                                       |
| L87 | Milestone: search, propagation and canonicality complete                   | present with qualification        | Preserve active behavior; do not infer disabled disconnection or realization is complete                                      |
| L88 | Next: cleanup                                                              | action                            | This project                                                                                                                  |
| L89 | Next: corner detection/Phase 8                                             | defer assignment; present pruning | Existing corner validation stays; future assignment/realization separate                                                      |
| L90 | Next: GraphML/visualization                                                | defer implementation              | Preserve historical results and test diagnostics; correct current capability claims                                           |
| L91 | Next: CLI/usability                                                        | defer implementation              | README commands are unsupported by current main.rs                                                                            |
| L92 | Suggested review order: debug, modules, splits, docs, safety, architecture | revised                           | Recover first, then enforce safety before independent caller readability; retire obsolete splits and speculative architecture |

## Candidate work-unit assessment and ownership constraints

These assessments deliberately change the brief's proposed decomposition. The
plan owner must enumerate exact files, exclusions, expected substantive sizes and
tests from the accepted design and refreshed source. No A–L letter is an issued
ticket or a required node count.

| Candidate                      | Refined outcome / ownership constraint                                                                                                                                                                                                                                                          |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A: recovery                    | Keep first. A single mechanical recovery may exceed 400 lines; split debug/symmetry-edge/context only if review benefits. Own all recovery imports/compatibility exports sequentially; no later writer edits those files before recovery lands                                                  |
| B: trail                       | Keep as safety owner, **expand mechanical migration ownership** to every affected caller/test so no unsafe compatibility seam is published. Own trail, context/state mutation and geometry/corner raw-pointer removal. This cross-file integration precedes C/D/E/F/H/I edits to the same paths |
| C: vertices                    | Keep focused readability and dynamic-link regressions after B. Own vertices.rs and its module-local tests; no core/setup/disconnection edits                                                                                                                                                    |
| D: restriction/setup           | Keep coherent core.rs + setup.rs outcome after B. Own propagation/mod.rs exports/docs/tests during this handoff; coordinate color_removal/adjacency/non_adjacency only for actual shared signature work in B                                                                                    |
| E: disconnection               | **Reduce** to accurate inactive-path documentation and bounded helper validation. No activation or new pruning; do not inflate it into an algorithm feature                                                                                                                                     |
| F: engine                      | Keep characterization/readability; B creates an actual API dependency, so F follows B's engine-call migration. Own engine/ and engine_integration_test.rs afterward                                                                                                                             |
| G: MEMO                        | **Narrow** to demonstrated constructor seams; no split of mod.rs. Can be independent of B after A if owned files are disjoint. One memo/mod.rs owner; preserve tables and numbering                                                                                                             |
| H: predicates/geometry hygiene | **Narrow and enumerate**: initialize.rs, innerface.rs, module docs and concrete geometry examples; no repo-wide naming sweep. B owns mutation-related geometry/corner work first. Reserve venn.rs/cycle_set.rs for J except A imports and B safety migration                                    |
| I: integration tests           | Keep tests/common, venn3/5/6, venn_integration, phase5, known_solution and debug_dihedral as an explicitly enumerated pool. Separate ownership of engine/trail tests remains F/B. May combine with a small E/D regression outcome only if file ownership remains exclusive                      |
| J: measured cycle selection    | Keep conditional optimization/no-change evidence after B and relevant correctness/tests. Sole later owner of cycle_set.rs and venn.rs, including their stale skeleton/TODO prose                                                                                                                |
| K: contributor docs            | Keep src/lib.rs, README, CLAUDE, DESIGN/TESTS and relevant capability notes in RESULTS. Wait for stable API contracts; module-local docs stay with module owners. No CLEANUP final-ledger edits                                                                                                 |
| L: final reconciliation        | Keep docs/CLEANUP.md plus integrated evidence. Fan-in after accepted deliveries; record deferrals and O1 outcome, no broad final refactor                                                                                                                                                       |

A safety integration PR may necessarily touch many files even with small
mechanical edits. Explain that exception instead of splitting ownership across
concurrent unsafe API migrations. Later readability work must reread the merged
result. No task commits predecessor changes absent from main.

Hard dependencies come only from required merged APIs, exclusive file handoffs
and final acceptance evidence. Prefer a shallow DAG; do not serialize disjoint
MEMO work behind unrelated propagation cleanup. The design→plan→fan-out seeds
have direct blocker relations; after verified wiring the seeds are activated,
and unfinished predecessors hold their dependents. Implementation fan-out belongs solely to 100-112 after
the human-reviewed plan is merged.

## Validation and final evidence

### V1: this design seed

Validate Markdown formatting and links/section/ledger completeness locally using
available tools. Only this document changes. Rust is intentionally delegated to
GitHub; Cargo is absent on the hosted worker. Docker is skipped because the
explicit issue direction and selected-base config both select **remote**.
CI/PR/workpad evidence records the published SHA; a source baseline run is never
substituted for this document PR's current-head run.

### V2: every implementation and documentation PR

Preserve the existing `.github/workflows/ci.yml` and these six required
GitHub Actions checks, emitting App **15368**:

| Check                  | Existing command contract                                                                                        |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Test Suite (NCOLORS=3) | `cargo test --release --features ncolors_3 --verbose`; corresponding `--doc` command                             |
| Test Suite (NCOLORS=4) | `cargo test --release --features ncolors_4 --verbose`; corresponding `--doc` command                             |
| Test Suite (NCOLORS=5) | `cargo test --release --features ncolors_5 --verbose`; corresponding `--doc` command                             |
| Test Suite (NCOLORS=6) | `cargo test --release --features ncolors_6 --verbose`; corresponding `--doc` command                             |
| Clippy (Linting)       | `cargo clippy --all-targets -- -D warnings` and `cargo clippy --all-targets --features ncolors_5 -- -D warnings` |
| Format Check           | `cargo fmt --all -- --check`                                                                                     |

Each doc-test command is `cargo test --release --doc --features ncolors_N --verbose`
for that job's single N. NCOLORS features are mutually exclusive; never use
`--all-features`. Preserve release testing, both Clippy configurations and
existing doc-test coverage. Do not weaken failures or turn required jobs into skips.

Record target/base SHA, workflow/run URL, event, App, run attempt, actual child
results, useful local checks and explicit limitations. Missing, pending, stale,
canceled or skipped checks do not pass. CI pending uses Unhappy plus wake:15m;
failures require Active rework; passing checks use Inactive for review.
Cadence is a separate review gate: repository configuration names the HackCadence
App with Codex preference, and the actual reviewer/provider/head/verdict must be
observed. The setup probe is not a live review of the design PR.

### V3: focused implementation checks

- Safety owner: D3/D4 scenarios, public API migration examples and four-color
  regression matrix. Preserve the overflow/freeze contract and useful panic checks.
- Propagation owners: forced singleton, empty/conflicting restriction, depth
  limit, vertex-count rollback, valid/invalid orientation and the existing
  known-solution/corner rejection cases. Preserve `test_55433`'s assertion of 6.
- Engine owner: failure past deterministic frames, exhausted alternatives,
  try/retry invalid results, terminal behavior, OpenClose lifecycle and
  repeated search after suspension.
- Test owner: N4 structure plus D5 action coverage, fixture equivalence and
  isolated file outputs. Resolve O1 only using the authorized counting contract.
- Performance owner: D7 paired runner evidence, bit-boundary/exhaustion cases
  and unchanged search assertions. No benchmark infrastructure project.

### V4: project completion

Finalizer maps every accepted requirement and L row to merged PR(s), verified
base contents or an explicit deferral with rationale. Remove project-introduced
temporary code and correct checklist parents whose children differ.
Record integrated main CI, performance/no-change findings and accurate next-stage
entry points. A deferred feature is never checked off as implemented.

Every task PR is based on main, labeled `symphony` and `red`, assigned to
Jeremy Carroll, and starts draft. Normal human handoff requires all current-head
CI checks passing, fresh current-head Cadence approval, closed mandatory feedback
and ready status. Human acceptance/merge owns Done. No deploy evidence is needed.

## Material open decisions, assumptions and execution inputs

| ID  | Item                                                   | Owner / dependency / handling                                                                                                                                                                                                                                                |
| --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O1  | Brief 3/23 versus executable 16/17 for N4/N5           | Jeremy Carroll: confirm preserving executable baseline and correcting docs, or explicitly commission a behavior change. Blocks final numeric acceptance and any count-changing implementation, not independent cleanup design/recovery. Question recorded in 100-110 workpad |
| A1  | Cleanup preserves active search semantics              | Proposed default based on the commissioned goal; no inferred permission to alter enumeration, enable disconnection or implement a new resume protocol                                                                                                                        |
| A2  | No required source unavailable                         | All mandatory inputs read at snapshot; refresh access/head/feedback before downstream implementation                                                                                                                                                                         |
| E1  | Exact post-recovery file lists, diff sizes and aliases | Plan owner verifies source at planning time; execution discovery, not a product question                                                                                                                                                                                     |
| E2  | PR #14 reuse feasibility/current state                 | Recovery owner reads fresh PR and permissions; use D1 replacement path when required, leaving old PR closure to Jeremy                                                                                                                                                       |
| E3  | Rust toolchain and runtime performance                 | GitHub CI/performance owner records emitted version, runner and results; no host installation or speculative timing                                                                                                                                                          |
| E4  | Generated IDs, labels and direct relations             | Fan-out owner resolves/readbacks through shared tooling after accepted plan; no new approval question                                                                                                                                                                        |
| E5  | Credentials or source access failure                   | Block only dependent work, record exact operation/source and named operator; never fabricate source contents or live evidence                                                                                                                                                |

The plan owner can refine node boundaries, exact helper names and disjoint file
lists using this document. O1 is the only outstanding product-level conflict
identified in the inspected sources. If its answer requires changing search
behavior, amend the affected scope before creating count-changing tickets;
do not smuggle an algorithm correction into a cleanup node.

## Source-read ledger

All mandatory sources below were accessible and read on 2026-09-12. Refreshed
main still matched the commissioning SHA. No Google Doc is required. Historical
references linked from repository docs are not treated as newly verified research
claims; only the explicitly listed portions support this design.

| Source                                                                                                                                                                                                                                                                                                                                                                           | Ref / acquisition and evidence                                                                                                                                                                                               |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Linear issue and full project content                                                                                                                                                                                                                                                                                                                                            | Injected GraphQL; project/issue metadata and comments read; project brief is the commissioned scope                                                                                                                          |
| [CLEANUP on main](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/docs/CLEANUP.md)                                                                                                                                                                                                                                                 | Full file read; L01–L92 reconcile specific items, children, architecture/test suggestions, metrics and sequencing                                                                                                            |
| [CLEANUP on PR #14](https://github.com/jeremycarroll/venn-search-rs/blob/c3b25341d1028f40a5c7bb3eb431b84730193735/docs/CLEANUP.md)                                                                                                                                                                                                                                               | Full git object and diff read; checked parents distinguished from unmerged code                                                                                                                                              |
| [PR #14](https://github.com/jeremycarroll/venn-search-rs/pull/14)                                                                                                                                                                                                                                                                                                                | REST PR/files/commits/reviews/inline/conversation endpoints, GraphQL reviewThreads, git diff and status rollup read; seven commits listed in D1                                                                              |
| [README](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/README.md), [CLAUDE](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/CLAUDE.md), [TESTS](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/docs/TESTS.md)                       | Full files read; stale CLI, counts, examples, phase, feature-matrix and complexity claims checked against code                                                                                                               |
| [Config](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/.symphony.cfg.json), [CI](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/.github/workflows/ci.yml), [Cargo.toml](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/Cargo.toml) | Selected-base config helper: configured, team 100, remote; workflow and mutually exclusive features read                                                                                                                     |
| SYMPHONY.md, .github/symphony/REVIEW.md and PR template/guidance                                                                                                                                                                                                                                                                                                                 | Full files at pinned main read; no target AGENTS.md found in repository inventory                                                                                                                                            |
| [Source tree](https://github.com/jeremycarroll/venn-search-rs/tree/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/src)                                                                                                                                                                                                                                                                 | Full trail, relevant context/state/encoding, engine dispatch, propagation mutation/call paths, CycleSet/cursor, initialization and MEMO constructors/tests inspected; line inventory and unsafe/TODO searches                |
| [Tests](https://github.com/jeremycarroll/venn-search-rs/tree/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/tests)                                                                                                                                                                                                                                                                     | Feature gates, count assertions, fixtures, safety/suspend coverage and debug_dihedral read; no local Rust execution claimed                                                                                                  |
| [DESIGN](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/docs/DESIGN.md), [MATH](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/docs/MATH.md), [RESULTS](https://github.com/jeremycarroll/venn-search-rs/blob/2af35ba8dc839c84bd3bd7f0bdef593b4160fe89/docs/RESULTS.md)             | DESIGN architecture/trail/engine/phase/corner sections and RESULTS overview/table introduction read; MATH full file read for current-search/future-realization boundary; historical data is not current Rust output evidence |
| Main baseline CI                                                                                                                                                                                                                                                                                                                                                                 | Run 34714460922, pinned main SHA, success; N4/N5 named-test success confirmed from logs                                                                                                                                      |
| Rust NonNull/pin documentation                                                                                                                                                                                                                                                                                                                                                   | Official pages linked under D3, read for pointer/pinning guarantees; no dependency/version change proposed                                                                                                                   |
| Shared Symphony proof/review instructions                                                                                                                                                                                                                                                                                                                                        | Resolved through hosted SYMPHONY_TOOLING_ROOT at tooling ref c5c36da145f169dc4d1f223a470727784f140ff6; no operator-local path assumed                                                                                        |

Unavailable required sources: **none**. Unavailable local validation tool:
**Cargo**, intentionally delegated to remote CI. Unobserved evidence: future
implementation results, this PR's checks/review until published, optimization
measurements and final human acceptance; these belong in subsequent workpads
and PR evidence, not invented into this source snapshot.
