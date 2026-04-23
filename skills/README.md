# Official Skills

This directory contains the official `docpact` skills.

Official skills are workflow assets built around the `docpact` CLI. They help agents and repository maintainers adopt, audit, and repair governance workflows more consistently, but they do not replace the engine.

## Boundaries

Official skills must follow these rules:

- Use existing `docpact` CLI commands and published semantics as the source of truth.
- Guide or automate workflow steps, but do not override pass/fail judgment.
- Prefer structured CLI outputs and report artifacts over parsing prose output.
- Apply the product modeling boundary before recommending config edits, source-doc maintenance, or derived-view treatment: deterministic facts belong in config, explanatory material belongs in source docs, and read-only summaries belong in derived views.
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
- Keep runtime guidance self-contained inside the skill directory. If a product concept also exists in normal product docs, copy or restate the minimum operational guidance inside the skill instead of requiring cross-directory reads.

## Official vs Custom Skills

Official skills in this directory are product-level, reusable workflows maintained with the public `docpact` repository.

Repository-specific or organization-specific skills should live outside the core product repository unless the underlying workflow is broadly reusable and aligns with published CLI semantics.

## Implemented Skills

Only two official skills are exposed as entrypoints:

- `docpact`: top-level direct workflow entrypoint for before-coding document discovery, after-coding lint/drill-down/review-mark flows, one-finding repair, ongoing freshness checks, and compact read-only summaries when `render` is the better fit than full route output.
- `docpact-governance`: top-level governance-maintainer entrypoint for onboarding, rule design, coverage backfill, routing configuration, rule audit, freshness-driven maintenance, and CI integration.

The detailed workflows now live as internal references under those two entrypoints rather than as separately exposed skills.

Internal direct-workflow references under `docpact/` include:

- failure repair

Internal maintainer-workflow references under `docpact-governance/` include:

- repository onboarding
- rule authoring
- coverage backfill
- routing configuration
- rule audit
- documentation maintenance
- CI integration

## Authoring Guidance

When adding a new official skill:

1. Start with the smallest directory that can express the workflow cleanly.
2. Keep the top-level `SKILL.md` focused on entrypoint routing, boundaries, and when to consult internal references.
3. Put detailed workflow breakdowns, examples, rubrics, and templates in that entrypoint skill's own `references/` or `assets/`.
4. Validate that the skill hands back to deterministic CLI commands rather than introducing hidden logic.
