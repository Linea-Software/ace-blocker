import { getDashboardSnapshot, type NetworkBlockSnapshot } from "$engine-client";
import type { PageLoad } from "./$types";
import type {
    NetworkBlockSchema,
    NetworkListLockSchema,
    ScheduleEntrySchema,
} from "$engine-client";
import { engineStore } from "$lib/stores/engine.svelte";

export type NetworkBlockWithLock = NetworkBlockSchema & {
    lockInfo?: NetworkListLockSchema | null;
    schedules?: ScheduleEntrySchema[];
    sitesListCount?: number;
    rulesCount?: number;
};

export const load: PageLoad = async () => {
    // Ensure engineStore is ready (port fetched)
    if (!engineStore.isReady) {
        await engineStore.refresh();
    }

    const response = await getDashboardSnapshot();

    if (response.error) {
        console.error("Failed to fetch dashboard snapshot", response.error);
        // Engine may be down — surface an offline notice instead of an
        // empty state (same pattern as resistance-mode).
        return { blocks: [], engineOffline: !engineStore.isAlive, loadError: engineStore.isAlive };
    }

    const snapshots: NetworkBlockSnapshot[] = response.data?.blocks ?? [];

    const blocks: NetworkBlockWithLock[] = snapshots.map((snapshot) => ({
        ...snapshot.block,
        lockInfo: snapshot.lock ?? null,
        schedules: snapshot.scheduleEntries,
        sitesListCount: snapshot.sitePackConnectionCount,
        rulesCount: snapshot.userRulesCount,
    }));

    return { blocks, engineOffline: false, loadError: false };
};
