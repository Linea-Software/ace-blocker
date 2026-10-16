import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "@tauri-apps/api/core";
import { client } from "$engine-client/client.gen";
import { ping } from "$engine-client";
import { dev } from "$app/environment";

class EngineStore {
    port = $state<number>(10567);
    isReady = $state<boolean>(false);
    isAlive = $state<boolean>(false);

    constructor() {
        this.init();
    }

    private async init() {
        // Try to get the port immediately
        await this.refresh();
        this.isReady = true;

        // Periodic refresh of engine status
        if (typeof window !== "undefined") {
            setInterval(() => this.refresh(), 10000);
        }
    }

    /**
     * Refreshes the engine port and status.
     * Also updates the global API client configuration.
     */
    async refresh() {
        if (this.isTauri && !dev) {
            try {
                // Try to get the dynamic port from Tauri command
                const port = await invoke<number>("get_engine_port");
                if (port) {
                    this.port = port;
                } else {
                    this.port = 10567;
                }
            } catch (error) {
                // In development, it's normal for invoke to fail if running in a pure browser
                console.error(
                    "Failed to get engine port via tauri invoke:",
                    error,
                );
                this.port = 10567;
            }
        } else {
            this.port = 10567;
        }

        // State-changing engine requests need the per-install capability
        // token (see ace-engine/docs/local-api-protection-design.md). In a
        // plain-browser dev run there is no Tauri IPC to read it; the debug
        // engine accepts token-less requests.
        let capability: string | null = null;
        if (this.isTauri) {
            try {
                capability = await invoke<string>("get_engine_capability");
            } catch (error) {
                console.error(
                    "Failed to get engine capability token via tauri invoke:",
                    error,
                );
                capability = null;
            }
        }

        // Automatically update the engine client configuration
        client.setConfig({
            baseUrl: `https://127.0.0.1:${this.port}`,
            headers: capability ? { "X-Ace-Capability": capability } : {},
        });

        const response = await ping();
        this.isAlive = !response.error;

        return this.port;
    }

    /**
     * Helper to check if running inside Tauri
     */
    get isTauri() {
        return typeof window !== "undefined" && isTauri();
    }
}

export const engineStore = new EngineStore();

/**
 * Gets the engine local port.
 * @deprecated Use engineStore.port instead, or wait for engineStore.isReady.
 */
export async function getEnginePort(): Promise<number> {
    if (!engineStore.isReady) {
        return await engineStore.refresh();
    }
    return engineStore.port;
}
