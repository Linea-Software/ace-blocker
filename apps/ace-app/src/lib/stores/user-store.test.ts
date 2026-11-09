import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getAccountInfo, refreshAccount, triggerContentUpdate } from "$engine-client";
import type { AccountInfoResponse } from "$engine-client";

vi.mock("$engine-client", () => ({
    getAccountInfo: vi.fn(),
    refreshAccount: vi.fn(),
    triggerContentUpdate: vi.fn(),
}));

const accountResponse = (isPro: boolean) =>
    ({ data: { isPro, tosAccepted: true } satisfies AccountInfoResponse }) as Awaited<
        ReturnType<typeof getAccountInfo>
    >;

describe("Pro activation content refresh", () => {
    let store: (typeof import("./user-store.svelte"))["userStore"];

    beforeEach(async () => {
        vi.useFakeTimers();
        // Vitest executes .svelte.ts modules as plain TS; supply a minimal
        // rune identity for this logic-only test (no component is rendered).
        vi.stubGlobal("$state", <T>(value: T) => value);
        vi.resetModules();
        vi.clearAllMocks();
        vi.mocked(getAccountInfo).mockResolvedValue(accountResponse(false));
        vi.mocked(refreshAccount).mockResolvedValue(accountResponse(false));
        vi.mocked(triggerContentUpdate).mockResolvedValue({} as Awaited<ReturnType<typeof triggerContentUpdate>>);
        ({ userStore: store } = await import("./user-store.svelte"));
        await store.fetch();
    });

    afterEach(() => {
        store?.destroy();
        vi.unstubAllGlobals();
        vi.useRealTimers();
    });

    it("only asks the engine to sync when Pro is confirmed, once per entitlement", async () => {
        expect(store.isPro).toBe(false);
        expect(triggerContentUpdate).not.toHaveBeenCalled();

        vi.mocked(refreshAccount).mockResolvedValue(accountResponse(true));
        await store.refreshAfterLoginOrEntitlementChange();
        // The content download starts without blocking account verification.
        await Promise.resolve();
        await Promise.resolve();
        expect(store.isPro).toBe(true);
        expect(triggerContentUpdate).toHaveBeenCalledTimes(1);
        expect(store.proContentRevision).toBe(1);

        await store.refreshAfterLoginOrEntitlementChange();
        expect(triggerContentUpdate).toHaveBeenCalledTimes(1);
    });

    it("refreshes available packs after a partial update failure and retries later", async () => {
        const log = vi.spyOn(console, "error").mockImplementation(() => {});
        try {
            vi.mocked(triggerContentUpdate).mockResolvedValueOnce({
                error: { code: "partial_failure", status: 503, error: "one pack unavailable" },
            } as Awaited<ReturnType<typeof triggerContentUpdate>>);
            vi.mocked(refreshAccount).mockResolvedValue(accountResponse(true));
            await store.refreshAfterLoginOrEntitlementChange();
            await Promise.resolve();
            await Promise.resolve();

            // Some metadata can already be saved, despite a partial download error.
            expect(store.proContentRevision).toBe(1);
            expect(triggerContentUpdate).toHaveBeenCalledTimes(1);
            await store.refreshAfterLoginOrEntitlementChange();
            expect(triggerContentUpdate).toHaveBeenCalledTimes(1);

            vi.mocked(getAccountInfo).mockResolvedValue(accountResponse(true));
            await vi.advanceTimersByTimeAsync(60_001);
            await store.refreshAfterLoginOrEntitlementChange();
            await Promise.resolve();
            await Promise.resolve();
            expect(triggerContentUpdate).toHaveBeenCalledTimes(2);
            expect(store.proContentRevision).toBeGreaterThanOrEqual(2);
        } finally {
            log.mockRestore();
        }
    });

    it("keeps Pro and installed content while release metadata is offline, then retries", async () => {
        const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
        const errorLog = vi.spyOn(console, "error").mockImplementation(() => {});
        try {
            vi.mocked(triggerContentUpdate).mockResolvedValueOnce({
                error: {
                    code: "content_update_unavailable",
                    status: 503,
                    error: "release metadata temporarily unavailable",
                },
            } as Awaited<ReturnType<typeof triggerContentUpdate>>);
            vi.mocked(refreshAccount).mockResolvedValue(accountResponse(true));
            vi.mocked(getAccountInfo).mockResolvedValue(accountResponse(true));

            await store.refreshAfterLoginOrEntitlementChange();
            await Promise.resolve();
            await Promise.resolve();

            expect(store.isPro).toBe(true);
            expect(store.proContentRevision).toBe(0);
            expect(triggerContentUpdate).toHaveBeenCalledTimes(1);
            expect(warn).toHaveBeenCalledOnce();
            expect(errorLog).not.toHaveBeenCalled();

            await store.refreshAfterLoginOrEntitlementChange();
            expect(triggerContentUpdate).toHaveBeenCalledTimes(1);

            await vi.advanceTimersByTimeAsync(60_001);
            await store.refreshAfterLoginOrEntitlementChange();
            await Promise.resolve();
            await Promise.resolve();
            expect(triggerContentUpdate).toHaveBeenCalledTimes(2);
            expect(store.isPro).toBe(true);
            expect(store.proContentRevision).toBe(1);
        } finally {
            warn.mockRestore();
            errorLog.mockRestore();
        }
    });

    it("syncs again if a previously active Pro account is reactivated", async () => {
        vi.mocked(refreshAccount).mockResolvedValue(accountResponse(true));
        await store.refreshAfterLoginOrEntitlementChange();
        await Promise.resolve();
        await Promise.resolve();
        expect(store.proContentRevision).toBe(1);

        vi.mocked(refreshAccount).mockResolvedValue(accountResponse(false));
        await store.refreshAfterLoginOrEntitlementChange();
        vi.mocked(refreshAccount).mockResolvedValue(accountResponse(true));
        await store.refreshAfterLoginOrEntitlementChange();
        await Promise.resolve();
        await Promise.resolve();
        expect(triggerContentUpdate).toHaveBeenCalledTimes(2);
        expect(store.proContentRevision).toBe(2);
    });
});
