import {
    getDashboardSnapshot,
    type NetworkBlockSchema,
    type NetworkListLockSchema,
    type ScheduleEntrySchema,
    type NetworkBlockSnapshot,
} from "$engine-client";
import { engineStore } from "$lib/stores/engine";

export type NetworkBlockWithLock = NetworkBlockSchema & {
    lockInfo?: NetworkListLockSchema | null;
    schedules?: ScheduleEntrySchema[];
    sitesListCount?: number;
    rulesCount?: number;
};

export type NetworkBlocksLoad = {
    blocks: NetworkBlockWithLock[];
    engineOffline: boolean;
    loadError: boolean;
};

/** Solid-compatible route loader. It makes no assumptions about generated SvelteKit types. */
export async function loadNetworkBlocks(): Promise<NetworkBlocksLoad> {
    if (!engineStore.isReady) {
        await engineStore.refresh();
    }

    const response = await getDashboardSnapshot();
    if (response.error) {
        console.error("Failed to fetch dashboard snapshot", response.error);
        return {
            blocks: [],
            engineOffline: !engineStore.isAlive,
            loadError: engineStore.isAlive,
        };
    }

    const snapshots: NetworkBlockSnapshot[] = response.data?.blocks ?? [];
    return {
        blocks: snapshots.map((snapshot) => ({
            ...snapshot.block,
            lockInfo: snapshot.lock ?? null,
            schedules: snapshot.scheduleEntries,
            sitesListCount: snapshot.sitePackConnectionCount,
            rulesCount: snapshot.userRulesCount,
        })),
        engineOffline: false,
        loadError: false,
    };
}
