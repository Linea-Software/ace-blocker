import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { isTauri } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";
import { initializeTheme, setMode, themeMode } from "./theme";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: vi.fn() }));
vi.mock("@tauri-apps/plugin-store", () => ({ load: vi.fn() }));

const store = {
    get: vi.fn(),
    set: vi.fn(async () => {}),
    save: vi.fn(async () => {}),
};

let cache: Map<string, string>;
let darkClass: boolean;

beforeEach(() => {
    cache = new Map();
    darkClass = false;
    vi.mocked(isTauri).mockReturnValue(false);
    vi.mocked(load).mockResolvedValue(store as never);
    store.get.mockResolvedValue({ value: "dark" });
    vi.stubGlobal("localStorage", {
        getItem: (key: string) => cache.get(key) ?? null,
        setItem: (key: string, value: string) => cache.set(key, value),
    });
    const media = {
        matches: false,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
    };
    vi.stubGlobal("window", { matchMedia: () => media });
    vi.stubGlobal("document", {
        documentElement: {
            classList: {
                toggle: (_name: string, enabled: boolean) => {
                    darkClass = enabled;
                },
            },
            style: { colorScheme: "light" },
        },
    });
});

afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
});

describe("theme persistence", () => {
    it("restores browser mode from localStorage and never opens a native store", async () => {
        cache.set("mode-watcher-mode", "dark");
        const dispose = initializeTheme();
        await vi.waitFor(() => expect(themeMode()).toBe("dark"));
        expect(darkClass).toBe(true);
        setMode("light");
        expect(cache.get("mode-watcher-mode")).toBe("light");
        expect(darkClass).toBe(false);
        expect(load).not.toHaveBeenCalled();
        dispose();
    });

    it("restores the native settings.json selection at application start", async () => {
        vi.mocked(isTauri).mockReturnValue(true);
        cache.set("mode-watcher-mode", "system");
        const dispose = initializeTheme();
        await vi.waitFor(() => expect(themeMode()).toBe("dark"));
        expect(load).toHaveBeenCalledWith("settings.json");
        await vi.waitFor(() => expect(cache.get("mode-watcher-mode")).toBe("dark"));
        expect(darkClass).toBe(true);
        dispose();
    });

    it("does not overwrite a new user selection with a stale native read", async () => {
        vi.mocked(isTauri).mockReturnValue(true);
        let deliver!: (value: { value: string }) => void;
        store.get.mockReturnValue(new Promise((resolve) => { deliver = resolve; }));
        const dispose = initializeTheme();
        await vi.waitFor(() => expect(deliver).toBeTypeOf("function"));
        setMode("light");
        deliver({ value: "dark" });
        await vi.waitFor(() => expect(store.save).toHaveBeenCalled());
        await vi.waitFor(() => expect(themeMode()).toBe("light"));
        expect(cache.get("mode-watcher-mode")).toBe("light");
        expect(store.set).toHaveBeenCalledWith("theme", { value: "light" });
        dispose();
    });
});
