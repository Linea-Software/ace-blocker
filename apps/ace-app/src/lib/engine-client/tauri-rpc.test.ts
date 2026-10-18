import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { rpcRequest, rpcStream } from "./tauri-rpc";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
    Channel: class {
        onmessage?: (bytes: number[]) => void;
    },
}));

afterEach(() => vi.resetAllMocks());

describe("typed desktop engine RPC adapter", () => {
    it("forwards a body as named RPC payload, without an HTTP URL", async () => {
        vi.mocked(invoke).mockResolvedValue({ networkBlock: { id: "block-id" } });

        const response = await rpcRequest("createNetworkBlock", {
            body: { blockName: "Work", icon: null },
        });

        expect(invoke).toHaveBeenCalledWith("engine_call", {
            op: "createNetworkBlock",
            args: { payload: { blockName: "Work", icon: null } },
        });
        expect(response.data.networkBlock.id).toBe("block-id");
    });

    it("sends account credentials in a single typed IPC payload, not query metadata", async () => {
        vi.mocked(invoke).mockResolvedValue(null);

        await rpcRequest("setApiKey", { body: { apiKey: "abc" } });

        expect(invoke).toHaveBeenCalledWith("engine_call", {
            op: "setApiKey",
            args: { payload: { apiKey: "abc" } },
        });
    });

    it("preserves the cooldown response's established UI error shape", async () => {
        vi.mocked(invoke).mockResolvedValue({
            status: 425,
            body: { error: "Cooldown not elapsed", retryAfterSeconds: 30 },
        });
        const result = await rpcRequest("confirmUnlock", {
            path: { network_block_id: "block-id" },
            body: { challengeId: "challenge-id" },
        });

        expect(result.error).toMatchObject({
            status: 425,
            code: "too_early",
            retryAfterSeconds: 30,
        });
        expect(invoke).toHaveBeenCalledWith("engine_call", {
            op: "confirmUnlock",
            args: {
                network_block_id: "block-id",
                payload: { challengeId: "challenge-id" },
            },
        });
    });

    it("passes typed remoc events through the Tauri channel without SSE decoding", async () => {
        vi.mocked(invoke).mockImplementation(async (_command, args) => {
            const channel = (args as { onEvent: { onmessage?: (event: unknown) => void } }).onEvent;
            channel.onmessage?.({ IN_PROGRESS: { phase: "DOWNLOADING" } });
            channel.onmessage?.({ COMPLETED: { phase: "COMPLETED" } });
            return undefined as never;
        });

        const result = await rpcStream("resyncSitePack", {
            path: { site_pack_id: "pack-id" },
        });
        const events: unknown[] = [];
        for await (const event of result.stream) {
            events.push(event);
        }
        expect(events).toEqual([{ IN_PROGRESS: { phase: "DOWNLOADING" } }, { COMPLETED: { phase: "COMPLETED" } }]);
        expect(invoke).toHaveBeenCalledWith(
            "engine_stream",
            expect.objectContaining({
                op: "resyncSitePack",
                args: { site_pack_id: "pack-id" },
            }),
        );
    });
});
