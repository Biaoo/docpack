# docpact

`docpact` is a standalone Rust CLI for deterministic, diff-driven documentation governance.

It helps teams and agents answer three practical questions:

- before coding: what documents should I read first?
- after coding: what documentation should this change have reviewed or updated?
- ongoing: which governed documents have gone stale?

`docpact` stays deterministic. It does not replace governance decisions with AI inference, and it does not hide state in background services or opaque caches.

## Install

Install from crates.io:

```bash
cargo install docpact
```

Run from source:

```bash
cargo run -- <command>
```

Install from a local checkout:

```bash
cargo install --path .
```

## Quick Start

1. Start from one of the bundled config examples:
   - [examples/repo-config.yaml](./examples/repo-config.yaml)
   - [examples/workspace-config.yaml](./examples/workspace-config.yaml)
   - [examples/workspace-child-config.yaml](./examples/workspace-child-config.yaml)
2. Copy the right shape into the target repository as `.docpact/config.yaml`.
3. Validate the config.
4. Run `lint` against an explicit diff source.

Example:

```bash
docpact validate-config --root /path/to/repo
docpact validate-config --root /path/to/repo --strict
docpact lint --root /path/to/repo --files src/api/client.ts,README.md --format text
```

`lint` always needs one explicit diff source. Use one of:

- `--files <csv>`
- `--staged`
- `--worktree`
- `--merge-base <ref>`
- `--base <sha> --head <sha>`

## Core Commands

### Validate configuration

```bash
docpact validate-config --root /path/to/repo
docpact validate-config --root /path/to/repo --strict
```

### Check a concrete change

```bash
docpact lint --root /path/to/repo --files src/api/client.ts,README.md --format json --output .docpact/runs/latest.json
```

### Drill into one finding

```bash
docpact diagnostics show --report .docpact/runs/latest.json --id d001 --format json
```

### Record completed review evidence

```bash
docpact review mark --root /path/to/repo --path docs/api.md
```

or, when coming from one explicit lint finding:

```bash
docpact review mark --root /path/to/repo --report .docpact/runs/latest.json --id d001
```

### Audit governance coverage

```bash
docpact coverage --root /path/to/repo --format json
```

### Audit document freshness

```bash
docpact freshness --root /path/to/repo --format json
```

### Route reading before coding

```bash
docpact route --root /path/to/repo --paths src/payments/** --format json
docpact route --root /path/to/repo --module src/payments --format text
docpact route --root /path/to/repo --intent payments --format json
```

## Adoption Controls

`docpact` supports explicit adoption controls for repositories that cannot enforce all existing debt immediately.

Create and apply a baseline:

```bash
docpact baseline create --report .docpact/runs/latest.json --output .docpact/baseline.json
docpact lint --root /path/to/repo --files src/api/client.ts,README.md --baseline .docpact/baseline.json
```

Add a waiver for one explicit finding:

```bash
docpact waiver add \
  --report .docpact/runs/latest.json \
  --id d001 \
  --reason "temporary exception during migration" \
  --owner "team-docs" \
  --expires-at 2026-05-31 \
  --output .docpact/waivers.yaml
```

Then apply it during lint:

```bash
docpact lint --root /path/to/repo --files src/api/client.ts,README.md --waivers .docpact/waivers.yaml
```

Use waivers sparingly. They are temporary, explicit exceptions, not a default suppression path.

## GitHub Actions

This repository ships a thin official GitHub Action wrapper in [action.yml](./action.yml).

Typical usage:

```yaml
- uses: <org>/docpact@v1
  with:
    version: 0.1.0
    args: >
      lint
      --root .
      --base ${{ github.event.pull_request.base.sha }}
      --head ${{ github.sha }}
      --mode enforce
```

Reference workflows:

- [examples/github-actions/pr-lint.yml](./examples/github-actions/pr-lint.yml)
- [examples/github-actions/pr-lint-with-adoption-controls.yml](./examples/github-actions/pr-lint-with-adoption-controls.yml)
- [examples/github-actions/coverage-audit.yml](./examples/github-actions/coverage-audit.yml)
- [examples/github-actions/freshness-audit.yml](./examples/github-actions/freshness-audit.yml)

## Skills

This repository also ships official workflow skills under [skills/](./skills):

- [skills/README.md](./skills/README.md)
- [skills/docpact/SKILL.md](./skills/docpact/SKILL.md): direct workflow entrypoint
- [skills/docpact-governance/SKILL.md](./skills/docpact-governance/SKILL.md): governance-maintainer entrypoint

## Current Capabilities

Current `docpact` capabilities include:

- repo and workspace config loading
- explicit workspace profile inheritance and child overrides
- deterministic trigger-to-required-doc matching
- metadata checks on governed Markdown and YAML docs
- diff coverage and repository coverage audit
- repository freshness audit
- deterministic routing with paths, module scope, and controlled intents
- report-backed diagnostics drill-down
- explicit review-evidence recording
- baseline and waiver lifecycle
- list-rules and doctor inspection commands
- text, JSON, and SARIF reporting
- official GitHub Action wrapper
- official skills for direct workflow and governance maintenance

Still deferred:

- symbol-level drift checks
- executable documentation hooks
- AI-assisted semantic review
- documentation generation from code
