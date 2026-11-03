# Resistance script shared runtime (ACE ↔ Boredom Engine)

The authoritative source for resistance-script bundle contents is Boredom
Engine's release ZIP. ACE Engine imports it through
`InjectableScriptTree::load_from_json`, and the proxy injects the selected
scripts as classic nonce-authorized inline script tags.

## Optional catalog fields

- `SiteNode.runtimePath`: catalog-relative JavaScript bootstrap for a site.
- `ScriptEntry.sharedChunks`: catalog-relative array of required shared
  chunk registration scripts, including transitive dependencies.

The loader reads and validates runtime and chunk files at import time.
Paths must be relative JavaScript files without traversal, empty components,
backslashes or drive prefixes. Canonicalized paths must remain within the
extracted catalog directory; missing or escaping files fail the import.
Shared-chunk metadata is not serialized through the UI-facing ScriptEntry
representation.

For a requested host, `get_scripts` walks the same host hierarchy and
applies enabled status exactly as before. For each matching site node with
enabled scripts, it assembles this order:

1. Site runtime bootstrap (if configured).
2. The union of required chunk registration scripts for enabled entries only.
3. The enabled entry scripts, in their existing order.

The proxy continues to inject intensity setup first and gives all scripts
the same generated nonce. No network fetch, dynamic import, local HTTP
service, or new IPC path is involved. The runtime module registry lives in
the intercepted document's JavaScript execution world; each page/frame has
its own instance.

Legacy standalone IIFEs remain valid because both catalog fields are
optional. ACE must be updated together with Boredom releases that include
shared chunks.

## Tests

- `cargo test -p ace-engine --lib core::common::injectable_scripts`
- `cargo test -p ace-engine --lib`
- `cargo check -p ace-engine --bin ace-engine`

The loader tests cover ordering, deduplication, enabled-state filtering,
missing artifacts and invalid paths. The producer repository independently
checks the ZIP's completeness and executes a test bundle in a JS VM.
