import { getDashboardSnapshot } from "$engine-client";
import {
    type EnforcementStatus,
    type NetworkBlockSchema,
    type NetworkBlockSnapshot,
} from "$engine-client";

class DashboardStore {
    isLoading = $state(true);
    loadError = $state(false);
    /** Today's aggregate blocked-attempts count (computed locally from block
     *  decisions; no usage or per-site data). */
    blockedAttempts = $state<number>(0);
    activeBlocksCount = $state<number>(0);
    totalBlocksCount = $state<number>(0);
    networkBlocks = $state<NetworkBlockSchema[]>([]);
    networkBlockSnapshots = $state<NetworkBlockSnapshot[]>([]);

    engineEnabled = $state(false);
    isLocked = $state(false);
    lockEndRaw = $state<string | null>(null);
    resistanceStatus = $state<EnforcementStatus>("disabled");

    constructor() {}

    async refresh() {
        this.loadError = false;
        // Single batch call replaces the previous dashboard polling.
        const response = await getDashboardSnapshot();

        if (response.error) {
            console.error("Dashboard Polling - Batch Error:", response.error);
            this.loadError = true;
            this.isLoading = false;
            return;
        }

        const snapshot = response.data;

        // Blocked-attempts aggregate (local, block decisions only)
        if (typeof snapshot?.blockedAttempts === "number") {
            this.blockedAttempts = snapshot.blockedAttempts;
        }

        // Network blocks
        if (snapshot?.blocks) {
            this.networkBlockSnapshots = snapshot.blocks;
            this.networkBlocks = snapshot.blocks.map((s) => s.block);
            this.totalBlocksCount = this.networkBlocks.length;
            this.activeBlocksCount = this.networkBlocks.filter(
                (b) => b.isActive,
            ).length;
        }

        // Resistance mode settings
        if (snapshot?.resistanceModeSettings) {
            const settings = snapshot.resistanceModeSettings;
            this.engineEnabled = settings.enabled;
            this.isLocked = settings.isLocked;
            this.lockEndRaw = settings.lockEnd || null;
            this.resistanceStatus = settings.enforcementStatus;
        }

        this.isLoading = false;
    }
}

export const dashboardStore = new DashboardStore();
