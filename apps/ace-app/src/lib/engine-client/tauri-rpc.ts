import { Channel, invoke } from "@tauri-apps/api/core";
import { rpcParameters } from "./rpc-parameters";

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
function argumentsFor(operation: string, options?: RequestOptions): Record<string, unknown> {
    const data: Record<string, unknown> = {};
    const path = options?.path ?? {};
    for (const param of rpcParameters[operation] ?? []) {
        if (param === "payload" || param === "entry" || param === "mode" || param === "body") {
            data[param] = options?.body;
        } else if (param === "query") {
            data[param] = options?.query ?? {};
        } else {
            data[param] = path[param] ?? options?.query?.[param];
        }
    }
    return data;
}

export async function rpcRequest(operation: string, options?: RequestOptions): Promise<any> {
    try {
        const value = await invoke<any>("engine_call", {
            op: operation,
            args: argumentsFor(operation, options),
        });
        // This operation also returns the original 425 cooldown response.
        if (operation === "confirmUnlock" && value?.status !== undefined) {
            if (value.status >= 400) {
                const error = { ...value.body, code: "too_early", status: value.status };
                if (options?.throwOnError) throw error;
                return { error };
            }
            return options?.responseStyle === "data" ? value.body : { data: value.body };
        }
        return options?.responseStyle === "data" ? value : { data: value };
    } catch (error) {
        if (options?.throwOnError) throw error;
        return { error };
    }
}

export async function rpcStream(operation: string, options?: RequestOptions): Promise<any> {
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
    };
}
