import { SvelteMap } from "svelte/reactivity";
import type { SitePackCreationStatus, SitePackImportPhase } from "$engine-client";

/**
 * Per-pack lifecycle status for site-pack downloads / resyncs.
 *
 * - `queued` — the user triggered the operation; no engine event yet.
 * - `downloading` — the engine reported STARTED/IN_PROGRESS progress.
 * - `completed` — the engine reported COMPLETED (progress 100).
 * - `failed` — the engine reported FAILED; the error text is kept so the UI
 *   can offer an explicit retry. Terminal states stay visible until cleared
 *   or superseded by a new operation for the same pack id.
 *
 * The store is keyed by pack id, so any number of packs can download at the
 * same time with fully independent state (no shared/global progress bar).
 */
export type SitePackSyncStatus = "queued" | "downloading" | "completed" | "failed";

export interface SitePackSyncEntry {
    status: SitePackSyncStatus;
    /** 0..100 */
    progress: number;
    phase?: SitePackImportPhase;
    error?: string;
}

/**
 * This is a store that tracks the progress of a sync operation for site
 * packs, keyed by pack id, with distinguishable queued / downloading /
 * completed / failed states. It can be used for any other per-item sync
 * operation.
 *
 * Usage:
 * import { syncStore } from "$lib/stores/syncing-store";
 *
 * async function checkSync() {
 *     while (syncStore.isSyncing("pack-id")) {
 *         console.log(syncStore.progressOf("pack-id"));
 *         await new Promise((resolve) => setTimeout(resolve, 100));
 *     }
 * }
 *
 * syncStore.begin("pack-id");
 * for (let i = 0; i <= 100; i++) {
 *     syncStore.markProgress("pack-id", i, "DOWNLOADING");
 *     console.log(syncStore.progressOf("pack-id"));
 * }
 * syncStore.complete("pack-id");
 *
 * syncStore.clear("pack-id");
 */
export class SyncingStore {
    private entries = new SvelteMap<string, SitePackSyncEntry>();

    public statusOf(id: string): SitePackSyncStatus | undefined {
        return this.entries.get(id)?.status;
    }

    public progressOf(id: string): number | undefined {
        return this.entries.get(id)?.progress;
    }

    public phaseOf(id: string): SitePackImportPhase | undefined {
        return this.entries.get(id)?.phase;
    }

    public errorOf(id: string): string | undefined {
        return this.entries.get(id)?.error;
    }

    /** True while the pack is queued or actively downloading. */
    public isSyncing(id: string): boolean {
        const status = this.entries.get(id)?.status;
        return status === "queued" || status === "downloading";
    }

    /** Alias kept for callers that reason about in-flight operations. */
    public isActive(id: string): boolean {
        return this.isSyncing(id);
    }

    public getProgress(id: string): number | undefined {
        return this.progressOf(id);
    }

    /** Start a new operation for a pack: queued at 0%. No-op while active. */
    public begin(id: string) {
        if (this.isSyncing(id)) return;
        this.entries.set(id, { status: "queued", progress: 0 });
    }

    /** Alias kept for existing callers; see {@link begin}. */
    public start(id: string) {
        this.begin(id);
    }

    /** Engine reported progress: mark downloading. */
    public markProgress(id: string, progress: number, phase?: SitePackImportPhase) {
        const entry = this.entries.get(id) ?? { status: "queued" as const, progress: 0 };
        this.entries.set(id, {
            status: "downloading",
            progress: Math.max(0, Math.min(100, progress)),
            phase: phase ?? entry.phase,
        });
    }

    /** Alias kept for existing callers; see {@link markProgress}. */
    public setProgress(id: string, progress: number) {
        this.markProgress(id, progress);
    }

    /** Engine reported completion: 100% and a terminal completed state. */
    public complete(id: string) {
        const entry = this.entries.get(id) ?? { status: "queued" as const, progress: 0 };
        this.entries.set(id, { status: "completed", progress: 100, phase: entry.phase });
    }

    /** Engine reported failure: keep the error for the retry affordance. */
    public fail(id: string, error: string) {
        const entry = this.entries.get(id) ?? { status: "queued" as const, progress: 0 };
        this.entries.set(id, { status: "failed", progress: entry.progress, error });
    }

    /** Remove the entry entirely (terminal states included). */
    public clear(id: string) {
        this.entries.delete(id);
    }

    /** Alias kept for existing callers; see {@link clear}. */
    public remove(id: string) {
        this.clear(id);
    }

    /**
     * Consumes an SSE stream for a site-pack download/resync and drives the
     * per-pack state machine:
     *
     *  - STARTED / IN_PROGRESS → downloading with progress + phase;
     *  - COMPLETED → completed (100);
     *  - FAILED → failed with the engine error.
     *
     * Resolves with the final status ("completed" | "failed"); never throws.
     * Terminal states persist until {@link clear} is called or the same pack
     * starts a new operation.
     *
     * A stream that ends *without* a terminal COMPLETED/FAILED event is
     * treated as a failure — never as a silent success. This covers the
     * bounded-retry case (transport gave up after `sseMaxRetryAttempts`) and
     * a dropped connection mid-stream: both leave the operation in a state
     * where the engine's terminal event never arrived, and auto-completing
     * would hide a failed download as if it had succeeded.
     */
    public async handleSyncStream(
        id: string,
        stream: AsyncIterable<SitePackCreationStatus>,
    ): Promise<SitePackSyncStatus> {
        if (this.isSyncing(id)) return this.statusOf(id) ?? "queued";

        try {
            this.begin(id);
            for await (const status of stream) {
                if ("STARTED" in status) {
                    this.markProgress(id, status.STARTED.progress, status.STARTED.phase);
                } else if ("IN_PROGRESS" in status) {
                    this.markProgress(id, status.IN_PROGRESS.progress, status.IN_PROGRESS.phase);
                } else if ("COMPLETED" in status) {
                    this.complete(id);
                    return "completed";
                } else if ("FAILED" in status) {
                    this.fail(id, status.FAILED.error);
                    return "failed";
                }
            }
            // Stream ended without a terminal event: the operation did not
            // reach a defined engine outcome (bounded retries exhausted,
            // connection dropped, or the stream closed early). Surface it as a
            // failure so the loading state always clears and the UI can offer
            // an explicit retry.
            this.fail(
                id,
                "The connection to the engine closed before the sync completed.",
            );
            return "failed";
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            this.fail(id, message);
            return "failed";
        }
    }
}

export const syncStore = new SyncingStore();
