import { beforeEach, describe, expect, it, vi } from "vitest";
import {
    createNetworkBlock,
    renameNetworkBlock,
    setNetworkBlockIcon,
    setNetworkBlockSafeSearch,
    type NetworkBlockSchema,
} from "$engine-client";
import { saveNetworkBlock, type BlockSaveDraft } from "./_save-network-block";

vi.mock("$engine-client", () => ({
    createNetworkBlock: vi.fn(),
    renameNetworkBlock: vi.fn(),
    setNetworkBlockIcon: vi.fn(),
    setNetworkBlockSafeSearch: vi.fn(),
}));

const block = {
    id: "block-1",
    name: "Previous",
    icon: null,
    safeSearch: false,
} as NetworkBlockSchema;

const delta = (dirty = false) => ({
    changes: dirty ? [{}] : [],
    pushChangesToServer: vi.fn().mockResolvedValue(undefined),
});

function draft(override: Partial<BlockSaveDraft> = {}): BlockSaveDraft {
    return {
        block,
        name: block.name,
        icon: block.icon ?? null,
        safeSearch: block.safeSearch,
        rules: delta(),
        schedule: delta(),
        packs: delta(),
        ...override,
    };
}

beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(createNetworkBlock).mockResolvedValue({
        data: { networkBlock: { ...block, id: "new-block" } },
    } as never);
    vi.mocked(renameNetworkBlock).mockResolvedValue({ data: {} } as never);
    vi.mocked(setNetworkBlockIcon).mockResolvedValue({ data: {} } as never);
    vi.mocked(setNetworkBlockSafeSearch).mockResolvedValue({ data: {} } as never);
});

describe("Network Block save orchestration", () => {
    it("does not acknowledge an icon RPC error or run later mutations", async () => {
        const rules = delta(true);
        vi.mocked(setNetworkBlockIcon).mockResolvedValue({
            error: { code: "permission_denied", status: 403, error: "locked" },
        } as never);
        const result = await saveNetworkBlock(draft({ icon: "shield", rules }));
        expect(result).toMatchObject({ kind: "failed", id: "block-1", attempted: true });
        expect(rules.pushChangesToServer).not.toHaveBeenCalled();
        expect(setNetworkBlockSafeSearch).not.toHaveBeenCalled();
    });

    it("does not acknowledge a safe-search RPC error", async () => {
        vi.mocked(setNetworkBlockSafeSearch).mockResolvedValue({
            error: { code: "validation_failed", status: 400, error: "invalid" },
        } as never);
        const result = await saveNetworkBlock(draft({ safeSearch: true }));
        expect(result.kind).toBe("failed");
    });

    it("stops after the first failing delta, even if an earlier group committed", async () => {
        const rules = delta(true);
        const schedule = delta(true);
        const packs = delta(true);
        schedule.pushChangesToServer.mockRejectedValue(new Error("failed after prior commit"));
        const result = await saveNetworkBlock(draft({ rules, schedule, packs }));
        expect(result).toMatchObject({ kind: "failed", attempted: true });
        expect(rules.pushChangesToServer).toHaveBeenCalledExactlyOnceWith("block-1");
        expect(packs.pushChangesToServer).not.toHaveBeenCalled();
        expect(setNetworkBlockSafeSearch).not.toHaveBeenCalled();
    });

    it("reports ambiguous create failure without pretending no writes occurred", async () => {
        vi.mocked(createNetworkBlock).mockRejectedValue(new Error("connection lost"));
        const result = await saveNetworkBlock(draft({ block: null }));
        expect(result).toMatchObject({ kind: "failed", attempted: true });
        expect(setNetworkBlockSafeSearch).not.toHaveBeenCalled();
    });

    it("saves each independent group in order and acknowledges only full success", async () => {
        const calls: string[] = [];
        const rules = delta(true);
        const schedule = delta(true);
        const packs = delta(true);
        rules.pushChangesToServer.mockImplementation(async () => { calls.push("rules"); });
        schedule.pushChangesToServer.mockImplementation(async () => { calls.push("schedule"); });
        packs.pushChangesToServer.mockImplementation(async () => { calls.push("packs"); });
        vi.mocked(setNetworkBlockSafeSearch).mockImplementation(async () => {
            calls.push("safe");
            return { data: {} } as never;
        });
        const result = await saveNetworkBlock(
            draft({ block: null, rules, schedule, packs }),
        );
        expect(result).toEqual({ kind: "saved", id: "new-block" });
        expect(calls).toEqual(["rules", "schedule", "packs", "safe"]);
    });
});
