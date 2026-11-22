import { reactiveFields } from "./reactive-fields";
import { getDashboardSnapshot } from "$engine-client";
import {
    type EnforcementStatus,
    type NetworkBlockSchema,
    type NetworkBlockSnapshot,
} from "$engine-client";

class DashboardStore {
    isLoading = (true);
    loadError = (false);
    /** Today's aggregate blocked-attempts count (computed locally from block
     *  decisions; no usage or per-site data). */
    blockedAttempts: number = (0);
    activeBlocksCount = (0);
    totalBlocksCount = (0);
    networkBlocks: NetworkBlockSchema[] = ([]);
    networkBlockSnapshots: NetworkBlockSnapshot[] = ([]);

    engineEnabled = (false);
    isLocked = (false);
    lockEndRaw: string | null = (null);
    resistanceStatus: EnforcementStatus = ("disabled");

    constructor() {
        reactiveFields(this, ["isLoading", "loadError", "blockedAttempts", "activeBlocksCount", "totalBlocksCount", "networkBlocks", "networkBlockSnapshots", "engineEnabled", "isLocked", "lockEndRaw", "resistanceStatus"]);}

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
