import { createSignal } from "solid-js";

export type ThemeMode = "light" | "dark" | "system";
const storageKey = "mode-watcher-mode";
const [mode, updateMode] = createSignal<ThemeMode>("system");
export const themeMode = mode;

function valid(value: unknown): value is ThemeMode {
    return value === "light" || value === "dark" || value === "system";
}

function applyTheme(value: ThemeMode): void {
    if (typeof document === "undefined") return;
    const systemDark = window.matchMedia(
        "(prefers-color-scheme: dark)",
    ).matches;
    const dark = value === "dark" || (value === "system" && systemDark);
    document.documentElement.classList.toggle("dark", dark);
    document.documentElement.style.colorScheme = dark ? "dark" : "light";
}

/** Retains the existing mode-watcher storage key and dark CSS class. */
export function setMode(value: ThemeMode): void {
    if (!valid(value)) return;
    updateMode(value);
    try {
        localStorage.setItem(storageKey, value);
    } catch {
        /* private WebView */
    }
    applyTheme(value);
}

export function initializeTheme(): () => void {
    if (typeof window === "undefined") return () => {};
    try {
        const stored = localStorage.getItem(storageKey);
        if (valid(stored)) updateMode(stored);
    } catch {
        /* storage unavailable */
    }
    applyTheme(mode());
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme(mode());
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
}
