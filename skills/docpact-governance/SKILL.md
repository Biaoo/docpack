---
name: docpact-governance
description: Route governance-maintainer work to the right official docpact skill without redefining CLI semantics. Use when the task is about onboarding a repository, designing or auditing rules, backfilling coverage, maintaining routing aliases, integrating CI, or remediating stale governed docs as governance maintenance rather than as a normal coding-task workflow.
---

# Docpact Governance

Use this skill as the maintainer-facing entrypoint for the `docpact` system itself.

This is not the direct product workflow. Do not use it for normal before-coding reading, after-coding lint validation, or one-off review marking. Use it when the task is about maintaining, extending, or auditing the repository's documentation-governance model.

Keep this skill thin:

- identify the maintainer task class
- gather the minimum structured evidence
- hand off to the right official skill
- fall back to CLI when no official skill fits

Do not duplicate child-skill procedures or invent new governance semantics here.

## Workflow

### 1. Classify the maintainer task first

Start by deciding which governance-maintainer problem you actually have.

Use these routing classes:

- `repository-onboarding`
  - first-time adoption
  - missing or incomplete `.docpact/config.yaml`
  - deciding between `layout: repo` and `layout: workspace`
- `rule-authoring`
  - adding, replacing, disabling, or refining rules
  - repairing uncovered areas by designing rule changes
- `coverage-backfill`
  - turning uncovered hotspots into grouped backlog work
  - deciding between `new-rule`, `adjust-existing-rule`, `candidate-exclude`, or `needs-more-context`
- `routing-configuration`
  - adding or repairing controlled `routing.intents`
  - maintaining workspace routing overrides
- `rule-audit`
  - reviewing rule-graph quality, dead rules, overlap, over-broad triggers, or weak required-doc bindings
- `ci-integration`
  - designing or repairing GitHub Actions integration around the official wrapper and current CLI semantics
- `documentation-maintenance`
  - stale-doc remediation
  - invalid review-reference cleanup
  - governance-aware document maintenance driven by `freshness`

If the work is already narrowed to one explicit lint finding from a structured report, hand off to `failure-repair` instead of staying in broad maintainer routing.

Read [references/maintainer-routing-map.md](./references/maintainer-routing-map.md) before choosing a child skill.

### 2. Gather only the minimum structured evidence

Do not load every CLI surface by default. Pick the smallest commands that disambiguate the task.

Common starting commands:

```bash
docpact doctor --root <repo> --format json
docpact list-rules --root <repo> --format json
docpact coverage --root <repo> --format json
docpact freshness --root <repo> --format json
docpact render --root <repo> --view catalog-summary --format json
docpact render --root <repo> --view ownership-summary --format json
docpact validate-config --root <repo> --strict
```

Typical usage:

- onboarding
  - `doctor`
  - `validate-config --strict`
  - optionally `list-rules` and `coverage`
- rule authoring or rule audit
  - `list-rules`
  - `coverage`
  - `validate-config --strict`
- routing configuration
  - `route`
  - optionally `render --view navigation-summary` when a shorter derived navigation snapshot is enough
  - `validate-config --strict`
  - optionally `doctor` in workspace layouts
- ownership or catalog context review
  - `render --view catalog-summary`
  - `render --view ownership-summary`
  - optionally `doctor` when tracked-path overlap/conflict surfacing matters
- CI integration
  - inspect existing workflow files
  - align them with the official action and documented trigger patterns
- documentation maintenance
  - `freshness`
  - optionally `list-rules`, `coverage`, `route`, or `render --view ownership-summary`

If the problem cannot yet be classified from structured evidence, say that explicitly instead of guessing.

### 3. Hand off to exactly one primary child skill

Once classified, move to one primary skill and stay there until the task changes materially.

Primary handoffs:

- onboarding -> `repository-onboarding`
- rule design or rule change -> `rule-authoring`
- grouped coverage gap planning -> `coverage-backfill`
- routing alias maintenance -> `routing-configuration`
- rule graph quality review -> `rule-audit`
- workflow and GitHub Actions design -> `ci-integration`
- stale-doc remediation and invalid review-reference maintenance -> `documentation-maintenance`

Use `failure-repair` only as a shared support path when a maintainer task collapses into one explicit finding with a `diagnostic_id`.

Read [references/skill-boundary-guide.md](./references/skill-boundary-guide.md) before selecting a fallback or secondary handoff.

### 4. Keep direct workflow and maintainer workflow separate

Do not use this skill for:

- "what should I read before coding?" -> use `docpact`
- "what docs should this change have touched?" -> use `docpact`
- "show me this one finding" -> use `docpact` or `failure-repair`
- "record completed review evidence" -> use `docpact`

This skill exists only for maintaining the governance system.

### 5. Fall back to CLI instead of inventing missing workflow

If no official maintainer skill fits, do not fabricate a pseudo-skill.

Use the closest deterministic CLI path and state the gap clearly. Typical fallback commands are:

```bash
docpact doctor --root <repo> --format json
docpact list-rules --root <repo> --format json
docpact coverage --root <repo> --format json
docpact freshness --root <repo> --format json
docpact render --root <repo> --view catalog-summary --format json
docpact render --root <repo> --view ownership-summary --format json
docpact validate-config --root <repo> --strict
```

If the missing workflow looks product-worthy, explicitly call it a product gap or future skill candidate.

Use:

- [assets/governance-triage-template.md](./assets/governance-triage-template.md)
- [assets/maintainer-routing-examples.md](./assets/maintainer-routing-examples.md)

## Output Requirements

Always include:

- the maintainer task class you identified
- the primary official skill to use next
- why that skill is the best fit
- the minimum required structured inputs
- whether `failure-repair` is needed for one narrowed finding
- whether CLI fallback is required because no official skill fits

Do not rewrite the child skill's full procedure in your answer. Route cleanly, then stop.
