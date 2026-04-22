---
name: rule-audit
description: Audit `docpact` rule graph quality using existing structured CLI outputs. Use when a repository already has a non-trivial rule graph, when `coverage` shows dead rules or suspicious hotspots, when inherited rules and overrides are becoming hard to explain, or when maintainers need a refactor plan that distinguishes duplicate, dead, over-broad, over-narrow, or poorly-bound rules without directly editing config.
---

# Rule Audit

Inspect rule graph quality without rewriting the rule graph inside the audit itself.

This skill is for diagnosis and refactor planning. It must stay grounded in `list-rules`, `coverage`, and `doctor`, and it must hand rule changes back to `rule-authoring` instead of directly modifying config.

## Workflow

### 1. Build a structured audit snapshot

Always start from structured CLI outputs.

Run:

```bash
docpact list-rules --root <repo> --format json
docpact coverage --root <repo> --format json
docpact doctor --root <repo> --format json
```

Treat these as the audit inputs:

- `list-rules.rules[]`
- `coverage.rule_coverage.dead_rules`
- `coverage.rule_coverage.uncovered_hotspots`
- `coverage.doc_reachability.orphan_docs`
- `doctor.summary`
- `doctor.configs`

Do not base a rule audit on prose output alone when the JSON reports are available.

### 2. Classify rule-quality issues

Use the current outputs to look for structural smells, not hypothetical metrics.

Primary audit classes:

- `dead-rule-candidate`
  - the rule appears in `coverage.rule_coverage.dead_rules`
- `duplicate-or-overlap-candidate`
  - two or more rules have heavily overlapping trigger families and near-identical required docs
- `over-broad-trigger-candidate`
  - one rule covers a large surface that likely hides multiple unrelated governance contracts
- `over-narrow-or-fragmented-candidate`
  - several tiny rules could probably collapse into one stable contract
- `required-doc-binding-candidate`
  - required docs look noisy, redundant, or mismatched to the actual contract surface
- `inheritance-provenance-candidate`
  - workspace profile rules and child overrides are hard to explain or appear unnecessarily divergent

Read [references/rule-audit-rubric.md](./references/rule-audit-rubric.md) before classifying findings.

### 3. Distinguish "coverage exists" from "rule quality is healthy"

Do not stop at the fact that a path is covered.

Coverage tells you:

- whether a path family is governed at all
- whether a rule is dead
- whether large uncovered hotspots still exist

Coverage does **not** prove:

- that the trigger is the right size
- that the rule is not duplicated elsewhere
- that the required docs are well chosen
- that workspace inheritance is clean

Use [references/rule-smell-patterns.md](./references/rule-smell-patterns.md) to avoid treating "covered" as "healthy."

### 4. Inspect overlaps and scope quality

Use `list-rules` to compare rules along these dimensions:

- trigger path families
- required doc reuse
- provenance kind
- workspace profile involvement
- rule count concentration in one area

Audit questions:

- Are two rules describing the same governance contract with slightly different trigger globs?
- Is one wide catch-all rule hiding a more useful decomposition?
- Are several tiny sibling rules really one domain split too aggressively?
- Are config docs or root docs required too often without a strong reason?
- Is a child override replacing inherited behavior when `add` would have been enough?

If the repository uses workspace inheritance, read [references/workspace-audit-considerations.md](./references/workspace-audit-considerations.md).

### 5. Produce refactor recommendations, not config edits

Each audit finding should end in one explicit recommendation:

- keep as-is
- delete rule
- merge rules
- split rule
- narrow trigger
- widen trigger carefully
- revise required docs
- move logic into workspace profile
- move logic into child override

Do not edit config directly inside this skill.

When a recommendation becomes one concrete rule change, hand it off to:

- `rule-authoring` for trigger / requiredDocs / provenance refactors
- `coverage-backfill` only when the real problem is still an uncovered backlog, not rule health

Read [references/rule-authoring-handoff.md](./references/rule-authoring-handoff.md) before writing the handoff.

### 6. End with an audit report and refactor queue

Use:

- [assets/audit-output-template.md](./assets/audit-output-template.md)
- [assets/refactor-recommendation-template.md](./assets/refactor-recommendation-template.md)

The final output should contain:

- structural findings
- why each finding matters
- which CLI signals support it
- whether the next step is deletion, merge, split, trigger adjustment, required-doc adjustment, or inheritance cleanup
- which findings should hand off to `rule-authoring`

## Output Requirements

Always include:

- the source reports used
- the audit class for each finding
- the supporting rule ids or coverage signals
- whether the next step is:
  - `no-change`
  - `rule-authoring`
  - `coverage-backfill`
  - `config cleanup`
- why coverage being present does or does not imply healthy rule design

Do not state that a rule is definitively wrong unless the current structured evidence supports it. When the evidence is ambiguous, classify it as a review candidate and explain what to inspect next.
