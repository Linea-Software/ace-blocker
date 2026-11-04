import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "svelte-sonner";
import * as m from "$lib/paraglide/messages.js";

type AccountPage = "pro" | "sign-in";

// These are public website URLs only. Credentials never travel through this
// browser-opening helper; login handoff remains the engine's allowlisted callback.
export const ACCOUNT_PAGE_URLS: Record<AccountPage, string> = {
    pro: "https://aceblocker.com/get-pro?origin=aceblocker",
    "sign-in": "https://aceblocker.com/signin?origin=aceblocker",
};

/**
 * Open the browser from the user's desktop session, not the privileged
 * engine process (which may run without a graphical session on Linux).
 */
export async function openAccountPage(page: AccountPage): Promise<boolean> {
    const url = ACCOUNT_PAGE_URLS[page];
    try {
        if (isTauri()) {
            await openUrl(url);
        } else {
            // Browser-only development: retain the same explicit public URL.
            window.open(url, "_blank", "noopener,noreferrer");
        }
        return true;
    } catch (error) {
        console.error("Could not open the Ace account page:", error);
        toast.error(m.account_browser_open_failed(), {
            description: m.account_browser_open_failed_detail({ url }),
        });
        return false;
    }
}
