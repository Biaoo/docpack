# docpact

`docpact` is a Rust-first standalone CLI for diff-driven AI documentation drift checks.

It is meant to be installed as a CLI and used in local workflows and CI.

Current distribution status:

- published on crates.io
- installable via `cargo install docpact`
- runnable from source via `cargo run`
- installable from a local checkout via `cargo install --path .`

## Quick Start

Install from crates.io:

```bash
cargo install docpact
```

Create `.docpact/config.yaml` in the target repository, then run:

```bash
docpact validate-config --root /path/to/repo
docpact validate-config --root /path/to/repo --strict
docpact lint --root /path/to/repo --files src/api/client.ts,README.md
```

For a full setup guide, start with [../docs/README.md](../docs/README.md).

If you are developing `docpact` itself, local source workflows are still supported via `cargo run` and `cargo install --path .`.

## Document Map

- [../docs/README.md](../docs/README.md) / [../docs/README.zh-CN.md](../docs/README.zh-CN.md): documentation hub and reading guide
- [../docs/installation.md](../docs/installation.md) / [../docs/installation.zh-CN.md](../docs/installation.zh-CN.md): how to install or run the CLI today
- [../docs/usage.md](../docs/usage.md) / [../docs/usage.zh-CN.md](../docs/usage.zh-CN.md): quick start, commands, diff modes, outputs, and local workflow
- [../docs/configuration.md](../docs/configuration.md) / [../docs/configuration.zh-CN.md](../docs/configuration.zh-CN.md): detailed config reference and rule behavior
- [../docs/github-actions.md](../docs/github-actions.md) / [../docs/github-actions.zh-CN.md](../docs/github-actions.zh-CN.md): GitHub Actions integration examples
- [examples/github-actions/](./examples/github-actions): official workflow examples for PR lint, adoption controls, coverage audit, and freshness audit
- [../docs/product-vision.md](../docs/product-vision.md) / [../docs/product-vision.zh-CN.md](../docs/product-vision.zh-CN.md): product positioning, scope, and roadmap
- [../docs/roadmap.md](../docs/roadmap.md) / [../docs/roadmap.zh-CN.md](../docs/roadmap.zh-CN.md): staged delivery plan, coverage milestones, and priorities
- [../docs/features.md](../docs/features.md) / [../docs/features.zh-CN.md](../docs/features.zh-CN.md): current capabilities, limitations, and implementation notes
- [examples/workspace-config.yaml](./examples/workspace-config.yaml): standalone reference snippet for a workspace `config.yaml`
- [examples/repo-config.yaml](./examples/repo-config.yaml): standalone reference snippet for a repo `config.yaml`

The new project also standardizes on one reserved config entrypoint:

- `.docpact/config.yaml`

## Current State

This repository currently contains a working Phase 1 core:

- changed-path collection from explicit files or git diff sources
- workspace/repo impact-rule loading
- trigger-to-required-doc matching
- key Markdown and YAML metadata checks
- warning vs blocking exit behavior
- text and JSON-capable reporting surfaces

It is not yet the full planned product. Higher-order drift detection, autofix, richer config validation, and broader ecosystem integrations remain future work.
