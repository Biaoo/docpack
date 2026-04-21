# ai-doc-lint

`ai-doc-lint` is a Rust-first standalone CLI for diff-driven AI documentation drift checks.

It is meant to be installed as a CLI and used in local workflows and CI.

Current distribution status:

- not published to crates.io yet
- usable today via `cargo run` or `cargo install --path .`
- intended future distribution: published crate and release binaries

## Quick Start

Install from source for now:

```bash
cargo install --path .
```

Create `.ai-doc-lint/config.yaml` in the target repository, then run:

```bash
ai-doc-lint validate-config --root /path/to/repo
ai-doc-lint validate-config --root /path/to/repo --strict
ai-doc-lint check --root /path/to/repo --files src/api/client.ts,README.md
```

For a full setup guide, start with [../docs/README.md](../docs/README.md).

## Document Map

- [../docs/README.md](../docs/README.md) / [../docs/README.zh-CN.md](../docs/README.zh-CN.md): documentation hub and reading guide
- [../docs/installation.md](../docs/installation.md) / [../docs/installation.zh-CN.md](../docs/installation.zh-CN.md): how to install or run the CLI today
- [../docs/usage.md](../docs/usage.md) / [../docs/usage.zh-CN.md](../docs/usage.zh-CN.md): quick start, commands, diff modes, outputs, and local workflow
- [../docs/configuration.md](../docs/configuration.md) / [../docs/configuration.zh-CN.md](../docs/configuration.zh-CN.md): detailed config reference and rule behavior
- [../docs/github-actions.md](../docs/github-actions.md) / [../docs/github-actions.zh-CN.md](../docs/github-actions.zh-CN.md): GitHub Actions integration examples
- [../docs/product-vision.md](../docs/product-vision.md) / [../docs/product-vision.zh-CN.md](../docs/product-vision.zh-CN.md): product positioning, scope, and roadmap
- [../docs/roadmap.md](../docs/roadmap.md) / [../docs/roadmap.zh-CN.md](../docs/roadmap.zh-CN.md): staged delivery plan, coverage milestones, and priorities
- [../docs/features.md](../docs/features.md) / [../docs/features.zh-CN.md](../docs/features.zh-CN.md): current capabilities, limitations, and implementation notes
- [examples/workspace-config.yaml](./examples/workspace-config.yaml): standalone reference snippet for a workspace `config.yaml`
- [examples/repo-config.yaml](./examples/repo-config.yaml): standalone reference snippet for a repo `config.yaml`

The new project also standardizes on one reserved config entrypoint:

- `.ai-doc-lint/config.yaml`

## Current State

This repository currently contains a working Phase 1 core:

- changed-path collection from explicit files or git diff sources
- workspace/repo impact-rule loading
- trigger-to-required-doc matching
- key Markdown and YAML metadata checks
- warning vs blocking exit behavior
- text and JSON-capable reporting surfaces

It is not yet the full planned product. Higher-order drift detection, autofix, richer config validation, an official GitHub Action wrapper, and published package distribution remain future work.
