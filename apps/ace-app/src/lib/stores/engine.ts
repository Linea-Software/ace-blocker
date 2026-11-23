import { reactiveFields } from "./reactive-fields";
import { isTauri } from "@tauri-apps/api/core";
import { ping } from "$engine-client";

/** Engine liveness is the only state kept here; port and token discovery
 * are not web-facing concerns after the typed IPC migration.
 */
class EngineStore {
    isReady = false;
    isAlive = false;

    constructor() {
        reactiveFields(this, ["isReady", "isAlive"]);
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
            // Browser preview is backed by read-only fixture RPCs, not a real engine.
            this.isAlive = true;
            return true;
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
