---
name: failure-repair
description: Diagnose and repair one docpact lint finding from a structured report. Use when `docpact lint` has already produced a JSON report, when you have a `diagnostic_id`, when you need to decide whether to update docs, refresh review evidence with `review mark`, adjust config or rules, or escalate to baseline or a narrowly-scoped waiver instead of treating a finding as a normal document repair.
---

# Failure Repair

Repair one `docpact` finding without redefining the engine's judgment.

Use this skill only after you already have a structured lint report and a target `diagnostic_id`. Prefer `diagnostics show` and JSON report fields over terminal text. The goal is to select the correct repair path, not to debate whether the finding exists.

## Workflow

### 1. Load the exact finding

Start from one explicit report artifact and one explicit diagnostic id.

Use:

```bash
docpact diagnostics show --report <report.json> --id <diagnostic_id> --format json
```

Treat these fields as the primary input:

- `finding_state`
- `problem_type`
- `path`
- `rule_id`
- `required_mode`
- `failure_reason`
- `suggested_action`
- `rule_source`
- `trigger_paths`

If `finding_state` is not `active`, do not treat the problem as a normal repair candidate until you explain why it is already suppressed or waived.

Read [references/finding-repair-matrix.md](./references/finding-repair-matrix.md) for the decision table.

### 2. Classify the repair path

Map the finding to one of these repair classes:

- document body update
- review evidence refresh
- metadata repair
- config or rule repair
- adoption-control escalation

The default mapping is:

- `problem_type=missing-review`
  - `failure_reason=doc_body_not_updated`
    - update the governed document body
  - `failure_reason=review_metadata_not_refreshed`
    - complete review, then use `review mark`
  - `failure_reason=required_doc_not_touched`
    - inspect `required_mode`
    - if the required doc should be substantively updated, edit it
    - if the doc only needs review evidence after a real review, use `review mark`
  - `failure_reason=required_doc_missing`
  - `failure_reason=required_doc_missing_after_change`
    - create or restore the governed doc first
- `problem_type=missing-metadata`
  - add the missing review metadata keys
  - if the review is genuinely complete and the file is Markdown or YAML, `review mark` is usually the cleanest repair
- `problem_type=uncovered-change`
  - do not use `review mark`
  - repair the rule graph instead by adding a rule or excluding the path from coverage

When the finding implies rule-graph work, use:

- `docpact list-rules --root <repo> --format json`
- `docpact validate-config --root <repo> --strict`

### 3. Check whether `review mark` is actually appropriate

Only recommend `review mark` when all of the following are true:

- the target path is a Markdown or YAML document
- the review has actually been completed
- the problem is about missing or stale review evidence rather than missing rule coverage

Prefer:

```bash
docpact review mark --root <repo> --report <report.json> --id <diagnostic_id>
```

That keeps navigation tied to the exact diagnostic instead of copying the path manually.

Read [references/review-mark-guidance.md](./references/review-mark-guidance.md) before recommending `review mark`.

### 4. Keep adoption controls explicit

Do not treat baseline or waiver as normal repair actions.

- Baseline is for historical adoption debt across an existing repository, not for a routine single-finding repair.
- Waiver is a narrow, temporary, explicitly owned exception and is not the default recommendation.

If a finding looks like historical debt rather than a fresh regression, say so explicitly and escalate to the adoption-control path instead of pretending the finding should be repaired inline.

Read [references/adoption-controls.md](./references/adoption-controls.md) before recommending either path.

### 5. End with executable next steps

Always end with a concrete repair sequence.

Typical sequences:

Document body repair:

```bash
docpact diagnostics show --report <report.json> --id <diagnostic_id> --format json
docpact lint --root <repo> <diff-source-args> --format json --output <report.json>
```

Review evidence refresh:

```bash
docpact diagnostics show --report <report.json> --id <diagnostic_id> --format json
docpact review mark --root <repo> --report <report.json> --id <diagnostic_id>
docpact lint --root <repo> <diff-source-args> --format json --output <report.json>
```

Rule/config repair:

```bash
docpact diagnostics show --report <report.json> --id <diagnostic_id> --format json
docpact list-rules --root <repo> --format json
docpact validate-config --root <repo> --strict
docpact lint --root <repo> <diff-source-args> --format json --output <report.json>
```

`<diff-source-args>` must be one of:

- `--staged`
- `--worktree`
- `--files <csv>`
- `--merge-base <ref>`
- `--base <sha> --head <sha>`

## Output Requirements

Always include:

- the target `diagnostic_id`
- whether the finding is `active`, `suppressed_by_baseline`, or `waived`
- the selected repair class
- why other repair classes were rejected
- the exact next commands to run

Use the templates in `assets/` instead of inventing a new repair report shape each time.
