# Design Principles

`docpact` is not a conversational assistant layered on top of lint output. It is a deterministic governance engine designed to work well with humans, agents, and CI systems.

This document captures the long-lived public principles behind the product.

## What docpact Optimizes For

`docpact` is built to help answer three recurring documentation-governance questions:

- before coding: what should I read first?
- after coding: what should have been reviewed or updated?
- ongoing: which governed documents may no longer be trustworthy?

Those questions become more important in agentic workflows, where code changes are cheap and documentation drift can spread quickly.

## Principles

### 1. Deterministic First

The engine is responsible for:

- loading config
- collecting diffs
- matching rules
- evaluating required docs
- validating metadata
- emitting stable findings

AI and higher-level workflow tooling can help decide what to fix next, but they should not replace pass/fail judgment or rule matching.

### 2. Structured Before Prose

`docpact` prefers stable structured results over long free-form explanations.

That means:

- text output stays compact and operational
- JSON is the preferred machine interface
- SARIF remains the standardized platform integration surface

Different views can look different, but they should all reflect the same underlying finding model.

### 3. Model Facts, Keep Explanations Separate

Model deterministic governance facts in config, keep explanatory source docs separate, and treat derived views as non-authoritative.

In practice:

- config holds facts the deterministic engine must consume directly
- source docs hold explanation, rationale, examples, exceptions, and runbooks
- derived views stay read-only and summarize existing authoritative facts rather than becoming a second truth source

This keeps `docpact` structured without forcing every useful explanation into schema fields.

### 4. Default Short, Expand Explicitly

The default result should contain only what is needed for the next decision.

Longer context should be requested explicitly through detail levels, drill-down, or follow-up commands rather than dumped by default. This keeps CLI output readable and reduces unnecessary agent token usage.

### 5. Token Cost Is Part of Product Design

For agent consumers, output size is not a cosmetic concern. It affects whether the tool is practical to use at all.

`docpact` therefore prefers:

- summaries before full dumps
- deterministic pagination
- single-finding drill-down
- reusable artifacts such as JSON reports

### 6. Explicit Artifacts Over Hidden State

`docpact` avoids hidden session state, background caches, or opaque in-memory coupling between commands.

When a workflow needs continuity, it should use explicit artifacts such as:

- lint reports
- baselines
- waivers

That keeps automation, CI, and agent workflows replayable.

### 7. Navigation Matters As Much As Detection

Finding a problem is only part of the job. The tool should also help answer:

- what should be fixed first
- what remains off-page
- which finding should be expanded next
- which governed document should be read before making a change

This is why `docpact` provides both enforcement commands and navigation commands such as `route`, diagnostics drill-down, and structured audit outputs.

## Design Implications

These principles show up in the current product shape:

- `route` recommends governed documents before coding
- `lint` enforces review/update requirements after coding
- `freshness` audits whether governed documents may be stale
- `baseline` and `waiver` stay explicit and file-backed
- config carries deterministic governance facts while source docs remain explanatory
- `render` exposes derived views without becoming a new authoring source
- `list-rules`, `doctor`, `coverage`, and skills build on top of the same deterministic core

## Scope

This document describes product-level principles only.

Feature-level decisions, CLI syntax changes, and implementation plans should continue to live in issues, design docs, and roadmap material rather than being embedded here.
