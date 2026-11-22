import { render } from "@solidjs/web";
import App from "./App";
import "./app.css";
import "./app-shell.css";

// Keep the existing debug-only Tauri/WebdriverIO guest bridge.
if (import.meta.env.DEV && typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    void import("@wdio/tauri-plugin");
}

const root = document.getElementById("app");
if (!root) throw new Error("Ace application mount point missing");
render(() => <App />, root);
