// The Tauri WebdriverIO guest adapter exists only in Vite development.
// Production bundles tree-shake this branch; Rust does not register the
// corresponding plugins outside debug_assertions builds.
if (import.meta.env.DEV && typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    void import("@wdio/tauri-plugin");
}
