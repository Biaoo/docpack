# Official Skills

This directory contains the official `docpact` skills.

Official skills are workflow assets built around the `docpact` CLI. They help agents and repository maintainers adopt, audit, and repair governance workflows more consistently, but they do not replace the engine.

## Boundaries

Official skills must follow these rules:

- Use existing `docpact` CLI commands and published semantics as the source of truth.
- Guide or automate workflow steps, but do not override pass/fail judgment.
- Prefer structured CLI outputs and report artifacts over parsing prose output.
- Do not invent new config fields, new finding semantics, or new adoption-control behavior.
- Escalate product-level gaps explicitly instead of silently working around them inside a skill.

## Directory Contract

Each official skill lives in its own directory:

```text
skills/
├── README.md
└── <skill-name>/
    ├── SKILL.md
    ├── references/
    └── assets/
```

Rules:

- Each skill directory must use a stable kebab-case name.
- `SKILL.md` is required for every implemented skill.
- `references/` is optional and should contain material intended to be loaded into context as needed.
- `assets/` is optional and should contain output-side resources such as templates or examples that do not need to be loaded into context by default.
- Do not add a shared `_shared/` template or prompt directory under `skills/`.
- If a resource is truly product-wide, keep it in normal product docs and reference it explicitly from the skill that needs it.

## Official vs Custom Skills

Official skills in this directory are product-level, reusable workflows maintained with the public `docpact` repository.

Repository-specific or organization-specific skills should live outside the core product repository unless the underlying workflow is broadly reusable and aligns with published CLI semantics.

## Implemented Skills

### Direct Workflow Entrypoint

- `docpact`: top-level direct workflow entrypoint for before-coding document discovery, after-coding lint/drill-down/review-mark flows, ongoing freshness checks, and compact read-only summaries when `render` is the better fit than full route output.

### Shared Remediation

- `failure-repair`: diagnose one explicit lint finding from a structured report and choose the correct repair path across doc edits, review evidence refresh, config repair, or adoption-control escalation. This is a shared repair skill that can be entered from the direct workflow or from maintainer work when the problem narrows to one concrete finding.

### Governance Maintainer Skills

- `docpact-governance`: top-level governance-maintainer entrypoint that routes maintainer work to the correct official skill, keeps direct workflow separate, and falls back to deterministic CLI inspection such as `doctor`, `coverage`, or `render` when no official maintainer skill fits.
- `repository-onboarding`: guide first-time repository adoption around `doctor`, `list-rules`, `coverage`, `validate-config --strict`, and baseline-first staged rollout.
- `rule-authoring`: turn uncovered areas or governance requirements into the smallest correct rule draft, with explicit reuse/replace/add decisions and strict config validation handoff.
- `coverage-backfill`: turn coverage audit gaps into grouped, prioritized backfill tasks, with explicit handoff to `rule-authoring` for concrete rule drafts.
- `routing-configuration`: maintain controlled `routing.intents` aliases and workspace routing overrides without expanding route into free-text intent handling.
- `rule-audit`: inspect rule graph quality using `list-rules`, `coverage`, and `doctor`, then hand refactor work back to `rule-authoring` instead of editing rules inside the audit.
- `documentation-maintenance`: turn `freshness` signals into stale-doc remediation, review-evidence repair, structure cleanup, or governance escalation without weakening the existing rule graph.
- `ci-integration`: design, review, and repair GitHub Actions integration around the thin official wrapper and existing CLI semantics without inventing a CI-only parameter model.

## Authoring Guidance

When adding a new official skill:

1. Start with the smallest directory that can express the workflow cleanly.
2. Keep `SKILL.md` focused on workflow, boundaries, and when to consult additional references.
3. Put large examples, rubrics, and templates in that skill's own `references/` or `assets/`.
4. Validate that the skill hands back to deterministic CLI commands rather than introducing hidden logic.
