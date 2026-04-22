---
name: repository-onboarding
description: Plan and draft first-pass docpact adoption for an existing repository. Use when a repository is adding docpact for the first time, when `.docpact/config.yaml` is missing or incomplete, when you need to choose between repo and workspace layouts, or when you need a staged onboarding plan built around `doctor`, `list-rules`, `coverage`, `validate-config --strict`, and baseline-first adoption controls.
---

# Repository Onboarding

Build a first-pass `docpact` adoption plan without inventing new product semantics.

Treat this skill as an onboarding workflow around the existing CLI. Use structured outputs and report artifacts wherever possible. Do not decide pass/fail yourself, and do not treat waiver as the default way to get started.

## Workflow

### 1. Build a current-state summary

Start with the repository as it exists today.

- Run `docpact doctor --root <repo> --format json` to summarize config presence, rule count, coverage, doc inventory, and freshness setup.
- If `.docpact/config.yaml` exists, run `docpact validate-config --root <repo> --strict`.
- If config validation fails, surface that first. Do not keep planning around an invalid config as if it were authoritative.

Use the doctor summary to classify the repository into one of these states:

- no config yet
- config exists but rule graph is empty or incomplete
- config exists and rules exist, but coverage or governance shape is still thin

Read [references/onboarding-checklist.md](./references/onboarding-checklist.md) for the standard summary sections and output expectations.

### 2. Decide the configuration shape

Choose the layout before drafting fixes.

- If the repository is a single governed unit, prefer `layout: repo`.
- If the repository clearly contains multiple governed child repositories with shared governance defaults, consider `layout: workspace`.
- Do not recommend workspace inheritance unless the repository genuinely needs shared profiles and child overrides.

Read [references/layout-selection.md](./references/layout-selection.md) when layout choice is not obvious.

Start with the local starter templates in `assets/`:

- [assets/repo-config-starter.yaml](./assets/repo-config-starter.yaml)
- [assets/workspace-config-starter.yaml](./assets/workspace-config-starter.yaml)
- [assets/workspace-child-config-starter.yaml](./assets/workspace-child-config-starter.yaml)

If you need richer examples after that, compare them with the public product examples:

- `docpact/examples/repo-config.yaml`
- `docpact/examples/workspace-config.yaml`
- `docpact/examples/workspace-child-config.yaml`

Do not create new config fields or undocumented variants.

### 3. Inspect the existing governance surface

Use structured inspection before proposing a plan.

- Run `docpact list-rules --root <repo> --format json` to inspect the effective rule graph.
- Run `docpact coverage --root <repo> --format json` to inspect uncovered hotspots and document reachability gaps.
- Optionally run `docpact freshness --root <repo> --format json` if the repository already has governed docs with review evidence and freshness is relevant to onboarding quality.

From these commands, separate findings into:

- config problems
- missing or weak rules
- uncovered areas that need backfill
- document/governance hygiene gaps
- optional follow-on CI work

### 4. Produce a staged onboarding plan

Output a plan that clearly separates:

- what to configure now
- what to defer
- what to validate next

Always distinguish:

- changes to configuration
- changes to rules
- changes to documents
- changes to CI or operational workflow

When historical lint debt is expected, recommend a baseline-first adoption sequence:

1. draft or repair config
2. validate with `validate-config --strict`
3. run `lint` to observe current debt
4. create baseline if needed
5. start blocking only new active findings

Read [references/adoption-controls.md](./references/adoption-controls.md) before recommending baseline or waiver.

Use these templates when formatting output:

- [assets/onboarding-summary-template.md](./assets/onboarding-summary-template.md)
- [assets/adoption-plan-template.md](./assets/adoption-plan-template.md)

### 5. Keep the boundaries explicit

Do not do any of the following:

- Do not invent config fields.
- Do not treat waiver as the default onboarding path.
- Do not suppress findings yourself.
- Do not say a repository is "ready" unless the recommendation can be checked by CLI commands.

If the repository needs product behavior that does not exist, say so explicitly and frame it as a product or roadmap gap.

## Command Pattern

Use this sequence as the default starting point:

```bash
docpact doctor --root <repo> --format json
docpact validate-config --root <repo> --strict
docpact list-rules --root <repo> --format json
docpact coverage --root <repo> --format json
```

If no config exists yet, say that clearly and draft the smallest valid first-pass config before returning to `validate-config --strict`.

If you need to recommend staged adoption after observing current debt, the follow-up path is:

```bash
docpact lint --root <repo> <diff-source-args> --format json --output .docpact/runs/onboarding.json
docpact baseline create --report .docpact/runs/onboarding.json --output .docpact/baseline.json
```

`<diff-source-args>` must be one of:

- `--staged`
- `--worktree`
- `--files <csv>`
- `--merge-base <ref>`
- `--base <sha> --head <sha>`

Do not recommend `waiver add` unless the user is dealing with a narrow, temporary, explicitly owned exception.

## Output Requirements

Your output should always include:

- current repository state
- recommended layout (`repo` or `workspace`) with a reason
- immediate config or rule changes
- whether baseline is recommended
- whether waiver is not recommended at this stage
- concrete next validation commands

Prefer concise structured sections over long prose. Reuse the templates in `assets/` instead of inventing a new report shape each time.
