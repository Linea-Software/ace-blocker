# Ace App — UI identity and rebuild brief

Status: **approved target direction** for the next Ace desktop frontend.

This document defines the local visual and interaction rules for `ace-app`.
It adds to the project-level brief at `../docs/ai/UI_IDENTITY.md`.

## 1. Objective

Rebuild Ace App as a dedicated desktop control instrument.
Do not make it look like a web dashboard inside Tauri.

The product thesis is:

> **Configure once. Hold the line.**

Ace enforces decisions that users make when they can think clearly.
These decisions must continue to apply when the user is tempted to change them.
Make configuration calm and clear.
Make enforcement and commitment visible in the structure of the UI.

Recognition target:

> Remove the Ace logo and product name from a screenshot.
> The app must still be recognizable by its colors, typography, geometry,
> state indicators, and product-specific components.

## 2. Technical foundation

Keep:

- Tauri 2;
- SolidJS 2.0 and Solid Router 2;
- Vite 8 (client-side static build);
- TypeScript;
- Bun;
- Tailwind CSS 4;
- Paraglide;
- generated `@hey-api` engine client;
- Vitest;
- Playwright.

Use **Solid 2-compatible accessible primitives** for new Ace-owned UI, with Ace-owned styling. The existing Svelte Bits UI implementation is superseded by the Solid 2 port.

The former frontend used shadcn-svelte and Bits UI. Treat those as legacy implementation details; do not restore Svelte components or their dependencies to the Solid frontend.

Rules:

- Do not add a new shadcn component only to get its default style.
- Reimplement necessary legacy components in Solid 2, preserving semantics, accessibility, and appearance.
- New design-system components must own their Ace styles and APIs.
- Prefer headless component behavior and Ace CSS over generic shadcn layouts.
- Use Tailwind as an implementation tool. Do not use it as the visual design language.

The engine API is a stable boundary for the frontend redesign.
Do not change engine contracts only to simplify the UI.
Change these contracts only when the task expressly permits an API change.

## 3. Mandatory design skills

For work that creates, redesigns, or materially changes visible UI:

1. Read this document.
2. Use `frontend-design`.
3. Use `ui-ux-pro-max`.
4. For Solid 2 code, use the _package-pinned_ `solid-js/skills/solid-2.0` and `reactivity-diagnostics` skill files, plus the current `CHEATSHEET.md`. Confirm third-party skills against Solid 2 docs before using them.
5. Use `web-design-guidelines` for the final standards and accessibility review when applicable.
6. Check important visual changes in actual screenshots, a browser, or the Tauri UI.

You can use `design-taste-frontend` to identify generic UI patterns.
This skill does not replace dashboard or multi-step product-UI design.
Always use product-specific UX reasoning.

## 4. Visual thesis

Make Ace look like a **disciplined personal enforcement instrument**.

Qualities:

- decisive;
- physical;
- controlled;
- resistant;
- deliberate;
- durable;
- calm when idle;
- uncompromising at commitment boundaries.

Useful references include industrial controls, measuring equipment, technical
signage, durable tools, and editorial information design.

Avoid:

- military aesthetics;
- cyber/hacker theater;
- terminal cosplay;
- wellness/meditation softness;
- generic SaaS dashboard patterns.

## 5. Palette

### Mineral / Gunmetal (shared with the website)

| Semantic surface | Light | Dark |
| --- | --- | --- |
| Workspace | `#CDDCE5` | `#12171C` |
| Panel | `#E2EAF0` | `#212932` |
| Raised dialog / menu | `#F0F5F8` | `#2C3641` |
| Modal header / footer | `#E2EAF0` | `#35414D` |
| Control | `#D5E1E9` | `#414E5C` |
| Strong border | `#647C8C` | `#94A4B3` |
| Primary text | `#202E39` | `#E9EEF3` |
| Secondary text | `#465A69` | `#BFCAD5` |

Brand amber remains `#FFB000` in both themes. Use semantic surface variables in
`src/app.css`; do not introduce hard-coded theme colors in individual controls.
Dark controls are brighter than their parent surfaces. Dialogs use a distinct
raised surface, opaque controls, a dimmed backdrop, a defined edge and a shadow.
Keep ordinary panels flat.

### Semantic

- Success: `#166534` (light), `#4ADE80` (dark)
- Danger: `#B42318` (light), `#EF786D` (dark)
- Information: `#426784` (light), `#8EACCB` (dark)

### Meaning

Amber identifies **Pro entitlement and upgrade actions**. Use a solid amber CTA only for a direct upgrade. Keep gated feature controls neutral with a compact amber lock/Pro marker. The brand amber is #FFB000 in both themes; in light mode use dark text on or next to the amber marker instead of darkening the brand color.

Network Block activity uses a 5px success-green rail, status dot and state text; inactive states use gray. Locks use neutral surfaces, typography and a lock icon, not amber. The Network Block editor uses full-surface inverted contrast for the current step and forward/save actions. Inactive steps have distinct bordered surfaces; completed steps display a check mark. Selected categories and schedule modes use monochrome borders and check/radio indicators. Blue is reserved for informational states. Errors use danger red; informational notices use the information token. Keep focus indicators visible in both themes.

## 6. Typography

Use:

- **IBM Plex Sans** for normal UI text;
- **IBM Plex Sans Condensed** for page titles and important state labels;
- **IBM Plex Mono** for times, durations, counts, schedules, and machine or status data.

Do not use monospace only as technical decoration.

Avoid:

- all-caps eyebrow labels;
- excessive letter spacing;
- large marketing-style display text inside the desktop app;
- hierarchy based only on font weight.

## 7. Geometry and surfaces

Target corner radii:

- controls: 4–6 px;
- panels: 6–8 px;
- large structural surfaces: <= 8 px.

Use pills only for true compact states or metadata.

Ace-owned domain components can use a small notched or cut-corner detail.
Use this detail on important product objects, not on every container.

Prefer borders, rails, rules, tonal surfaces, and inset regions.
Avoid floating cards and large shadows except for modal separation.
The app must feel **assembled**, not floated.

## 8. Application shell

Target shell:

- a thin navigation rail, approximately 64–72 px;
- a main workspace;
- a right-side contextual inspector when a domain object is selected or edited;
- global engine and enforcement status that remains easy to see.

Do not rebuild the broad, web-style sidebar.

Suggested top-level navigation:

- Control
- Blocks
- Resistance
- Categories
- Guide
- Settings

UX tests can change the exact labels.
Keep the information architecture compact.

### Contextual inspector

Use a contextual inspector for lightweight object details and quick edits.
The Network Block policy editor is a full-page route by design.
Its content, schedule, and lock controls need more space than an inspector.
Keep inspectors for Categories and Custom Lists.

Use dialogs for:

- destructive actions;
- commitment and lock actions;
- unusual multi-step operations.

## 9. Page archetypes

Do not make all product concepts use `PageHeader + Card grid`.

### Control

The home screen is a **state board**, not a KPI dashboard.

It must answer:

1. Is Ace working?
2. What is Ace enforcing now?
3. What changes next?

Show active policies and upcoming schedule changes before generic metric cards.

### Blocks

Network Blocks are **policy objects**.

Show these facts for each block:

- name;
- current state;
- enforced schedule;
- policy contents;
- lock state;
- next relevant transition.

Use a strong state rail or a similar structural indicator.
For a locked block, visibly limit editing.
Do not show only a small padlock badge.

### Resistance

Resistance is an Ace-specific screen for **independent interventions**.

Do not show a global intensity or severity slider.
The engine no longer has this setting.
Give a short, concrete explanation.
Show exactly **five** categories with equal priority:

- feeds and suggestions;
- videos and playback;
- visual stimulation;
- delays and limits;
- simulated verifications (CAPTCHA).

Start all five categories collapsed.
Use only these five top-level categories.
Do not add supergroups, extra headings, or another disclosure layer.

Each category header has an always-visible, three-state checkbox.
Keep this checkbox separate from the expand button.
Users must be able to select an entire category without opening it or browsing sites.
Within a category, show additional intervention choices for each site.

CAPTCHA scripts are off by default in a new catalog installation.
Users can enable all CAPTCHA scripts with the category checkbox.
Do not overwrite existing user choices with these defaults.
Time Locks prevent users from clearing or weakening protected selections.
Time Locks still permit users to add interventions.
Show the difference between configured selections and active enforcement.
Put script updates and installed-version maintenance in Settings, not on this page.

### Categories

Categories are curated sets of known websites provided by Ace.
Custom Lists are domain lists imported by the user.
Show both in a dense, easy-to-scan catalog or inventory.
Show Categories first and Custom Lists as a separate secondary group.
Prefer rows or tables with an inspector over marketing cards.

The engine still represents both types as Site Packs.
Use Site Packs at the implementation boundary.
Use Categories and Custom Lists in normal product text.

### Schedule

Prefer a weekly time matrix or timeline.
Show enforcement periods by their position in the week or day.

## 10. Signature components

Design these components first:

1. `NetworkPolicy`
2. `ScriptSections (independent intervention browser)`
3. `LockSeal`
4. `ScheduleTimeline` / `WeekGrid`
5. `EngineState`
6. `SitePackRow` (internal name; displays Categories and Custom Lists)
7. `Inspector`

Make these components distinctive and consistent.
Other screens can use their common visual structure.

## 11. Lock interaction

Locking has important consequences.
Require a deliberate commitment action.

A lock surface must clearly show:

- the lock duration;
- what the user cannot change while the lock is active;
- when the user can edit again.

A hold-to-confirm action is acceptable for a real lock or commitment.
Make the hold long enough to be deliberate, but not punitive.
Do not require hold-to-confirm for ordinary actions.

## 12. Motion

Typical duration: 120–180 ms.

Use motion to explain a change in structure:

- an inspector opens;
- a policy expands;
- a lock seal closes;
- a schedule snaps into place;
- a state rail changes.

Avoid:

- hover-lift cards;
- bouncy motion;
- ambient pulse;
- decorative section entrances;
- animation added only for a “premium feel.”

Respect reduced-motion settings.

## 13. Copy and terminology

Use clear terms before unusual brand language.

These repeated terms are possible product vocabulary:

- Armed — possible global enforcement state;
- Engage resistance — possible Resistance action;
- Hold to lock — commitment action.

Do not rename familiar actions only to make them sound special.
Use “Save changes” instead of an unclear branded verb.

## 14. Anti-patterns

Do not use these patterns as defaults:

- generic rounded card grids;
- gradient washes;
- blurred blobs;
- glassmorphism;
- giant soft shadows;
- rounded icon tiles beside every title;
- tiny uppercase eyebrow labels;
- decorative rainbow feature colors;
- pills everywhere;
- arrows everywhere;
- premium glows;
- cybersecurity shield, hexagon, or terminal motifs.

## 15. Component system architecture

Use three layers.

### Primitives

Examples:

- Button
- Input
- Toggle
- Menu
- Dialog
- Tooltip
- Scroller

These components must be accessible, restrained, and visually simple.

### Ace structural components

Examples:

- Workspace
- NavigationRail
- Inspector
- SectionRail
- StatusStrip
- CommitBar
- WeekGrid

These components create the common Ace visual structure.

### Domain components

Examples:

- NetworkPolicy
- ScriptSections (independent intervention browser)
- LockSeal
- SitePackRow
- EngineState
- ScheduleTimeline

These components carry the clearest Ace identity.

Do not make the generic `Button` component carry the entire brand.

## 16. Accessibility and validation

A distinctive design must also be easy to use.

Required:

- keyboard navigation;
- visible keyboard focus;
- semantic controls;
- correct labels;
- sufficient contrast in both themes;
- reduced-motion support;
- no state that relies only on color;
- target sizes suitable for desktop input;
- readable UI with zoom or scaling.

Before you complete a major visual task:

- run the applicable Svelte checks;
- run the Svelte autofixer on changed Svelte files;
- inspect the rendered result;
- review representative light and dark states;
- test active, inactive, locked, error, and offline states when the component supports them.

## 17. Migration principle

You can replace the current app in stages or through a larger rebuild.
Preserve the stable engine boundary and product behavior in both cases.

Do not build a “new Ace” layer that only wraps the old generic card layout.
When you migrate a screen, move it toward this approved design system.
Do not retain its old visual assumptions by default.
