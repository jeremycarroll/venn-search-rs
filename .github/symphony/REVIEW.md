# Repository review context

Review "jeremycarroll/venn-search-rs" against the issue's selected base (default
"main"). Read `SYMPHONY.md`, existing application
instructions, the Linear issue/project and current human PR feedback. Use the
target's `.symphony.cfg.json` and actual current-head CI evidence.

The explicit reviewer choice is "claude".

Provision `CADENCE_AI_REVIEW_ANTHROPIC_API_KEY` for this choice.

Both keys present still selects this reviewer. Missing/invalid selection or a
missing matching key must fail before provider execution, without fallback.
CT-L must wire this value as `cadence_reviewer` in the thin review caller after
the reviewed provider interface exists. This initial client has no review caller
and makes no live provider claim.

Keep reviewer implementation in the shared workflows. App/Linear credentials
and optional provider keys must be explicitly mapped at each review boundary.
The author and reviewer remain distinct; human acceptance owns merge and Done.
