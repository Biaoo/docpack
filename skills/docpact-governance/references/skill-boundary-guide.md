# Skill Boundary Guide

This skill only routes governance-maintainer work. It does not replace the direct workflow entrypoint and it does not absorb child-skill procedures.

## Use `docpact` instead of `docpact-governance` when the task is:

- choosing what to read before coding
- running `lint` for a concrete diff
- drilling into one diagnostic
- recording completed review evidence
- checking whether governed docs are stale for immediate task trust

## Use `docpact-governance` when the task is:

- deciding how to onboard or restructure repository governance
- planning coverage backfill across uncovered areas
- drafting or refactoring rules
- maintaining controlled routing aliases
- auditing the rule graph as a system
- designing CI usage of the official wrapper
- performing stale-doc maintenance as repository governance work

## Use `failure-repair` only when:

- you already have a structured lint report
- the problem is narrowed to one `diagnostic_id`
- the next step is to repair, escalate, or classify that one finding

`failure-repair` is shared across direct workflow and maintainer workflow. It is not the default governance entrypoint.

## Child-skill duplication rule

When you route to a child skill:

- name the child skill
- explain why it matches
- list the minimum inputs to gather

Do not restate the child skill's whole process unless the user explicitly asks for that detail.

## Product-gap rule

If none of the official maintainer skills fit, say so. Fall back to CLI inspection and call out the gap explicitly. Do not invent:

- new config semantics
- new skill-only workflow states
- new adoption-control behaviors
- new route or lint meanings
