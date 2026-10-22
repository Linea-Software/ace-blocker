/** Narrow, transport-independent types for the Svelte/Tauri RPC facade. */
import type { RpcFailure } from "./rpc-error";
type ResponseValue<T> = T extends object ? T[keyof T] : T;
export type RequestResult<TResponse, TError, ThrowOnError extends boolean = false> =
    Promise<ThrowOnError extends true
        ? { data: ResponseValue<TResponse>; error?: never }
        : { data: ResponseValue<TResponse>; error?: never } |
          { data?: undefined; error: RpcFailure & { readonly domainError?: ResponseValue<TError> } }>;
export type RpcStreamResult<TResponse> = {
    stream?: AsyncIterable<ResponseValue<TResponse>>;
};
type Field<T, K extends string> = T extends { [P in K]: infer V }
    ? { [P in K]: V } : { [P in K]?: never };
export type RpcOptions<TData extends object, ThrowOnError extends boolean, TResponse> =
    Field<TData, "body"> & Field<TData, "path"> & Field<TData, "query"> & {
        throwOnError?: ThrowOnError;
        responseStyle?: "fields" | "data";
        onStreamError?: (error: unknown) => void;
        /** This is a typed Tauri facade; no URL, headers, or fetch configuration. */
        readonly __responseMarker?: TResponse;
    };
