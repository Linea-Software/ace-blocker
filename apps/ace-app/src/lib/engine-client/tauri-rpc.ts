import { Channel, invoke } from "@tauri-apps/api/core";
import { rpcParameters, type RpcOperation } from "./rpc-parameters";
import type { RequestResult, RpcStreamResult } from "./rpc-types";
import { normalizeRpcFailure } from "./rpc-error";

type RequestOptions = {
    body?: unknown;
    path?: Record<string, unknown>;
    query?: Record<string, unknown>;
    throwOnError?: boolean;
    responseStyle?: "fields" | "data";
    onStreamError?: (error: unknown) => void;
};

// Only this adapter knows about Tauri. The WebView never handles pipe names,
// transport credentials or serialized remoc messages.
function argumentsFor(operation: RpcOperation, options?: RequestOptions): Record<string, unknown> {
    const data: Record<string, unknown> = {};
    const path = options?.path ?? {};
    for (const param of rpcParameters[operation]) {
        // The current generated RPC contract has no "body"/"query" parameters,
        // but the adapter also supports those transport field names if added.
        const fieldName: string = param;
        if (fieldName === "payload" || fieldName === "entry" || fieldName === "mode" || fieldName === "body") {
            data[fieldName] = options?.body;
        } else if (fieldName === "query") {
            data[fieldName] = options?.query ?? {};
        } else {
            data[fieldName] = path[fieldName] ?? options?.query?.[fieldName];
        }
    }
    return data;
}

async function rpcRequestInternal(operation: RpcOperation, options?: RequestOptions): Promise<unknown> {
    try {
        const value = await invoke<unknown>("engine_call", {
            op: operation,
            args: argumentsFor(operation, options),
        });
        // Only confirmUnlock embeds its cooldown result in an otherwise
        // successful remoc response. Keep 425 as an explicit typed error.
        if (operation === "confirmUnlock" && value !== null && typeof value === "object" && "status" in value) {
            const outcome = value as { status: number; body: unknown };
            if (outcome.status >= 400) {
                const details = outcome.body && typeof outcome.body === "object"
                    ? outcome.body : { error: "Unlock confirmation failed" };
                throw normalizeRpcFailure({
                    ...details,
                    code: outcome.status === 425 ? "too_early" : "unlock_failed",
                    status: outcome.status,
                });
            }
            return options?.responseStyle === "data" ? outcome.body : { data: outcome.body };
        }
        return options?.responseStyle === "data" ? value : { data: value };
    } catch (error) {
        const failure = normalizeRpcFailure(error);
        if (options?.throwOnError) throw failure;
        return { error: failure };
    }
}

export function rpcRequest<TResponse, TError = unknown, ThrowOnError extends boolean = false>(
    operation: RpcOperation,
    options?: RequestOptions,
): RequestResult<TResponse, TError, ThrowOnError> {
    // The unchecked cast is owned here at the untrusted Tauri serialization seam.
    return rpcRequestInternal(operation, options) as RequestResult<TResponse, TError, ThrowOnError>;
}

export async function rpcStream<TResponse>(operation: RpcOperation, options?: RequestOptions): Promise<RpcStreamResult<TResponse>> {
    const parsed: unknown[] = [];
    let firstUnreadIndex = 0;
    let done = false;
    let streamError: unknown;
    let notify: (() => void) | undefined;
    const channel = new Channel<unknown>();

    function wake() {
        const callback = notify;
        notify = undefined;
        callback?.();
    }

    // Events are serialized by Tauri directly from the typed remoc channel.
    // No HTTP response framing, SSE parsing, or byte decoding is involved.
    channel.onmessage = (event) => {
        parsed.push(event);
        wake();
    };

    const promise = invoke<void>("engine_stream", {
        op: operation,
        args: argumentsFor(operation, options),
        onEvent: channel,
    });
    void promise
        .catch((error: unknown) => {
            streamError = error;
            try {
                options?.onStreamError?.(error);
            } catch (callbackError) {
                streamError = callbackError;
            }
        })
        .finally(() => {
            done = true;
            wake();
        });

    return {
        stream: {
            async *[Symbol.asyncIterator](): AsyncGenerator<any> {
                while (!done || firstUnreadIndex < parsed.length) {
                    if (firstUnreadIndex < parsed.length) {
                        const event = parsed[firstUnreadIndex++];
                        // Keep amortized constant-time dequeue without retaining consumed events.
                        if (firstUnreadIndex === parsed.length) {
                            parsed.length = 0;
                            firstUnreadIndex = 0;
                        } else if (firstUnreadIndex >= 64 && firstUnreadIndex * 2 >= parsed.length) {
                            parsed.splice(0, firstUnreadIndex);
                            firstUnreadIndex = 0;
                        }
                        yield event;
                    } else {
                        await new Promise<void>((resolve) => {
                            notify = resolve;
                        });
                    }
                    if (streamError) throw streamError;
                }
                if (streamError) throw streamError;
            },
        },
    } as RpcStreamResult<TResponse>;
}
