/**
 * Deterministic presentation fixtures for the *non-Tauri* browser preview.
 * No data is read from a real profile and no mutation is ever persisted.
 * The native adapter must never call this module when Tauri is present.
 */
import catalog from "../data/site-packs.json";
import type { RpcOperation } from "./rpc-parameters";
import type {
    AccountInfoResponse,
    DashboardSnapshotResponse,
    NetworkBlockSchema,
    NetworkBlockSnapshot,
    ResistanceModeSettings,
    ScheduleEntrySchema,
    ScriptInfo,
    SitePackSchema,
    UserDefinedRuleSchema,
} from "./types";

const unlockState = { state: "unlocked" };
const blocks: NetworkBlockSchema[] = [
    {
        id: "demo-adult", name: "Adult websites", icon: "shield",
        enabled: true, isActive: true, isLocked: false,
        lockState: unlockState, scheduleMode: "always", safeSearch: true,
    },
    {
        id: "demo-social", name: "Social media", icon: "social",
        enabled: true, isActive: true, isLocked: false,
        lockState: unlockState, scheduleMode: "always", safeSearch: false,
    },
    {
        id: "demo-focus", name: "Focus hours", icon: "briefcase",
        enabled: true, isActive: false, isLocked: false,
        lockState: unlockState, scheduleMode: "scheduled", safeSearch: false,
    },
];

const schedule: ScheduleEntrySchema[] = [{
    id: "demo-schedule", networkBlockId: "demo-focus",
    startTime: "09:00", endTime: "17:00",
    monday: true, tuesday: true, wednesday: true,
    thursday: true, friday: true, saturday: false, sunday: false,
}];

const rules: UserDefinedRuleSchema[] = [
    { id: "demo-rule-1", networkBlockId: "demo-social", rule: "reddit.com", ruleType: "domain" },
    { id: "demo-rule-2", networkBlockId: "demo-focus", rule: "news.ycombinator.com", ruleType: "domain" },
];

const packIds: Record<string, string[]> = {
    "demo-adult": ["pack-adult-free"],
    "demo-social": ["pack-social-free"],
    "demo-focus": ["pack-gaming-free"],
};
const connections = Object.entries(packIds).flatMap(([networkBlockId, ids]) =>
    ids.map((sitePackId) => ({ networkBlockId, sitePackId })),
);

const sitePacks: SitePackSchema[] = catalog.map((item) => ({
    id: item.id, name: item.name, description: item.description,
    icon: item.icon, entriesCount: item.domains_count,
    requiresPro: item.requires_pro,
    isRemote: true, isSynced: true, isBundled: false, isLocked: false,
    lastUpdatedAt: "2026-11-20T09:00:00Z", version: "2026.11.20",
}));
sitePacks.push({
    id: "demo-custom", name: "My custom list",
    description: "A sample personal website list", entriesCount: 3,
    icon: "globe", requiresPro: false, isRemote: false,
    isSynced: true, isBundled: false, isLocked: false,
    lastUpdatedAt: "2026-11-20T09:00:00Z", version: "1",
});

const scripts: ScriptInfo[] = [
    { id: "reddit_block_recomandations", domain: "reddit.com", name: "Hide infinite feed", description: "Remove the endless homepage feed", isEnabled: true, requiresPro: false },
    { id: "youtube_shorts_remove", domain: "youtube.com", name: "Hide short videos", description: "Hide short-form video recommendations", isEnabled: true, requiresPro: false },
    { id: "demo-grayscale", domain: "instagram.com", name: "Reduce visual stimulation", description: "Desaturate vivid media", isEnabled: false, requiresPro: true },
    { id: "demo-delay_navigation", domain: "reddit.com", name: "Delay navigation", description: "Add a pause before opening new posts", isEnabled: true, requiresPro: true },
    { id: "demo-captcha", domain: "youtube.com", name: "Reflection challenge", description: "Require a challenge before continuing", isEnabled: false, requiresPro: true },
];

const resistance: ResistanceModeSettings = {
    enabled: true, enforcementStatus: "operational",
    isLocked: false, lockEnd: null, lockState: unlockState,
    proEligible: true, scriptsEnabled: 3, scriptsLoaded: true,
    scriptsTotal: scripts.length, scriptsVersion: "2026.11.20",
};

const snapshots: NetworkBlockSnapshot[] = blocks.map((block) => ({
    block, scheduleEntries: block.id === "demo-focus" ? schedule : [],
    sitePackConnectionCount: packIds[block.id]?.length ?? 0,
    userRulesCount: rules.filter((rule) => rule.networkBlockId === block.id).length,
}));
const dashboard: DashboardSnapshotResponse = {
    blockedAttempts: 248, blocks: snapshots, resistanceModeSettings: resistance,
};
const account: AccountInfoResponse = { isPro: true, tosAccepted: true };

/**
 * Read-only RPC simulator. Returns undefined for unsupported operations.
 * Always returns fresh copies to prevent UI code mutating shared fixtures.
 */
export function browserDemoResponse(
    operation: RpcOperation,
    args: Record<string, unknown>,
): unknown | undefined {
    const blockId = String(args.network_block_id ?? "");
    const sitePackId = String(args.site_pack_id ?? "");
    let value: unknown;
    switch (operation) {
        case "ping": value = null; break;
        case "getAccountInfo":
        case "refreshAccount": value = account; break;
        case "getDashboardSnapshot": value = dashboard; break;
        case "getAllNetworkBlocks": value = { networkBlocks: blocks }; break;
        case "getAllSitePacks": value = { sitePacks }; break;
        case "getAllScripts": value = { scripts }; break;
        case "getResistanceModeSettings": value = resistance; break;
        case "getScheduleEntries": value = {
            scheduleEntries: schedule.filter((row) => row.networkBlockId === blockId),
        }; break;
        case "getUserRules": value = {
            userDefinedRules: rules.filter((row) => row.networkBlockId === blockId),
        }; break;
        case "getNetBlockToSitePackConnections": value = {
            connections: connections.filter((row) => row.networkBlockId === blockId),
        }; break;
        case "getLock": value = { lock: {
            networkBlockId: blockId, isActive: false, lockState: unlockState,
        } }; break;
        case "getSitePackSources": value = {
            sources: catalog.find((item) => item.id === sitePackId)?.sources.map((item, i) => ({
                id: `demo-source-${i}`, sitePackId,
                source: item.source,
                sourceType: item.source_type, format: item.format,
            })) ?? [],
        }; break;
        case "getScript": value = {
            script: scripts.find((script) => script.id === String(args.script_id ?? "")),
        }; break;
        case "getVersion": value = { version: "0.1.0-demo", build: "browser" }; break;
        case "getUpdateCheckInterval": value = { intervalSeconds: 86400 }; break;
        case "getDebugProPolicy": value = { policy: "Default" }; break;
        default: return undefined;
    }
    return value === null ? null : structuredClone(value);
}
