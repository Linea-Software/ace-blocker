/** Typed error crossing the Rust/Tauri WebView boundary. Never return a primitive sentinel. */
export type RpcFailure = {
    code: string;
    status: number;
    error: string;
    retryAfterSeconds?: number;
};

/** Tauri may reject with a serialized Rust RpcError, an Error, or a string. */
export function normalizeRpcFailure(source: unknown): RpcFailure {
    if (source !== null && typeof source === "object") {
        const candidate = source as Record<string, unknown>;
        const code = typeof candidate.code === "string"
            ? candidate.code
            : source instanceof Error ? "transport_unavailable" : "rpc_failed";
        const status = typeof candidate.status === "number" && Number.isInteger(candidate.status)
            ? candidate.status : 503;
        const error = typeof candidate.error === "string"
            ? candidate.error
            : source instanceof Error ? source.message : "Engine request failed";
        const retryAfterSeconds = typeof candidate.retryAfterSeconds === "number"
            && Number.isFinite(candidate.retryAfterSeconds)
            ? candidate.retryAfterSeconds : undefined;
        return { code, status, error, ...(retryAfterSeconds === undefined ? {} : { retryAfterSeconds }) };
    }
    return {
        code: "transport_unavailable",
        status: 503,
        error: typeof source === "string" ? source : "Engine request failed",
    };
}
