# Symphony client

Repository: "jeremycarroll/venn-search-rs". Default branch: "main".
Linear team: "100". Each issue supplies its own project;
this repository can serve multiple projects.

Read the target's README, AGENTS.md and CLAUDE.md when present. Use the build and
test argument arrays in `.symphony.cfg.json`; preserve existing setup, lint and
application CI. Omitted `ci.mode` means native validation. Run available local
checks, use Docker for environment gaps, and require CI on the published head.
The initial `ci.requiredChecks: []` is **unconfigured**, never passing evidence.
Record the target's observed check names, workflow paths and producing App IDs
before activation. Keep the Cadence advisory check outside required checks.

The optional command CI and native CI wakeup callers use reviewed shared code.
Omit `symphony-client-ci.yml` when existing application CI covers the commands;
otherwise keep its command arrays aligned with config and provide any required
application setup. It runs the exact PR head with no named secrets. Wakeups pass
only `CADENCE_LINEAR_API_TOKEN` and use trusted source helpers, reading config
from this repository's default branch. Record actual check/workflow/App evidence
before activation; the generated empty required-check list is not a CI contract.

Select `ci.mode` during onboarding without changing the eight Copier answers:

- `native` (or omission): use installed tools. Passing local checks skip Docker;
  missing tools/services may use the client's documented container as fallback.
- `docker`: supply the application's Dockerfile and explicit setup/build/test
  arrays to build and run it. The field does not wrap commands automatically.
  Mount only the issue workspace, use its UID/GID, remove task containers and
  record image digest/results. No generic Dockerfile is supplied.
- `remote`: run useful available checks and record missing tools, then publish
  for mandatory current-head GitHub CI. Fix known failures; unrun checks are not
  passes. No host toolchain installation or Docker setup is required.

GitHub CI needs its own working toolchain/commands in every mode. Required CI
pending or missing uses Unhappy with `wake:15m`; failures return nonterminal
tickets to Active; passing checks return them to Inactive for review. Preserve
terminal states and current-head guards. Live mode/bridge proof remains CT-A.

This initial publication includes CI, wakeups and review ingress. Review, handoff
and cleanup callers are deferred; the ingress alone does not run a review.
Use the available workflows and report missing functionality when it is needed.

## Client session skills

Load these skills from this generated client in the human-operated session:

- [Project factory](.agents/skills/symphony-project-factory/SKILL.md), retaining
  its `templates/` directory; invoke only when a human requests project setup.
- [Linear GraphQL](.agents/skills/linear-graphql/SKILL.md), retaining
  `agents/openai.yaml` and `scripts/linear-graphql.mjs`. Factory operations require
  the injected `linear_graphql` tool; the fallback script does not replace it.
- [Replan](scripts/symphony/runtime-bundle/skills/symphony-replan/SKILL.md), loaded
  explicitly from this nested location with the client
  [replanning guide](docs/engineering/symphony/replanning.md) and factory templates.
- [Karpathy guidelines](.agents/skills/karpathy-guidelines/SKILL.md), with
  [examples](.agents/skills/karpathy-guidelines/EXAMPLES.md), for coding work.

Keep a separate [reviewed tooling checkout](https://github.com/1000lines/symphony-example/tree/fd383f5760a2ba62ea6f6295bd6dd21cc0cb9e9e)
with its own locked dependencies and CT-C mode-aware config reader. Verify the
hosted reader supports `ci.mode` before enabling an explicit mode on that host. `SYMPHONY_TOOLING_ROOT` names that checkout;
external workflow guidance, color helper, DAG tooling and proof/review guides
resolve there. Client plans and skill-relative resources resolve in this client.
Record both checkout refs/locations and the loaded skill/resource paths. The
client repository and its Linear projects remain the operation's targets.

[CT-O's setup and invocation procedure](https://linear.app/1000lines/issue/100-54)
will verify initial, repeated and additional-project loading, including replan's
nested path. Copying these files does not install or run the skills. Keep the
project factory out of the unattended hosted worker profile.

## Review and App setup

Author App: "1000lines-symphony". Reviewer App: "1000lines-cadence".
Reviewer choice: "claude"; see [review context](.github/symphony/REVIEW.md).
Provision credentials separately from answers and commands. The selected reviewer
requires its matching API key even if the other provider's key is present.

Public MVP forks use the accepted existing Cadence App; installation does not
provide the named Actions secrets. Direct/private targets use the inert
[App manifest](.github/symphony/cadence-app-manifest.json) in the operator's
[GitHub manifest registration flow](https://docs.github.com/en/apps/sharing-github-apps/registering-a-github-app-from-a-manifest).
It requests repository read access and PR/issue/check write access, with no
webhook receiver or subscribed events; native GitHub Actions provide delivery.
The owner verifies the created App's grants/IDs and provisions secrets through
CT-O. Rendering is not App registration or credential provisioning.

## Source and license

Primary source: [Orchestra-Bio/symphony-example](https://github.com/Orchestra-Bio/symphony-example).
Orchestra Bio symphony-example / Copyright 2026 Orchestra Bio, Inc.
Development source and [usage/provenance](https://github.com/1000lines/symphony-example/tree/baa646a45721713231a1801c2271f524ccfc37ce/templates/symphony-client):
1000lines/symphony-example, [Apache-2.0](https://github.com/1000lines/symphony-example/blob/d5e9692b84c3f338014b964fd9713143fb723b55/LICENSE).
The copied Karpathy skill retains its MIT declaration and original attribution.
Preserve the target's own LICENSE, NOTICE and unrelated files.

## Repository onboarding

Generated from `1000lines/symphony-client-template@58021a7` using Copier.
The existing application CI replaces the optional generated command caller.
Use Linear team `100`; each project or issue must name this repository.
Follow `CLAUDE.md` for the research and algorithm constraints. Existing CI checks all four NCOLORS variants.
