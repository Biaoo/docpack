---
title: Sample SDK Agent Guide
docType: contract
scope: repo
status: active
authoritative: true
owner: sample-sdk
language: en
whenToUse:
  - when working inside the sample-sdk package
  - when deciding whether a change belongs in API code, commands, or repo-local AI docs
  - when demonstrating repo-level docpact behavior
whenToUpdate:
  - when repo ownership changes
  - when package-level routing changes
  - when validation expectations change
checkPaths:
  - AGENTS.md
  - .docpact/config.yaml
  - src/api/**
  - src/commands/**
  - docs/**
lastReviewedAt: 2026-04-20
lastReviewedCommit: example-sample-sdk
---

# Sample SDK

This package is a small example repository embedded inside `sample-workspace`.

## Ownership

- this package owns the TypeScript files under `src/`
- this package owns the repo-local AI docs under `.docpact/`

## Read Next

1. `.docpact/config.yaml`
2. `docs/api.md`
