# JSDoc Best Practices

Structure JavaScript/TypeScript docs with JSDoc: doc pages generate cleanly, and devs read one standard format.

## Documentation Patterns

Organize and format block comments using Markdown.

General structure for all entities:

| Element | Usage | Purpose |
|---|---|---|
| description | Required |One-line phrase explaining function and purpose. Placed in tag for preambles. |
| `@remarks` | Optional | Interesting implementation/design decisions, implications, constraints, non-obvious behavior. Use bullet points for multiple remarks. |
| `{@link <link>}` | Optional, Inline, Multi | Link to related code entity or external url. |
| `@see` | Optional, Multi | Reference to related content. Must be used with `@link`. |
| `@typeParam` | Required on generic, Multi | Document a generic param. |
| `@deprecated` | Optional | Obsolete, unsupported, or not recommended. |

### File Preambles

Every file must have a preamble comment at the top with all elements:

| Element | Purpose |
|---|---|
| `@file` | The filename (for navigation and search). |
| `@description` | What this file provides. |
| `@module` | The feature module this belongs to. |

### Method Documentation

| Element | Usage | Purpose |
|---|---|---|
| `@param` | Multi, Optional in TypeScript, but all-or-none. | What a parameter is in context, preconditions, units, defaults. In JavaScript, every param must be documented with its type; if a param is self-explanatory, omit the description. Object fields can be documented as `objectName.field`; invent an object name if destructured. |
| `@returns` | Required if non-void return | What the method returns and when. |
| `@throws` | Multi, Required for exceptions manually thrown or possible when preconditions are met. | What exceptions may be thrown and when. Use with `{@link <exception>}` on the first line. |

### Type/Constant Documentation

Doc comments can be placed on both the type def itself and above each element inside.

## Oxlint/ESLint Enforcement

Doc standards are enforced via the linter, which must be run after every set of changes.

### Required Documentation

| Rule | Setting | What It Enforces |
|---|---|---|
| `jsdoc/require-jsdoc` | error | JSDoc on file preambles, function declarations, interfaces, type aliases, and PascalCase arrow functions |
| `jsdoc/require-returns-description` | error | All `@returns` tags must have descriptions |
| `jsdoc/require-property-description` | error | All `@property` tags must have descriptions |

### Allowed Tags

| Rule | Setting | Effect |
|---|---|---|
| `jsdoc/check-tag-names` | `definedTags: ["remarks"]` | Allows `@remarks` for "why" documentation |
| `jsdoc/no-types` | off | TypeScript types in JSDoc are optional |
| `jsdoc/require-param-type` | off | Types come from TypeScript, not JSDoc |
| `jsdoc/require-returns-type` | off | Types come from TypeScript, not JSDoc |
