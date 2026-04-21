---
title: Sample Workspace Agent Guide
docType: contract
scope: workspace
status: active
authoritative: true
owner: sample-workspace
language: en
whenToUse:
  - when routing work from the workspace root
  - when deciding whether a change belongs in the sample workspace or in the sample-sdk package
  - when demonstrating workspace-level docpact behavior
whenToUpdate:
  - when workspace routing changes
  - when the package inventory changes
  - when root validation or review rules change
checkPaths:
  - AGENTS.md
  - .docpact/config.yaml
  - docs/branch-policy.md
  - .github/workflows/**
  - sample-sdk/**
lastReviewedAt: 2026-04-20
lastReviewedCommit: example-sample-workspace
---

# Sample Workspace

This workspace exists only as an `docpact` example.

## Ownership

- root owns workspace routing, branch policy, and shared validation expectations
- `sample-sdk` owns the SDK code and its package-local AI docs

## Bootstrap Order

1. read this file
2. read `.docpact/config.yaml`
3. enter `sample-sdk` only when the task clearly belongs to the SDK package
