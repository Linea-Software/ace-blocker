---
name: inline-docs
description: Document code using efficient inline comments. Use whenever writing, editing, or reviewing code or if the user mentions documentation.
argument-hint: What code do you want to document?
license: MIT
---

# Inline Documentation

Documentation remedies ambiguity and supplies missing context. Good code is _self-documenting_ — it shows the _what_, so a comment must _earn_ its place by adding the _why_. Keep docs lean.

## What to document

- Comment where the code _surprises_: intent, constraints, workarounds, logic that looks wrong but works. Add the _why_ when it spares the reader a wrong guess; let plain code speak for itself.
- Reserve inline comments for genuinely complicated logic; short phrases.
- Mark unfinished work with TODO/FIXME.
- Keep a comment beside its code and revise both in one edit; comments _rot_ as code drifts.
- Write for the _future reader_, who knows the code but not your session or the surrounding stack: keep comments _timeless_ and _self-contained_.

## How to write it

- Use block doc comments (JavaDoc, JSDoc, docstrings) for the standard, dev-readable format.
- Write like a senior developer: _terse_ fragments, human names, specific over generic.
- Every comment is _signal_: cut the noise (keeps, so, because, stays), and prefer commas and semicolons to full sentences.
- Shun _ceremony_: structure speaks for itself, so section headers like `// -------- Variables --------` only bloat tokens.
- Use typeable ASCII only; it keeps docs easy to edit.

## Language References

Load in the specific documentation standard(s) for the current project:

| Language              | Reference         |
| --------------------- | ----------------- |
| JavaScript/TypeScript | [JSDoc](jsdoc.md) |
