import { beforeEach, describe, expect, it, vi } from "vitest";
import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "svelte-sonner";
import {
    ACCOUNT_PAGE_URLS,
    getAccountPageUrl,
    openAccountPage,
} from "./account-browser";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("svelte-sonner", () => ({ toast: { error: vi.fn() } }));
vi.mock("$lib/paraglide/messages.js", () => ({
    account_browser_open_failed: () => "Could not open your browser",
    account_browser_open_failed_detail: ({ url }: { url: string }) => url,
}));

describe("account browser handoff", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        vi.mocked(openUrl).mockResolvedValue(undefined);
    });

    it("uses the local website in debug and the public website in release builds", () => {
        expect(getAccountPageUrl("pro", true)).toBe(
            "http://localhost:5173/get-pro?origin=aceblocker",
        );
        expect(getAccountPageUrl("sign-in", true)).toBe(
            "http://localhost:5173/signin?origin=aceblocker",
        );
        expect(getAccountPageUrl("pro", false)).toBe(
            "https://aceblocker.com/get-pro?origin=aceblocker",
        );
        expect(getAccountPageUrl("sign-in", false)).toBe(
            "https://aceblocker.com/signin?origin=aceblocker",
        );
        expect(ACCOUNT_PAGE_URLS.pro).toBe(getAccountPageUrl("pro"));
    });

    it("opens Get Pro from the desktop session without calling the engine", async () => {
        vi.mocked(isTauri).mockReturnValue(true);

        expect(await openAccountPage("pro")).toBe(true);
        expect(openUrl).toHaveBeenCalledWith(ACCOUNT_PAGE_URLS.pro);
    });

    it("sign-in includes the origin required for deep-link activation", async () => {
        vi.mocked(isTauri).mockReturnValue(true);

        expect(await openAccountPage("sign-in")).toBe(true);
        expect(openUrl).toHaveBeenCalledWith(ACCOUNT_PAGE_URLS["sign-in"]);
        expect(ACCOUNT_PAGE_URLS["sign-in"]).toContain("origin=aceblocker");
    });

    it("uses a regular browser tab outside Tauri", async () => {
        vi.mocked(isTauri).mockReturnValue(false);
        const open = vi.fn();
        vi.stubGlobal("window", { open });

        expect(await openAccountPage("pro")).toBe(true);
        expect(open).toHaveBeenCalledWith(
            ACCOUNT_PAGE_URLS.pro,
            "_blank",
            "noopener,noreferrer",
        );
        expect(openUrl).not.toHaveBeenCalled();
        vi.unstubAllGlobals();
    });

    it("reports browser launch failure instead of silently succeeding", async () => {
        vi.mocked(isTauri).mockReturnValue(true);
        vi.mocked(openUrl).mockRejectedValue(new Error("No browser available"));
        const log = vi.spyOn(console, "error").mockImplementation(() => {});

        expect(await openAccountPage("pro")).toBe(false);
        expect(toast.error).toHaveBeenCalledWith(
            "Could not open your browser",
            { description: ACCOUNT_PAGE_URLS.pro },
        );
        log.mockRestore();
    });
});
