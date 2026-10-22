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

    it("delivers buffered and newly arriving events in FIFO order", async () => {
        let emit!: (event: unknown) => void;
        let finish!: () => void;

        vi.mocked(invoke).mockImplementation((_command, args) => {
            emit = (args as { onEvent: { onmessage: (event: unknown) => void } }).onEvent.onmessage;
            return new Promise<void>((resolve) => {
                finish = resolve;
            }) as never;
        });

        const { stream } = await rpcStream("resyncSitePack");
        const iterator = stream[Symbol.asyncIterator]();
        const observed: unknown[] = [];

        for (let index = 0; index < 128; index++) emit(index);
        for (let index = 0; index < 96; index++) {
            const next = await iterator.next();
            expect(next.done).toBe(false);
            observed.push(next.value);
        }
        for (let index = 128; index < 256; index++) emit(index);
        finish();

        for await (const event of iterator) observed.push(event);
        expect(observed).toEqual(Array.from({ length: 256 }, (_, index) => index));
    });

    it("wakes an awaiting consumer when an event arrives after subscription", async () => {
        let emit!: (event: unknown) => void;
        let finish!: () => void;
        vi.mocked(invoke).mockImplementation((_command, args) => {
            emit = (args as { onEvent: { onmessage: (event: unknown) => void } }).onEvent.onmessage;
            return new Promise<void>((resolve) => {
                finish = resolve;
            }) as never;
        });

        const { stream } = await rpcStream("resyncSitePack");
        const iterator = stream[Symbol.asyncIterator]();
        const pending = iterator.next();
        emit({ phase: "late" });
        expect(await pending).toEqual({ done: false, value: { phase: "late" } });
        finish();
        expect(await iterator.next()).toEqual({ done: true, value: undefined });
    });

    it("keeps the stream error callback and rejection behavior after buffered events", async () => {
        const failure = new Error("IPC disconnected");
        const onStreamError = vi.fn();
        vi.mocked(invoke).mockImplementation(async (_command, args) => {
            const channel = (args as { onEvent: { onmessage?: (event: unknown) => void } }).onEvent;
            channel.onmessage?.({ phase: "first" });
            channel.onmessage?.({ phase: "second" });
            throw failure;
        });

        const { stream } = await rpcStream("resyncSitePack", { onStreamError });
        const iterator = stream[Symbol.asyncIterator]();
        expect(await iterator.next()).toEqual({ done: false, value: { phase: "first" } });
        await expect(iterator.next()).rejects.toBe(failure);
        expect(onStreamError).toHaveBeenCalledExactlyOnceWith(failure);
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
