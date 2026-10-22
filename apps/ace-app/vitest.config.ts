import { defineConfig } from "vitest/config";
import { resolve } from "node:path";

/**
 * Vitest is scoped to pure unit tests under `src/` (e.g.
 * `src/lib/utils/pack-status.test.ts`, `src/lib/stores/syncing-store.test.ts`).
 * The Playwright e2e specs in `e2e/` are deliberately excluded — they run
 * through `bun run test:e2e`.
 *
 * A dedicated config is used instead of vite.config.js so the SvelteKit /
 * Paraglide / Tailwind plugins never load for unit tests.
 */
export default defineConfig({
    resolve: {
        alias: {
            "$lib": resolve(import.meta.dirname, "src/lib"),
            "$engine-client": resolve(import.meta.dirname, "src/lib/engine-client"),
        },
    },
    test: {
        include: ["src/**/*.test.ts"],
        environment: "node",
    },
});
