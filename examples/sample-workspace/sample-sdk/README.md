# sample-sdk

This package is the repo-local example used by `docpact`.

## Code Areas

- `src/api/**`: user-facing SDK API paths
- `src/commands/**`: command entrypoints

## AI Docs

- `AGENTS.md`
- `.docpact/config.yaml`

Changing `src/api/**` or `src/commands/**` should require touching the appropriate docs according to `.docpact/config.yaml`.
