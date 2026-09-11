---
name: symphony-project-factory
description: Use only when a human asks to create or update a Symphony Linear project with metadata and the three planning seeds, or explicitly requests a specific additional seed or ticket move.
---

# Symphony Project Factory

Use this skill when a human asks to start, create, wake, reshape, or update a
Symphony project. Default project creation produces project metadata and a brief
plus only three planning seeds: requirements/design, plan project, and trigger
fan-out. Apply those payloads through Linear once required inputs, labels, and
helper checks are resolved, then activate the ready planning frontier by default.
An explicit request to park the project, leave it in Backlog, or wait for a
specified start time overrides this default.

Use it before the project plan is approved and merged and before fan-out has
been triggered. Once project work is in progress, use the replanning path
instead of reshaping the project directly from this skill. A human may still
explicitly direct a specific seed addition or existing-ticket move; carry out
that bounded request without inferring further project changes.

Do not use this skill for ordinary implementation tickets, project finalization,
deploy handoffs, evidence collection, or Cadence review rework unless the human
is changing the project shape itself.

## Project Creation Boundary

Symphony owns project decomposition, implementation tickets, dependencies, and
final wrap-up through planning with human PR review and accepted fan-out. The
project factory must not preempt that process by inventing downstream tickets.
Its only default ticket dependencies are the two planning-seed blockers below.

Preserve publishing, review, deployment, cleanup, and finalization requirements
in the project brief and acceptance criteria. Mentioning these outcomes in an
end-to-end brief does not authorize creating tickets for them during setup.
Extraction wrap-up such as constructing the final artifact and preparing Example Reviewer
review (DEMO-543), publishing the approved extraction (DEMO-544), and removing
private scaffolding (DEMO-545) belongs in Symphony's reviewed plan.

Additional seeds require an explicit human request to create the particular
ticket, not merely confirmation that its subject is in project scope. A human
may also direct an existing ticket to be moved into a project. Example's move of
DEMO-534 into the not-yet-started SampleMetrics project was such a specific human
decision; it does not authorize agent-selected additions or moves.

This boundary applies to existing projects too. Inspect existing tickets and
reuse existing seeds rather than duplicating them. Apply only the requested
metadata, state, specific seed addition, or ticket move; route other downstream
additions through reviewed planning/fan-out or replanning. Requests such as
“create the project,” “get it going,” or “make tickets Active” do not authorize
extra ticket scope. A status request changes the requested tickets' states,
not the project ticket set.

## Read First

Before asking project questions or drafting payloads, read the smallest complete
set of required sources:

- The current Linear issue, its project description/content, labels, state,
  team, and `## Codex Workpad` when one exists.
- The source project description or planning request from the human.
- Every required issue, document, attachment, Google Doc, uploaded file, plan,
  prior PR, or other source material the human names.
- Prior fan-out examples relevant to the request, including committed plans in
  `docs/symphony-plans/` and old project plans the current issue names.
- `$SYMPHONY_TOOLING_ROOT/scripts/symphony/runtime-bundle/workflow/WORKFLOW.md`.
- `.agents/skills/karpathy-guidelines/SKILL.md`.
- Existing `.agents/skills/*/SKILL.md` frontmatter conventions and any local
  Symphony skills that the new project's starter tickets will reference.
- The project color helper output from the accepted color-helper implementation.
  When the helper is unavailable, drafting may continue with a visible
  `color-helper-output: unavailable` field, but live Linear writes must stop
  unless the human supplies equivalent verified availability output.

If a required source cannot be read, record `Unavailable source`, the source
name or URL, the access method attempted, and the exact failure in the workpad.
Do not guess its contents. Continue independent work; ask for access or pasted
contents only when the source is needed for the next decision or action. Use
`Human Input Needed` when operating inside Symphony if no independent work
remains and that access is required to proceed.

## Required Inputs

Resolve each field from sources, existing configuration, or the defaults below
before writing project or ticket payloads. Ask only for material unresolved
choices:

- Project goal.
- Out-of-scope boundaries.
- Acceptance criteria.
- Required source documents and whether each is blocking. Google Doc URLs,
  uploaded documents, Linear comments, repo docs, and PRs are all acceptable
  sources when they can be read.
- `project-code`.
- `project-color`, proposed from the F2-002 helper output. A human may override
  the helper suggestion, but preserve that override explicitly in the write
  plan.
- `base-branch`, defaulting to `main` only when the human or project metadata
  leaves it absent.
- `human-lead`, defaulting to the current human when no project metadata names
  a different lead. Resolve Linear assignee identity and GitHub username when
  known.
- Required GitHub PR labels, including the project color and `symphony` when the
  project contract requires it.
- Required Linear issue labels.
- The three planning seeds and any specific additions or omissions explicitly
  requested by the human.
- The exact human request for each additional seed or existing-ticket move, if
  any. Do not propose downstream tickets merely because they appear in scope.

Resolve the Linear team from the adopter's project configuration and verify it
through Linear before writing. `DEMO` is only an example team key. If no team is
configured, include the missing team in the write-plan questions.

Use available sources, existing project conventions, and stated defaults to
resolve routine choices. Record material assumptions without requiring human
confirmation. Do not invent observed IDs, permissions, credentials, or evidence.

Ask a human only when plausible answers would materially change scope,
implementation approach, acceptance criteria, or an authorized next action,
and the choice cannot be resolved from available context. Explain what decision
the answer changes. If the plan is the same either way, proceed: discovery of
App IDs or CI check names and compatibility tests are execution tasks, not
planning questions. Record externally supplied credentials as dependencies of
the work that needs them; continue independent planning and implementation.

## Project Description Rules

Render project descriptions from `templates/project-description.md`. The
description must include this metadata block near the top:

```yaml
project-code: <short-project-code>
project-color: <color-label>
base-branch: <base-branch>
human-lead: <full-name>
```

Resolve missing fields from verified sources or the defaults above. Keep only
material unresolved decisions in `Open Decisions`, with the affected work
identified. Preview output may show placeholders, but live writes require
resolved metadata, labels, and color availability. A missing API-required ID
must be looked up; if it remains unavailable, block only the dependent write.

Keep the full project scope in the brief, including delivery and wrap-up
outcomes. Only put an outcome into an additional seed payload when the human
explicitly requests that ticket. Scope confirmation alone is insufficient.

## Starter Tickets

Use only these three templates for the default starter ticket set:

| Template                     | Default title                      | Creation status | Status after setup                                    |
| ---------------------------- | ---------------------------------- | --------------- | ----------------------------------------------------- |
| `requirements-and-design.md` | `Create requirements & design doc` | `Backlog`       | `Active`, unless the human explicitly requests a hold |
| `plan-project.md`            | `Plan project - seed ticket`       | `Backlog`       | `Active`, held by requirements/design until Done      |
| `trigger-fan-out.md`         | `Trigger fan out`                  | `Backlog`       | `Active`, held by the planning relation until Done    |

Stage all seeds outside the dispatch queue, create the two blocker relations,
and read back the tickets and relation direction before activating anything.
After successful verification, move all three seeds to
`Active` without asking for another confirmation. Do not leave a newly created
project parked merely because the human did not separately say "start now".
Blocking is a derived property of unfinished predecessor relations, not a
workflow status. An Active dependent waits for its predecessor to reach Done;
no dependent status change is needed. Verify relation-aware dispatch before
activation. Never create or require a `Blocked` status or use `Do Not Use`.
If setup or relation verification fails, leave the seeds undispatched and report
the failure. Resolve these state names against the actual workspace and workflow;
do not assume an example `Todo` state is dispatchable.

The other templates (`broaden-fanout-integration.md` and `standup.md`) are
available only for a specific human-requested additional seed or later use
through Symphony's reviewed plan. Their availability is not a reason to create
those tickets at setup. Include the same contract fields for any explicitly
requested additional seed.

The requirements-and-design ticket should block the planning ticket. The
planning ticket should block the fan-out trigger ticket. Create these two Linear
blocker relations by default in live writes. Do not infer downstream
dependencies during project setup; Symphony's reviewed plan owns them.

## DAG Project Planning

DAG planning is the default for Symphony projects. Produce the ordinary
sequential fan-out contract only when the human request, project description,
requirements/design source, or uploaded spec explicitly asks for a linear plan or
asks to avoid a DAG. Even moderately complex projects should be nudged toward a
DAG because dependency patterns rarely linearize cleanly.

For DAG projects, the planning ticket must require:

- a Mermaid graph marked `%% symphony-dag/v1`, with graph diffs as the
  reviewable unit for topology changes;
- a `symphony-dag-manifest/v1` block with project metadata, defaults, nodes,
  edges, node types, branch declarations, PR policies, maturity defaults,
  direct relation semantics, and decisions. Do not include generated join
  artifacts, `base_node`, `stack_policy`, `stack:*`, branch-base exceptions,
  rebase-on-land, or neutralization fields;
- a branch manifest that names task branches, their selected base branch, birth
  policies, and draft PR policies;
- a Decisions section with rationale and the ticket or generated artifact that
  enforces each decision;
- Linear relation payloads for every hard DAG edge, preserving direction as
  `issueId: <blocker>`, `relatedIssueId: <blocked>`, `type: blocks`;
- direct fan-in semantics: represent fan-in with multiple direct incoming
  blocker relations to the downstream issue. Do not create no-op join tickets or
  generated join branches;
- `mature` as the default blocker-side maturity label. Set it only when the
  blocker PR's current head has the required checks and review approval needed
  for human review readiness.

For DAG projects, the fan-out trigger ticket must require live or dry-run
payload evidence for generated issues, task branch refs, required labels,
assignees, relation payloads, and any failed mutation. If a required `mature`
label, PR label, assignee, or blocker relation cannot be created, the trigger
must fail closed before creating a partial graph.

## Write Plan And Preview

Do not default to a no-write preview. The human's request to create, start,
wake, or update a project is the write intent for that operation within the
project creation boundary above. It does not authorize extra tickets. Once
required sources, metadata, labels, color availability, workflow states, and
identity mappings are resolved, proceed to live Linear writes through
`linear_graphql` without pausing for a second human response.

Before or during writes, print or record a concise write plan:

- Project metadata and unresolved questions.
- Source documents read and unavailable sources.
- Color helper output or an explicit `unavailable` marker.
- Linear team `DEMO`, labels, workflow states, and human lead identity
  resolutions.
- Rendered project description.
- Starter ticket payloads with title, initial status, labels, assignee,
  dependencies, blocker relations, and rendered description path.
- The specific human request authorizing each additional seed or ticket move;
  omit such mutations when there is no explicit request.
- The GraphQL mutation input objects that would be sent.

If the human explicitly asks for preview only, payloads only, or no writes, stop
after printing the write plan and do not mutate Linear.

## Linear Writes

Live Linear reads and writes must use Symphony's injected `linear_graphql` tool.

Before writing, resolve:

- Linear team id for `DEMO`.
- Workflow state ids for `Backlog` and the configured dispatch state
  (`Active` in 1000lines).
- Linear label ids.
- Human lead assignee id when a matching Linear identity is known.
- Existing project id when updating.
- F2-002 color helper output proving the selected project color is available,
  or a human-supplied verified equivalent.
- The project icon color: the hex for `project-color` from
  `SYMPHONY_PROJECT_COLOR_HEX` (`$SYMPHONY_TOOLING_ROOT/scripts/symphony/project-colors.ts`), passed as
  the project `color` so the Linear project icon matches its lane.

For a new project, use `projectCreate`, then `issueCreate` for the three planning
seeds and any explicitly requested additions, then the two standard seed blocker
relations. Read back the setup, then use `issueUpdate` to activate the ready
frontier unless the human requested a hold. For an existing project, use
`projectUpdate` only for requested metadata changes, and do not replay the
new-project fixture. Reuse existing
seeds; create a specific additional seed only when the human requests it. For
an explicit ticket move, resolve and read the existing ticket and target project,
then use `issueUpdate` with that ticket's id and the target `projectId`. Preserve
other fields unless the human also requests changes to them.

If any lookup, label application, color availability check, assignee mapping,
mutation, or relation creation fails, stop and record the exact failure in the
workpad. Do not continue with partial starter-ticket creation unless the human
chooses the reduced write set in a new request after seeing the failure.

## Fail-Closed Cases

Stop before live writes when any of these are true:

- A required source document is unavailable.
- The `DEMO` Linear team cannot be resolved.
- Required Linear labels are missing or cannot be applied.
- Required GitHub PR labels are missing or cannot be applied.
- The project color helper reports no available color, an unknown color, a
  duplicate active color, or missing project metadata.
- The project appears to need more than one color.
- The human lead cannot be mapped and assignment is required by the project
  contract.
- `linear_graphql` is unavailable.
- The human request is preview-only or does not ask to create or update a
  project, starter tickets, or project shape.

## Sample-factory Payload Fixture

This fixture documents the default three-seed payload shape using sample-factory
project content. The historical source supplies context, not authorization for
additional setup tickets or replaying creation against an existing project.

This is an inline synthetic example. Supply your own project plan; no historical
project document is required.

Project metadata:

```yaml
project-code: sample-factory
project-color: blue
base-branch: main
human-lead: Example Lead
github-pr-labels:
  - blue
  - symphony
linear-issue-labels:
  - blue
linear-team: DEMO
```

Starter ticket payloads:

| Title                              | Initial status                                                   | Labels | Template                     | Default blocker relation           |
| ---------------------------------- | ---------------------------------------------------------------- | ------ | ---------------------------- | ---------------------------------- |
| `Create requirements & design doc` | `Backlog`, then `Active` after setup verification unless held    | `blue` | `requirements-and-design.md` | none                               |
| `Plan project - seed ticket`       | `Backlog`, then `Active` after relation verification unless held | `blue` | `plan-project.md`            | blocked by requirements-and-design |
| `Trigger fan out`                  | `Backlog`, then `Active` after relation verification unless held | `blue` | `trigger-fan-out.md`         | blocked by plan-project            |

## Write-Capable Payload Fixture

When the human asks for live creation of a new project and all prerequisites
resolve, send exactly these three seed issue payloads and two seed relations
through `linear_graphql`, unless specific additions were explicitly requested.
Resolve concrete ids before sending:

```yaml
project_mutation:
  operation: projectCreate
  input:
    teamIds:
      - "<demo-team-id>"
    name: "<project-name>"
    description: "<rendered templates/project-description.md>"
    color: "<hex for project-color from SYMPHONY_PROJECT_COLOR_HEX in $SYMPHONY_TOOLING_ROOT/scripts/symphony/project-colors.ts>"

starter_issue_mutations:
  - operation: issueCreate
    key: requirements-and-design
    input:
      teamId: "<demo-team-id>"
      projectId: "<project-id>"
      title: "Create requirements & design doc"
      description: "<rendered templates/tickets/requirements-and-design.md>"
      stateId: "<backlog-state-id>"
      labelIds:
        - "<blue-label-id>"
      assigneeId: "<human-lead-linear-user-id when resolved>"
  - operation: issueCreate
    key: plan-project
    input:
      teamId: "<demo-team-id>"
      projectId: "<project-id>"
      title: "Plan project - seed ticket"
      description: "<rendered templates/tickets/plan-project.md>"
      stateId: "<backlog-state-id>"
      labelIds:
        - "<blue-label-id>"
      assigneeId: "<human-lead-linear-user-id when resolved>"
  - operation: issueCreate
    key: trigger-fan-out
    input:
      teamId: "<demo-team-id>"
      projectId: "<project-id>"
      title: "Trigger fan out"
      description: "<rendered templates/tickets/trigger-fan-out.md>"
      stateId: "<backlog-state-id>"
      labelIds:
        - "<blue-label-id>"
      assigneeId: "<human-lead-linear-user-id when resolved>"

starter_relation_mutations:
  - operation: issueRelationCreate
    input:
      issueId: "<requirements-and-design-issue-id>"
      relatedIssueId: "<plan-project-issue-id>"
      type: blocks
  - operation: issueRelationCreate
    input:
      issueId: "<plan-project-issue-id>"
      relatedIssueId: "<trigger-fan-out-issue-id>"
      type: blocks
```

After both relations and the three seed states are verified, activate all three
seeds with `issueUpdate(input: {stateId: <active-state-id>})`,
unless the human explicitly requested a hold. Record and verify the resulting
state. A hold changes only activation, not creation of the blocker relations.

If the Linear schema in the current workspace requires `content` instead of
`description`, use the schema-supported field but keep the same rendered
Markdown body. If the relation mutation spelling differs in the current schema,
keep the same
requirements-and-design blocks plan and plan blocks fan-out semantics.

## Request Boundary Examples

These examples assume required inputs and write prerequisites are resolved.
They describe expected payloads, not instructions to alter the named projects
or tickets while maintaining this skill.

| Human request                                                                                                                                                                                                                            | Expected factory output                                                                                                                                                                                           |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| “Create Sample Tooling Export and its seed tickets. The brief includes constructing the final extraction artifact, Example Reviewer review, publishing the approved extraction, and removing private scaffolding after the public push.” | Project metadata/brief preserving all outcomes, exactly the three planning seeds, and two seed blocker relations. Defer wrap-up like DEMO-543–545 to Symphony's human-reviewed plan; no wrap-up tickets at setup. |
| “Create the project and get it going.”                                                                                                                                                                                                   | The same three seeds and two seed blockers, using the requested initial states; no extra tickets.                                                                                                                 |
| “The existing project's brief also includes deployment and cleanup. Make its tickets Active.”                                                                                                                                            | Apply the requested state changes to existing tickets. Preserve the brief; do not create deploy, cleanup, or other tickets.                                                                                       |
| “Move DEMO-534 into the not-yet-started SampleMetrics project.”                                                                                                                                                                          | Update only DEMO-534's project membership. No replacement issue, sibling moves, or inferred additions.                                                                                                            |

For the explicit DEMO-534 move request, the mutation is an update, with no
`issueCreate` or unrelated field changes:

```yaml
existing_ticket_move_mutation:
  operation: issueUpdate
  id: "<resolved-DEMO-534-issue-id>"
  input:
    projectId: "<resolved-SampleMetrics-project-id>"
```
