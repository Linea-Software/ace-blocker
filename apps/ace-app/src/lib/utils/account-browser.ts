import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "svelte-sonner";
import * as m from "$lib/paraglide/messages.js";

type AccountPage = "pro" | "sign-in";

// Account pages open in the user's browser, never through the privileged Engine.
// Development points to the local Ace website; release builds use the public
// origin. Credentials still use the Engine's allowlisted login callback.
export function getAccountPageUrl(
    page: AccountPage,
    debug = import.meta.env.DEV,
): string {
    const origin = debug ? "http://localhost:5173" : "https://aceblocker.com";
    const pathname = page === "pro" ? "/get-pro" : "/signin";
    return `${origin}${pathname}`;
}

export const ACCOUNT_PAGE_URLS: Record<AccountPage, string> = {
    pro: getAccountPageUrl("pro"),
    "sign-in": getAccountPageUrl("sign-in"),
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
            // Browser-only development uses the same local account site.
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
