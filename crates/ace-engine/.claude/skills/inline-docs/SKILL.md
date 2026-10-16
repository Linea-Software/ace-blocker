---
name: inline-docs
description: Document code using efficient inline comments. Use whenever writing, editing, or reviewing code or if the user mentions documentation.
argument-hint: What code do you want to document?
license: MIT
---

# Inline Documentation

Documentation remedies ambiguity and supplies missing context. Good code is *self-documenting* — it shows the *what*, so a comment must *earn* its place by adding the *why*. Keep docs lean.

## What to document

- Comment where the code *surprises*: intent, constraints, workarounds, logic that looks wrong but works. Add the *why* when it spares the reader a wrong guess; let plain code speak for itself.
- Reserve inline comments for genuinely complicated logic; short phrases.
- Mark unfinished work with TODO/FIXME.
- Keep a comment beside its code and revise both in one edit; comments *rot* as code drifts.
- Write for the *future reader*, who knows the code but not your session or the surrounding stack: keep comments *timeless* and *self-contained*.

## How to write it

- Use block doc comments (JavaDoc, JSDoc, docstrings) for the standard, dev-readable format.
- Write like a senior developer: *terse* fragments, human names, specific over generic.
- Every comment is *signal*: cut the noise (keeps, so, because, stays), and prefer commas and semicolons to full sentences.
- Shun *ceremony*: structure speaks for itself, so section headers like `// -------- Variables --------` only bloat tokens.
- Use typeable ASCII only; it keeps docs easy to edit.

## Language References

Load in the specific documentation standard(s) for the current project:

| Language | Reference |
| --- | --- |
| JavaScript/TypeScript | [JSDoc](jsdoc.md) |
