import { isTauri } from "@tauri-apps/api/core";
import { ping } from "$engine-client";

/** Engine liveness is the only state kept here; port and token discovery
 * are not web-facing concerns after the typed IPC migration.
 */
class EngineStore {
    isReady = $state(false);
    isAlive = $state(false);

    constructor() {
        void this.init();
    }

    private async init() {
        await this.refresh();
        this.isReady = true;
        if (typeof window !== "undefined") {
            setInterval(() => void this.refresh(), 10000);
        }
    }

    async refresh(): Promise<boolean> {
        if (!this.isTauri) {
            this.isAlive = false;
            return false;
        }
        try {
            const response = await ping();
            this.isAlive = !response.error;
        } catch {
            this.isAlive = false;
        }
        return this.isAlive;
    }

    get isTauri() {
        return typeof window !== "undefined" && isTauri();
    }
}

export const engineStore = new EngineStore();
