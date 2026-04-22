---
name: docpact
description: Use `docpact` as the direct workflow entrypoint for agentic documentation governance. Use when an agent needs to decide what to read before coding, what docs should have been reviewed or updated after coding, how to drill into one lint finding, how to record completed review evidence, or how to check whether governed documents have gone stale.
---

# Docpact

Use `docpact` directly for the product's three main workflow phases:

- before coding
- after coding
- ongoing trust checks

This skill is the product-facing entrypoint. It is not the governance-maintainer router, and it is not a replacement for the CLI. Its job is to choose the correct existing `docpact` command first, then escalate to a maintainer skill only when the problem is no longer a normal task workflow.

## Workflow Map

### 1. Before coding: decide what to read

Use `route` when the agent is about to make a change and needs the minimum relevant reading set.

Choose the smallest stable input:

- `--paths <csv>` for explicit target files or path globs
- `--module <csv>` for a repo-relative module prefix
- `--intent <csv>` only when the repo already defines controlled aliases in `routing.intents`

Default commands:

```bash
docpact route --root <repo> --paths <csv> --format json
docpact route --root <repo> --module <prefix> --format json
docpact route --root <repo> --intent <alias> --format json
```

Use `--detail full` only when you need the score breakdown, matched triggers, or provenance. Start with compact JSON for agent efficiency.

If the repository does not have useful routing behavior yet, do not invent new route semantics here. Hand off to:

- `routing-configuration` when the task is "define or fix controlled intent aliases"
- `rule-authoring` when the route result is weak because the rule graph is weak

### 2. After coding: determine what should have changed

Use `lint` when the agent has made or plans to validate a concrete change.

`lint` always needs one explicit diff source:

- `--files <csv>`
- `--staged`
- `--worktree`
- `--merge-base <ref>`
- `--base <sha> --head <sha>`

Default command pattern:

```bash
docpact lint --root <repo> <diff-source-args> --format json --output .docpact/runs/latest.json
```

Use a saved report path whenever the result may need drill-down.

If `lint` returns findings and you need to inspect one exact result, move immediately to:

```bash
docpact diagnostics show --report .docpact/runs/latest.json --id <diagnostic_id> --format json
```

If the task becomes "repair this one finding," hand off to `failure-repair`.

### 3. After coding: record completed review evidence

Use `review mark` only after a review has actually been completed.

Prefer the diagnostics-driven form when the review comes from one explicit lint finding:

```bash
docpact review mark --root <repo> --report .docpact/runs/latest.json --id <diagnostic_id>
```

Use explicit path mode only when you genuinely know the document path without going through diagnostics:

```bash
docpact review mark --root <repo> --path docs/api.md
```

Do not use `review mark` for:

- uncovered-change
- missing rules
- speculative review completion

After recording review evidence, rerun `lint` with the same diff source.

### 4. Ongoing: check whether governed docs have gone stale

Use `freshness` when the task is not about one immediate code diff, but about whether governed documents still look trustworthy.

Default command:

```bash
docpact freshness --root <repo> --format json
```

Use this when:

- deciding whether docs are safe to trust before a broad task
- triaging stale governance debt
- checking whether review references are invalid

If the freshness result leads to config or rule maintenance work, that is no longer a direct workflow problem. Hand off to the appropriate maintainer skill instead of forcing the direct workflow path.

## Handoff Rules

Stay in this direct workflow skill when the question is:

- what should I read first?
- what docs should this change have touched?
- what does this one finding mean?
- how do I record completed review evidence?
- are these governed docs stale?

Hand off to maintainer-oriented skills when the question becomes:

- how do we onboard this repository? -> `repository-onboarding`
- how should we repair one finding? -> `failure-repair`
- how should we design or change rules? -> `rule-authoring`
- how should we turn uncovered hotspots into backlog? -> `coverage-backfill`
- how should we maintain routing aliases? -> `routing-configuration`
- how healthy is the current rule graph? -> `rule-audit`

Until a dedicated governance orchestrator exists, route directly to those existing skills rather than trying to keep all maintainer logic in this skill.

Read:

- [references/product-workflow-routing-map.md](./references/product-workflow-routing-map.md)
- [references/cli-handoff-guide.md](./references/cli-handoff-guide.md)

## Output Requirements

Always include:

- which workflow phase this task belongs to: `before-coding`, `after-coding`, or `ongoing`
- the first CLI command to run
- the minimum required inputs for that command
- whether a saved report artifact is needed
- whether the task should remain in direct workflow or hand off to a maintainer skill

Use the templates in `assets/` instead of inventing a new output structure each time.
