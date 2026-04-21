# sample-sdk

This package is the repo-local example used by `ai-doc-lint`.

## Code Areas

- `src/api/**`: user-facing SDK API paths
- `src/commands/**`: command entrypoints

## AI Docs

- `AGENTS.md`
- `.ai-doc-lint/config.yaml`

Changing `src/api/**` or `src/commands/**` should require touching the appropriate docs according to `.ai-doc-lint/config.yaml`.
