import { createSignal } from "solid-js";
import { isTauri } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";

export type ThemeMode = "light" | "dark" | "system";
const storageKey = "mode-watcher-mode";
const [mode, updateMode] = createSignal<ThemeMode>("system");
export const themeMode = mode;

// Keep consecutive native saves ordered so an older selection cannot win.
let saveQueue: Promise<void> = Promise.resolve();
let selectionRevision = 0;

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

function cacheTheme(value: ThemeMode): void {
    try {
        localStorage.setItem(storageKey, value);
    } catch {
        /* storage unavailable */
    }
}

/** Change the active theme and persist it in the platform's canonical store. */
export function setMode(value: ThemeMode): void {
    if (!valid(value)) return;
    selectionRevision++;
    updateMode(value);
    cacheTheme(value);
    applyTheme(value);

    // Browser preview stores only localStorage; Tauri also persists in settings.json.
    if (typeof window !== "undefined" && isTauri()) {
        saveQueue = saveQueue
            .then(async () => {
                const store = await load("settings.json");
                await store.set("theme", { value });
                await store.save();
            })
            .catch((error: unknown) => {
                console.error("Failed to save theme preference:", error);
            });
    }
}

/**
 * Restore the cached mode immediately, then reconcile the authoritative native
 * settings.json preference without needing to visit the Settings route.
 * Ignore a late native read if the user already selected a different theme.
 */
export function initializeTheme(): () => void {
    if (typeof window === "undefined") return () => {};
    const initialRevision = selectionRevision;
    let active = true;
    let initialMode = mode();
    try {
        const stored = localStorage.getItem(storageKey);
        if (valid(stored)) {
            initialMode = stored;
            updateMode(stored);
        }
    } catch {
        /* storage unavailable */
    }
    // Solid 2 may defer the signal update; apply the read value immediately.
    applyTheme(initialMode);

    if (isTauri()) {
        void load("settings.json")
            .then((store) => store.get<{ value: string }>("theme"))
            .then((saved) => {
                if (!active || selectionRevision !== initialRevision ||
                    !valid(saved?.value)) return;
                updateMode(saved.value);
                cacheTheme(saved.value);
                applyTheme(saved.value);
            })
            .catch((error: unknown) => {
                console.error("Failed to load theme preference:", error);
            });
    }

    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme(mode());
    query.addEventListener("change", onChange);
    return () => {
        active = false;
        query.removeEventListener("change", onChange);
    };
}
