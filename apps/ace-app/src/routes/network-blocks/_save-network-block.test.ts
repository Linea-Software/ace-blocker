import { beforeEach, describe, expect, it, vi } from "vitest";
import { saveNetworkBlock as saveNetworkBlockRpc, type NetworkBlockSchema } from "$engine-client";
import { saveNetworkBlock, type BlockSaveDraft } from "./_save-network-block";

vi.mock("$engine-client", () => ({
    saveNetworkBlock: vi.fn(),
}));

const block = {
    id: "block-1",
    name: "Original",
    icon: null,
    safeSearch: false,
} as NetworkBlockSchema;

function draft(override: Partial<BlockSaveDraft> = {}): BlockSaveDraft {
    return {
        block,
        name: "New name",
        icon: "shield",
        safeSearch: true,
        rules: {
            currentRules: [{ rule: "example.com", ruleType: "domain" }],
        },
        schedule: {
            currentScheduleEntries: [{
                startTime: "09:00",
                endTime: "17:00",
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            }],
        },
        packs: { currentConnectedListIds: ["pack-social-free"] },
        ...override,
    };
}

beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(saveNetworkBlockRpc).mockResolvedValue({
        data: { networkBlock: { ...block, name: "New name" } },
    } as never);
});

describe("Network Block atomic save", () => {
    it("sends one complete editor draft in one RPC", async () => {
        const result = await saveNetworkBlock(draft());
        expect(result).toMatchObject({ kind: "saved", block: { id: "block-1", name: "New name" } });
        expect(saveNetworkBlockRpc).toHaveBeenCalledExactlyOnceWith({
            body: {
                id: "block-1",
                name: "New name",
                icon: "shield",
                safeSearch: true,
                rules: [{ rule: "example.com", ruleType: "domain" }],
                scheduleEntries: [{
                    startTime: "09:00",
                    endTime: "17:00",
                    monday: true,
                    tuesday: false,
                    wednesday: false,
                    thursday: false,
                    friday: false,
                    saturday: false,
                    sunday: false,
                }],
                sitePackIds: ["pack-social-free"],
            },
        });
    });

    it("creates a block using the same atomic call", async () => {
        await saveNetworkBlock(draft({ block: null }));
        expect(saveNetworkBlockRpc).toHaveBeenCalledOnce();
        expect(vi.mocked(saveNetworkBlockRpc).mock.calls[0][0].body.id).toBe(null);
    });

    it("never calls a second operation after a rejected transaction", async () => {
        vi.mocked(saveNetworkBlockRpc).mockResolvedValue({
            error: { code: "forbidden", status: 403, error: "locked" },
        } as never);
        expect(await saveNetworkBlock(draft())).toMatchObject({
            kind: "failed",
            uncertain: false,
        });
        expect(saveNetworkBlockRpc).toHaveBeenCalledOnce();
    });

    it("marks transport loss as uncertain: a COMMIT may have succeeded", async () => {
        vi.mocked(saveNetworkBlockRpc).mockResolvedValue({
            error: { code: "transport_unavailable", status: 503, error: "connection lost" },
        } as never);
        expect(await saveNetworkBlock(draft())).toMatchObject({
            kind: "failed", uncertain: true,
        });
    });

    it("does not acknowledge malformed or absent save responses", async () => {
        vi.mocked(saveNetworkBlockRpc).mockResolvedValue({ data: {} } as never);
        expect(await saveNetworkBlock(draft())).toMatchObject({
            kind: "failed", uncertain: true,
        });
    });
});
