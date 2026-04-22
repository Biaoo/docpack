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

- `repository-onboarding`: guide first-time repository adoption around `doctor`, `list-rules`, `coverage`, `validate-config --strict`, and baseline-first staged rollout.
- `failure-repair`: diagnose one lint finding from a structured report and choose the correct repair path across doc edits, review evidence refresh, config repair, or adoption-control escalation.

## Authoring Guidance

When adding a new official skill:

1. Start with the smallest directory that can express the workflow cleanly.
2. Keep `SKILL.md` focused on workflow, boundaries, and when to consult additional references.
3. Put large examples, rubrics, and templates in that skill's own `references/` or `assets/`.
4. Validate that the skill hands back to deterministic CLI commands rather than introducing hidden logic.
