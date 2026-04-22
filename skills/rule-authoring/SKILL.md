---
name: rule-authoring
description: Draft or revise `docpact` governance rules without inventing new schema or enforcement semantics. Use when `coverage` or `uncovered-change` exposes a blind spot, when a new module needs first-pass governance, or when an existing rule is too broad, too narrow, or attached to the wrong required docs and must be reused, replaced, or extended within the current config model.
---

# Rule Authoring

Turn a governance need into the smallest correct rule change that the current engine can validate.

This skill is for drafting or revising rules, not for bypassing the rule system. Prefer structured CLI outputs, preserve the current schema, and always end by handing control back to `validate-config --strict`.

## Workflow

### 1. Define the authoring target

Start from one explicit target:

- an uncovered path or hotspot from `docpact coverage`
- an `uncovered-change` finding discovered through `lint`
- a new module or path family that now needs governed docs
- an existing rule that is clearly too broad, too narrow, or mapped to the wrong docs

Collect these inputs before drafting anything:

- target paths or modules
- candidate governed documents
- desired governance strength
- current config shape (`repo` or `workspace`)

If the config shape or inheritance state is unclear, run:

```bash
docpact doctor --root <repo> --format json
```

Do not jump to rule authoring until you know whether you are editing:

- a plain `layout: repo` config
- a workspace root config
- a child repo config that inherits a workspace profile

### 2. Inspect the current rule graph and gap

Use structured inspection before proposing any new rule.

Always run:

```bash
docpact coverage --root <repo> --format json
docpact list-rules --root <repo> --format json
```

Use `coverage` to confirm whether the target path is:

- truly uncovered
- intentionally excluded
- already governed, but maybe by a rule with the wrong required docs

Use `list-rules` to inspect:

- nearby trigger patterns
- existing required docs that can be reused
- current `rule_source` and config provenance

Optionally run:

```bash
docpact route --root <repo> --paths <csv> --format json --detail full
```

Use `route` only to inspect likely document reuse. It does not replace `coverage` as the source of truth for whether a rule is missing.

Read:

- [references/rule-design-checklist.md](./references/rule-design-checklist.md)
- [references/trigger-required-doc-principles.md](./references/trigger-required-doc-principles.md)

### 3. Decide the correct authoring action

Choose one of these outcomes before drafting YAML:

1. **No new rule needed**
   - The target path is already governed by an existing rule.
   - The real problem is document maintenance, review evidence, or a validation gap.
2. **Modify an existing rule**
   - The target belongs to the same governance contract, but triggers or required docs are wrong.
3. **Add a new rule**
   - The target introduces a distinct governance contract that should not be folded into an existing rule.
4. **Adjust workspace inheritance**
   - The target should live in a shared workspace profile, or in a child repo override rather than a local top-level rule.

Use these rules when inheritance is involved:

- Shared defaults across child repos belong in `workspace.profiles.<name>.rules`.
- Child-specific additions belong in `overrides.rules.add`.
- Child-specific replacement of an inherited rule belongs in `overrides.rules.replace`.
- If inheritance is enabled in a child repo, top-level `rules`, `coverage`, `docInventory`, and `freshness` are invalid there.

Do not add a new rule when reusing or replacing an existing rule would keep the graph smaller and clearer.

Read [references/bad-rule-antipatterns.md](./references/bad-rule-antipatterns.md) before finalizing the action.

### 4. Draft only within current schema

Every drafted rule must stay inside the published config model.

The minimum rule shape is:

- `id`
- `scope`
- `repo`
- `triggers`
- `requiredDocs`
- `reason`

Current trigger fields:

- `path`
- `kind`

Current required doc fields:

- `path`
- `mode`

Supported `requiredDocs[].mode` values are:

- `review_or_update`
- `metadata_refresh_required`
- `body_update_required`
- `must_exist`

Current guardrails:

- `scope`, `repo`, and trigger `kind` are descriptive. Do not invent runtime meaning for them.
- Trigger paths are repo-relative globs.
- `requiredDocs` define review obligations after a trigger matched. They do not define coverage scope.
- Reuse an existing required doc when the governance contract is the same. Add a new required doc only when a distinct document really needs to carry that contract.

Start from [assets/rule-draft-template.yaml](./assets/rule-draft-template.yaml). Use the smallest section that matches the current config shape:

- repo rule
- workspace profile rule
- child `overrides.rules.add`
- child `overrides.rules.replace`

If the right answer is "no new rule needed," say that explicitly instead of forcing the template.

### 5. Validate and explain the draft

Every rule draft must end with strict validation.

Always run:

```bash
docpact validate-config --root <repo> --strict
```

Then re-check the surrounding graph:

```bash
docpact list-rules --root <repo> --format json
docpact coverage --root <repo> --format json
```

If the rule was designed to support a specific task path, optionally confirm the routing side:

```bash
docpact route --root <repo> --paths <csv> --format json --detail full
```

Use [assets/validation-steps-template.md](./assets/validation-steps-template.md) to format the validation handoff.

## Output Requirements

Always include:

- the chosen authoring action: `no-new-rule`, `modify-existing-rule`, `add-new-rule`, or `workspace-override-change`
- the exact config location to edit
- the proposed YAML snippet, or an explicit statement that no new rule is needed
- why existing rules were reused, replaced, or rejected
- which required docs were reused versus newly introduced
- the exact validation commands to run next

If the repository needs behavior that the current schema cannot express, stop and say so explicitly. Do not invent new fields or hidden semantics.
