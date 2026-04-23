# Modeling Boundary

This document explains where information should live in a `docpact`-governed repository:

- `.docpact/config.yaml`
- source docs
- derived views

Use it before proposing config edits, document maintenance, or generated summary surfaces.

## Quick Decision Order

Ask these questions in order:

1. Does this information already have an authoritative executable source somewhere else?
2. Does the `docpact` engine need to consume it directly as deterministic governance data?
3. Is its main value explanation, rationale, examples, exceptions, troubleshooting, or runbook guidance?
4. Is it only a short summary over facts that already live somewhere authoritative?

Default outcomes:

- if the authoritative source already exists elsewhere, do not copy it into `docpact` unless the engine must consume it directly
- if the engine must consume it directly, put it in config
- if the main value is explanation or guidance, keep it in source docs
- if it is a short summary over authoritative facts, treat it as a derived view

## What Belongs In Config

Put information in `.docpact/config.yaml` when it is part of the deterministic governance model.

Good fit:

- data the engine must read directly
- facts that affect routing, lint judgments, coverage, freshness, or derived render output
- information that can be expressed stably as schema fields

Examples:

- `catalog`
- `ownership`
- `routing.intents`
- `rules`
- `coverage`
- `docInventory`
- `freshness`

Do not use config as a dumping ground for explanations that only help humans or agents understand the system.

## What Belongs In Source Docs

Keep information in source docs when its main value is explanation rather than deterministic evaluation.

Good fit:

- troubleshooting guides
- runbooks
- design rationale
- ADRs and historical plans
- exception handling guidance
- narrow domain references and background material

These documents can be long, contextual, and narrative. That is a feature, not a modeling failure.

## What Belongs In Derived Views

Treat derived views as read-only entry surfaces over authoritative facts.

Good fit:

- ownership summaries
- catalog summaries
- navigation summaries
- bootstrap cheat sheets
- repo cards
- short path maps

Derived views help with fast reading and low token cost, but they are not a second authoring surface. If a summary needs manual maintenance to stay correct, it probably does not belong here.

`render` is the current derived-view layer in `docpact`. Use it to expose compact context, not to create a new source of truth.

## Do Not Copy Other Executable Truth

`docpact` config is not a mirror of every important repository file.

Examples of authoritative executable truth that should usually stay where they already live:

- `.nvmrc`
- `package.json`
- workflow YAML files
- hook scripts
- build tool configuration

`docpact` can route to these surfaces, govern docs around them, or summarize related ownership/context. It should not duplicate their values in config unless those values are also required inputs to the deterministic governance model.

## Common Misclassifications

Bad pattern:

- copying config facts into hand-maintained summaries, ownership tables, or bootstrap docs

Better:

- keep the facts in config
- expose short summaries as derived views

Bad pattern:

- forcing rationale, exceptions, or troubleshooting detail into schema fields

Better:

- keep the deterministic contract in config
- keep explanatory detail in source docs

Bad pattern:

- treating `render` output as something humans should edit by hand

Better:

- treat `render` output as disposable, read-only output
- update config or source docs instead

## When In Doubt

Use this shortcut:

- if the engine needs it, config
- if readers need it, source docs
- if readers only need a short summary of existing facts, derived views

When a case still feels ambiguous, prefer keeping the deterministic model small and keeping explanation in source docs.
