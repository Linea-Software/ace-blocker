import { getDashboardSnapshot } from "$engine-client";
import { engineStore } from "$lib/stores/engine";
import type { NetworkBlockWithLock } from "../_data";

export interface BlockEditorLoad {
    block: NetworkBlockWithLock | null;
    requestedId: string | null;
    engineOffline: boolean;
    loadError: boolean;
}

/** Fetches canonical editor state without SvelteKit's generated PageLoad/$types. */
export async function loadBlockEditor(
    requestedId: string | null,
): Promise<BlockEditorLoad> {
    if (!engineStore.isReady) await engineStore.refresh();
    const response = await getDashboardSnapshot();
    if (response.error) {
        return {
            block: null,
            requestedId,
            engineOffline: !engineStore.isAlive,
            loadError: true,
        };
    }
    const snapshot = requestedId
        ? response.data?.blocks.find((item) => item.block.id === requestedId)
        : null;
    const block = snapshot
        ? {
              ...snapshot.block,
              lockInfo: snapshot.lock ?? null,
              schedules: snapshot.scheduleEntries,
              sitesListCount: snapshot.sitePackConnectionCount,
              rulesCount: snapshot.userRulesCount,
          }
        : null;
    return { block, requestedId, engineOffline: false, loadError: false };
}
