# sample-workspace

This example demonstrates a small workspace-level and repo-level AI documentation system that `docpact` can inspect.

This example intentionally does not rely on `.docpact/task-router.md` or `.docpact/validation.md`. Those files are optional conventions, not engine-reserved names.

## Layout

- `AGENTS.md`: root guidance for the sample workspace
- `.docpact/config.yaml`: workspace-level AI contract and rule entrypoint
- `docs/branch-policy.md`: a non-AI doc that triggers root doc review rules
- `sample-sdk/`: a repo-like package with its own AI docs and code

## Useful Demo Commands

Successful repo-local review example:

```bash
cargo run -- lint \
  --root /Users/biao/Code/docpact/examples/sample-workspace \
  --files sample-sdk/src/api/client.ts,sample-sdk/README.md,sample-sdk/AGENTS.md,sample-sdk/.docpact/config.yaml,sample-sdk/docs/api.md
```

Intentional failure example:

```bash
cargo run -- lint \
  --root /Users/biao/Code/docpact/examples/sample-workspace \
  --files sample-sdk/src/api/client.ts
```

Workspace config validation:

```bash
cargo run -- validate-config \
  --root /Users/biao/Code/docpact/examples/sample-workspace
```

Path explanation:

```bash
cargo run -- explain sample-sdk/src/commands/sync.ts \
  --root /Users/biao/Code/docpact/examples/sample-workspace
```
