# CLI Handoff Guide

Use this guide to keep the direct workflow skill aligned with the existing CLI.

## Route

Use:

```bash
docpact route --root <repo> --paths <csv> --format json
```

Or:

```bash
docpact route --root <repo> --module <prefix> --format json
docpact route --root <repo> --intent <alias> --format json
```

Notes:

- `route` is advisory only
- `--intent` only accepts configured aliases
- `--detail full` is optional

## Lint

Use:

```bash
docpact lint --root <repo> <diff-source-args> --format json --output .docpact/runs/latest.json
```

Required diff source:

- `--files <csv>`
- `--staged`
- `--worktree`
- `--merge-base <ref>`
- `--base <sha> --head <sha>`

## Diagnostics Show

Use:

```bash
docpact diagnostics show --report .docpact/runs/latest.json --id <diagnostic_id> --format json
```

Use only when one saved lint report already exists.

## Review Mark

Use:

```bash
docpact review mark --root <repo> --report .docpact/runs/latest.json --id <diagnostic_id>
```

Or:

```bash
docpact review mark --root <repo> --path <doc-path>
```

Notes:

- only after a review is genuinely complete
- path mode and report/id mode cannot be mixed

## Freshness

Use:

```bash
docpact freshness --root <repo> --format json
```

Use when the question is about whether governed documents still look trustworthy, not whether one specific diff passed lint.
