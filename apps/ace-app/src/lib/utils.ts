import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import { dev } from "$app/env";
import { openUrl } from "@tauri-apps/plugin-opener";

export function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs));
}
export type WithoutChild<T> = T extends { child?: unknown }
    ? Omit<T, "child">
    : T;
export type WithoutChildren<T> = T extends { children?: unknown }
    ? Omit<T, "children">
    : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & {
    ref?: U | null;
};

export function openGetProPage() {
    const baseUrl = dev ? "https://localhost:5173/" : "https://aceblocker.com/";
    openUrl(`${baseUrl}get-pro?origin=aceblocker`);
}

/**
 * Converts bytes into a human-readable string (e.g., "1.89 MB").
 */
export function formatBytes(bytes: number, decimals: number = 2) {
    if (bytes <= 0 || !Number.isFinite(bytes)) return "0 Bytes";

    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

    const i = Math.floor(Math.log(bytes) / Math.log(k));

    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

/**
 * Formats a number with thousands separator.
 */
export function formatNumber(num: number): string {
    return new Intl.NumberFormat().format(num);
}
