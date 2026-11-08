import type { PageLoad } from "./$types";
import { getDashboardSnapshot } from "$engine-client";
import { engineStore } from "$lib/stores/engine.svelte";

export const load: PageLoad = async ({ url }) => {
    const id = url.searchParams.get("block");
    if (!engineStore.isReady) await engineStore.refresh();
    const response = await getDashboardSnapshot();
    if (response.error) {
        return { block: null, requestedId: id, engineOffline: !engineStore.isAlive, loadError: true };
    }
    const snapshot = id
        ? response.data?.blocks.find((item) => item.block.id === id)
        : null;
    const block = snapshot ? {
        ...snapshot.block,
        lockInfo: snapshot.lock ?? null,
        schedules: snapshot.scheduleEntries,
        sitesListCount: snapshot.sitePackConnectionCount,
        rulesCount: snapshot.userRulesCount,
    } : null;
    return { block, requestedId: id, engineOffline: false, loadError: false };
};
