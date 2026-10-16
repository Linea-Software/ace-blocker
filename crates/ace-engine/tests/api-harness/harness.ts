/**
 * Ace Engine API Test Harness
 *
 * Lightweight test utility for exercising the ace-engine REST API.
 * Designed to be extended by LLMs when testing new features.
 *
 * ## Quick start
 *
 * ```bash
 * cd ace-engine/tests/api-harness
 * bun install
 * bun run harness.ts          # run all smoke tests
 * bun run harness.ts ping     # health check only
 * ```
 *
 * ## How LLMs use this
 *
 * 1. Read `ace-engine/docs/openapi.json` for the full API surface.
 * 2. Extend this harness with test scenarios for new endpoints.
 * 3. Run against a local engine: `bun run harness.ts`
 *
 * ## Port discovery
 *
 * The harness auto-discovers the engine port:
 * - Debug builds: always port 10567
 * - Release builds: reads `port.lock` from the engine's app-data directory
 * - Override: set `ACE_ENGINE_PORT` environment variable
 */

// ---------------------------------------------------------------------------
// Types (mirrors the OpenAPI schema — extend as needed)
// ---------------------------------------------------------------------------

interface NetworkBlock {
    id: string;
    name: string;
    isActive: boolean;
    isLocked: boolean;
    lockState: string;
    safeSearch: boolean;
    icon?: string | null;
}

interface GetAllNetworkBlocksResponse {
    networkBlocks: NetworkBlock[];
}

interface CreateNetworkBlockRequest {
    blockName: string;
    icon?: string | null;
}

interface CreateNetworkBlockResponse {
    networkBlock: NetworkBlock;
}

interface VersionInfo {
    version: string;
    build: string;
}

/** Dashboard snapshot shape (only the fields the smoke test reads). */
interface DashboardSnapshotResponse {
    blockedAttempts: number;
}

// ---------------------------------------------------------------------------
// Port discovery
// ---------------------------------------------------------------------------

/**
 * Discover the engine's API port.
 *
 * Priority:
 *   1. ACE_ENGINE_PORT env var
 *   2. Debug default (10567)
 *   3. port.lock file in the engine's app-data dir
 */
function discoverPort(): number {
    // 1. Environment override
    const envPort = process.env.ACE_ENGINE_PORT;
    if (envPort) {
        const parsed = parseInt(envPort, 10);
        if (!isNaN(parsed)) return parsed;
    }

    // 2. Debug default
    const isDebug = process.env.ACE_DEBUG !== "0";
    if (isDebug) return 10567;

    // 3. Read port.lock from engine's app-data dir
    try {
        const os = require("node:os") as typeof import("node:os");
        const fs = require("node:fs") as typeof import("node:fs");
        const path = require("node:path") as typeof import("node:path");

        const platform = os.platform();
        let appDataRoot: string;

        if (platform === "win32") {
            appDataRoot = path.join(os.homedir(), "AppData", "Roaming", "com.aceblocker.ace-engine");
        } else if (platform === "darwin") {
            appDataRoot = path.join(os.homedir(), "Library", "Application Support", "com.aceblocker.ace-engine");
        } else {
            appDataRoot = path.join(os.homedir(), ".local", "share", "com.aceblocker.ace-engine");
        }

        const portFile = path.join(appDataRoot, "port.lock");
        if (fs.existsSync(portFile)) {
            const portStr = fs.readFileSync(portFile, "utf-8").trim();
            const port = parseInt(portStr, 10);
            if (!isNaN(port)) return port;
        }
    } catch {
        // Fall through to default
    }

    return 10567;
}

// ---------------------------------------------------------------------------
// API client
// ---------------------------------------------------------------------------

const PORT = discoverPort();
const BASE_URL = `https://127.0.0.1:${PORT}`;

/** All API calls go through this. Accepts self-signed certs. */
async function api<T = unknown>(
    method: string,
    path: string,
    body?: unknown,
): Promise<{ data?: T; error?: string; status: number }> {
    const url = `${BASE_URL}${path}`;
    const options: RequestInit = {
        method,
        headers: body ? { "Content-Type": "application/json" } : undefined,
        body: body ? JSON.stringify(body) : undefined,
    };

    try {
        const response = await fetch(url, { ...options, tls: { rejectUnauthorized: false } as any });
        const text = await response.text();

        if (!response.ok) {
            let errorMsg = `HTTP ${response.status}`;
            try {
                const json = JSON.parse(text);
                errorMsg = json.error ?? errorMsg;
            } catch { /* use raw text */ }
            return { error: errorMsg, status: response.status };
        }

        if (!text) return { status: response.status };
        try {
            const json = JSON.parse(text);
            return { data: json as T, status: response.status };
        } catch {
            // Plain text response (e.g. /ping returns "pong")
            return { data: text as T, status: response.status };
        }
    } catch (err: any) {
        return { error: `Connection failed: ${err.message}`, status: 0 };
    }
}

// ---------------------------------------------------------------------------
// Convenience wrappers (extend these as you add features)
// ---------------------------------------------------------------------------

const apiClient = {
    /** GET convenience */
    get<T>(path: string) { return api<T>("GET", path); },
    /** POST convenience */
    post<T>(path: string, body?: unknown) { return api<T>("POST", path, body); },
    /** DELETE convenience */
    del<T>(path: string) { return api<T>("DELETE", path); },
};

// ---------------------------------------------------------------------------
// Test scenarios
// ---------------------------------------------------------------------------

/** Smoke test: check the engine is alive. */
async function testPing(): Promise<boolean> {
    console.log(`\n🔍 Ping test (port ${PORT})...`);
    const { data, error, status } = await apiClient.get<string>("/ping");
    if (error) {
        console.error(`  ❌ FAILED: ${error}`);
        return false;
    }
    console.log(`  ✅ pong (${status})`);
    return true;
}

/** Smoke test: basic CRUD on network blocks. */
async function testNetworkBlocksCRUD(): Promise<boolean> {
    console.log("\n🔍 Network blocks CRUD test...");

    // 1. List existing blocks
    const { data: existing, error: listErr } = await apiClient.get<GetAllNetworkBlocksResponse>("/network-blocks");
    if (listErr) { console.error(`  ❌ GET failed: ${listErr}`); return false; }
    const countBefore = existing!.networkBlocks.length;
    console.log(`  📋 ${countBefore} existing blocks`);

    // 2. Create a test block
    const { data: created, error: createErr } = await apiClient.post<CreateNetworkBlockResponse>(
        "/network-blocks",
        { blockName: "llm-harness-test" } satisfies CreateNetworkBlockRequest,
    );
    if (createErr) { console.error(`  ❌ POST failed: ${createErr}`); return false; }
    console.log(`  📝 Created: ${created!.networkBlock.name} (${created!.networkBlock.id})`);

    // 3. Verify count increased
    const { data: after, error: listErr2 } = await apiClient.get<GetAllNetworkBlocksResponse>("/network-blocks");
    if (listErr2) { console.error(`  ❌ GET failed: ${listErr2}`); return false; }
    if (after!.networkBlocks.length !== countBefore + 1) {
        console.error(`  ❌ Expected ${countBefore + 1} blocks, got ${after!.networkBlocks.length}`);
        return false;
    }
    console.log(`  ✅ Count increased to ${after!.networkBlocks.length}`);

    // 4. Cleanup: delete the test block
    const { error: delErr } = await apiClient.del(`/network-blocks/${created!.networkBlock.id}`);
    if (delErr) { console.error(`  ❌ DELETE failed: ${delErr}`); return false; }

    // 5. Verify count decreased
    const { data: final, error: listErr3 } = await apiClient.get<GetAllNetworkBlocksResponse>("/network-blocks");
    if (listErr3) { console.error(`  ❌ GET failed: ${listErr3}`); return false; }
    if (final!.networkBlocks.length !== countBefore) {
        console.error(`  ❌ Expected ${countBefore} blocks, got ${final!.networkBlocks.length}`);
        return false;
    }
    console.log("  ✅ CRUD cycle complete, no side effects");
    return true;
}

/** Smoke test: system endpoints. */
async function testSystem(): Promise<boolean> {
    console.log("\n🔍 System endpoints test...");

    const { data: version, error: verErr } = await apiClient.get<VersionInfo>("/system/version");
    if (verErr) { console.error(`  ❌ Version failed: ${verErr}`); return false; }
    console.log(`  📦 Engine v${version!.version} (${version!.build})`);

    // Usage reporting endpoints were removed; the dashboard snapshot now
    // carries a local aggregate blocked-attempts counter instead.
    const { data: dashboard, error: dashErr } = await apiClient.get<DashboardSnapshotResponse>(
        "/network-blocks/dashboard",
    );
    if (dashErr) { console.error(`  ❌ Dashboard snapshot failed: ${dashErr}`); return false; }
    if (typeof dashboard!.blockedAttempts !== "number") {
        console.error("  ❌ Dashboard snapshot missing blockedAttempts counter");
        return false;
    }
    console.log(`  ✅ System endpoints healthy (blocked attempts today: ${dashboard!.blockedAttempts})`);

    return true;
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

async function main() {
    console.log(`🚀 Ace Engine API Harness — targeting ${BASE_URL}`);
    console.log("=" .repeat(50));

    const results = await Promise.all([
        testPing(),
        testNetworkBlocksCRUD(),
        testSystem(),
    ]);

    const passed = results.filter(Boolean).length;
    const total = results.length;

    console.log(`\n${"=" .repeat(50)}`);
    console.log(`Results: ${passed}/${total} passed`);

    if (passed < total) {
        console.error("❌ Some tests failed — check the engine is running and accessible.");
        process.exit(1);
    }

    console.log("✅ All smoke tests passed.\n");
}

main();
