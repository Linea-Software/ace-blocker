import { describe, expect, it } from "vitest";
import { SyncingStore, type SitePackSyncEntry } from "./syncing-store.svelte";
import type { SitePackCreationStatus } from "$engine-client";

/**
 * State-model tests for per-pack download progress. The reported bug was a
 * shared/mis-placed progress bar and indistinguishable per-pack states; these
 * tests lock the corrected contract:
 *
 *  - each pack id has fully independent state (concurrent downloads never
 *    share progress or leak status between each other);
 *  - queued / downloading / completed / failed are distinct;
 *  - failures keep their error text so the UI can offer a retry;
 *  - a retry (new stream for the same id) restarts from queued.
 */

function event(partial: SitePackCreationStatus): SitePackCreationStatus {
    return partial;
}

function streamOf(...events: SitePackCreationStatus[]): AsyncIterable<SitePackCreationStatus> {
    return (async function* () {
        for (const item of events) yield item;
    })();
}

describe("SyncingStore per-pack state model", () => {
    it("tracks each pack independently during concurrent downloads", async () => {
        const store = new SyncingStore();

        store.begin("pack-a");
        store.markProgress("pack-a", 10, "DOWNLOADING");
        store.begin("pack-b");
        store.markProgress("pack-b", 55, "IMPORTING");

        // Progress never jumps between packs.
        expect(store.progressOf("pack-a")).toBe(10);
        expect(store.progressOf("pack-b")).toBe(55);
        expect(store.isSyncing("pack-a")).toBe(true);
        expect(store.isSyncing("pack-b")).toBe(true);

        store.markProgress("pack-a", 40, "EXTRACTING");
        expect(store.progressOf("pack-a")).toBe(40);
        expect(store.progressOf("pack-b")).toBe(55);

        store.complete("pack-a");
        expect(store.statusOf("pack-a")).toBe("completed");
        expect(store.isSyncing("pack-a")).toBe(false);
        // pack-b is untouched by pack-a's completion.
        expect(store.statusOf("pack-b")).toBe("downloading");
        expect(store.isSyncing("pack-b")).toBe(true);
    });

    it("walks queued -> downloading -> completed through an SSE stream", async () => {
        const store = new SyncingStore();

        const outcome = await store.handleSyncStream(
            "pack-a",
            streamOf(
                event({ STARTED: { phase: "INITIALIZING", progress: 2, site_pack_id: "pack-a" } }),
                event({ IN_PROGRESS: { phase: "DOWNLOADING", progress: 47, site_pack_id: "pack-a" } }),
                event({ IN_PROGRESS: { phase: "IMPORTING", progress: 88, site_pack_id: "pack-a" } }),
                event({ COMPLETED: { phase: "COMPLETED", progress: 100, site_pack_id: "pack-a" } }),
            ),
        );

        expect(outcome).toBe("completed");
        expect(store.statusOf("pack-a")).toBe("completed");
        expect(store.progressOf("pack-a")).toBe(100);
        expect(store.isSyncing("pack-a")).toBe(false);
    });

    it("records failure with the engine error and keeps the row retryable", async () => {
        const store = new SyncingStore();

        const outcome = await store.handleSyncStream(
            "pack-a",
            streamOf(
                event({ IN_PROGRESS: { phase: "DOWNLOADING", progress: 30, site_pack_id: "pack-a" } }),
                event({ FAILED: { error: "connection reset" } }),
            ),
        );

        expect(outcome).toBe("failed");
        expect(store.statusOf("pack-a")).toBe("failed");
        expect(store.errorOf("pack-a")).toBe("connection reset");
        expect(store.isSyncing("pack-a")).toBe(false);
        // Progress from before the failure is retained for context.
        expect(store.progressOf("pack-a")).toBe(30);
    });

    it("lets a failed pack be retried: a new stream restarts from queued", async () => {
        const store = new SyncingStore();

        await store.handleSyncStream(
            "pack-a",
            streamOf(event({ FAILED: { error: "timeout" } })),
        );
        expect(store.statusOf("pack-a")).toBe("failed");

        const outcome = await store.handleSyncStream(
            "pack-a",
            streamOf(
                event({ IN_PROGRESS: { phase: "DOWNLOADING", progress: 20, site_pack_id: "pack-a" } }),
                event({ COMPLETED: { phase: "COMPLETED", progress: 100, site_pack_id: "pack-a" } }),
            ),
        );

        expect(outcome).toBe("completed");
        expect(store.statusOf("pack-a")).toBe("completed");
        expect(store.errorOf("pack-a")).toBeUndefined();
    });

    it("ignores a second stream while a pack is already downloading", async () => {
        const store = new SyncingStore();

        store.begin("pack-a");
        store.markProgress("pack-a", 20, "DOWNLOADING");

        const outcome = await store.handleSyncStream(
            "pack-a",
            streamOf(
                event({ IN_PROGRESS: { phase: "IMPORTING", progress: 99, site_pack_id: "pack-a" } }),
            ),
        );

        // The in-flight operation is not superseded by a second caller.
        expect(outcome).toBe("downloading");
        expect(store.progressOf("pack-a")).toBe(20);
    });

    it("handles a broken stream (non-FAILED error) as a failure", async () => {
        const store = new SyncingStore();

        const outcome = await store.handleSyncStream("pack-a", {
            async *[Symbol.asyncIterator]() {
                throw new Error("stream crashed");
                yield event({
                    IN_PROGRESS: { phase: "DOWNLOADING", progress: 5, site_pack_id: "pack-a" },
                });
            },
        });

        expect(outcome).toBe("failed");
        expect(store.statusOf("pack-a")).toBe("failed");
        expect(store.errorOf("pack-a")).toBe("stream crashed");
    });

    it("treats EOF mid-download (no terminal event) as a failure, never success", async () => {
        // Regression: the stream used to auto-complete when the connection
        // ended without COMPLETED/FAILED. A dropped connection or an
        // exhausted bounded retry must surface as a failure, not a success.
        const store = new SyncingStore();

        const outcome = await store.handleSyncStream(
            "pack-a",
            streamOf(
                event({ IN_PROGRESS: { phase: "DOWNLOADING", progress: 60, site_pack_id: "pack-a" } }),
                // stream ends here without a terminal event
            ),
        );

        expect(outcome).toBe("failed");
        expect(store.statusOf("pack-a")).toBe("failed");
        expect(store.isSyncing("pack-a")).toBe(false);
        expect(store.progressOf("pack-a")).toBe(60);
    });

    it("treats an empty stream (no events at all) as a failure, not a queued leak", async () => {
        // Regression: a stream that ended before its first event used to leave
        // the pack queued forever (isSyncing true, no retry possible).
        const store = new SyncingStore();

        const outcome = await store.handleSyncStream("pack-a", streamOf());

        expect(outcome).toBe("failed");
        expect(store.statusOf("pack-a")).toBe("failed");
        expect(store.isSyncing("pack-a")).toBe(false);
        expect(store.errorOf("pack-a")).toBeTruthy();
    });

    it("treats a stream that ends right after STARTED as a failure", async () => {
        const store = new SyncingStore();

        const outcome = await store.handleSyncStream(
            "pack-a",
            streamOf(
                event({ STARTED: { phase: "INITIALIZING", progress: 1, site_pack_id: "pack-a" } }),
            ),
        );

        expect(outcome).toBe("failed");
        expect(store.statusOf("pack-a")).toBe("failed");
        expect(store.isSyncing("pack-a")).toBe(false);
    });

    it("clear removes a terminal state entirely", () => {
        const store = new SyncingStore();
        store.begin("pack-a");
        store.complete("pack-a");
        expect(store.statusOf("pack-a")).toBe("completed");

        store.clear("pack-a");
        expect(store.statusOf("pack-a")).toBeUndefined();
        expect(store.progressOf("pack-a")).toBeUndefined();
    });
});

// Keep the entry type referenced so future shape changes are type-checked.
export type { SitePackSyncEntry };
