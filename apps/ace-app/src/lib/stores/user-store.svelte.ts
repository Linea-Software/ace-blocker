import {
    getAccountInfo,
    refreshAccount,
    type AccountInfoResponse,
} from "$engine-client";
import { engineStore } from "$lib/stores/engine.svelte";

const USER_FETCH_INTERVAL_MS = 30_000;
/**
 * How long the store waits before refreshing again after a transient failure
 * (engine restarting, network blip). Entitlement is never cleared on such
 * errors: the last known-good value stays visible until a later refresh
 * succeeds or the engine explicitly reports logged-out state.
 */
const TRANSIENT_FAILURE_BACKOFF_MS = 10_000;

/**
 * This is a Svelte store that tracks a few things about the user. Right now
 * it is limited to knowing if the user is pro or not, and whether the ToS has
 * been accepted.
 *
 * Refresh model (single owner; see issue 4):
 *
 * - an immediate fetch on construction and a bounded periodic fetch while the
 *   app is open (logged-out requests are cheap local engine reads);
 * - a forced remote re-verification (`POST /account/refresh`) when the app
 *   regains focus / becomes visible again — this is the trigger that surfaces
 *   a subscription upgrade/downgrade completed in the browser;
 * - `refreshAfterLoginOrEntitlementChange()` for callers that know an
 *   entitlement-changing action just happened (login deep link, checkout
 *   return, engine (re)connect);
 * - transient errors keep the last known-good entitlement (never flip Pro →
 *   Free on a network error); only a successful refresh or an explicit
 *   logged-out account state changes it.
 *
 * Overlapping refreshes are coalesced: concurrent triggers share one in-flight
 * request instead of stampeding the engine.
 */
class UserStore {
    isPro: boolean = $state(false);
    tosAccepted: boolean = $state(true); // ignored until accountInfoLoaded
    accountInfoLoaded: boolean = $state(false);
    private intervalId: ReturnType<typeof setInterval> | null = null;
    private inFlight: Promise<void> | null = null;

    constructor() {
        this.init();
    }

    async init() {
        await this.fetch();
        this.intervalId = setInterval(
            () => this.fetch(),
            USER_FETCH_INTERVAL_MS,
        );
        // Refresh when the window regains focus or becomes visible — the most
        // common moment a subscription/entitlement change completes (the user
        // returns from the checkout/login page). Coalesced, so a burst of
        // focus events produces a single refresh.
        if (typeof window !== "undefined") {
            const refreshOnVisible = () => {
                if (document.visibilityState === "visible") {
                    this.refreshAfterLoginOrEntitlementChange();
                }
            };
            window.addEventListener("focus", refreshOnVisible);
            document.addEventListener("visibilitychange", refreshOnVisible);
        }
    }

    /** Call this to stop polling when the store is no longer needed. */
    destroy() {
        if (this.intervalId !== null) {
            clearInterval(this.intervalId);
            this.intervalId = null;
        }
    }

    /**
     * Force a remote entitlement re-verification then mirror the result.
     * Safe to call after login / logout / subscription changes / focus
     * regain: the engine skips the request entirely when logged out, keeps
     * the cached entitlement on transient errors, and only downgrades when
     * the server actually reports the account as free/unauthorized.
     */
    async refreshAfterLoginOrEntitlementChange(): Promise<void> {
        await this.withCoalescing(async () => {
            try {
                const { data, error } = await refreshAccount({
                    baseUrl: this.engineBaseUrl(),
                });
                if (error) {
                    // Keep the last known-good entitlement; the periodic
                    // fetch will retry.
                    return;
                }
                if (data) {
                    this.applyAccountInfo(data);
                }
            } catch (error) {
                if (error instanceof TypeError) {
                    // Network error during engine startup/shutdown — keep
                    // cached state and let the poll retry later.
                    return;
                }
                console.error("Failed to refresh account info:", error);
            }
        });
    }

    /**
     * Coalesces concurrent calls: while a refresh is in flight, further
     * triggers await the same operation instead of starting another request.
     */
    private async withCoalescing(operation: () => Promise<void>): Promise<void> {
        if (this.inFlight) return this.inFlight;
        const run = operation().finally(() => {
            this.inFlight = null;
        });
        this.inFlight = run;
        return run;
    }

    async fetch() {
        // Coalesce: the periodic poll and an external trigger (sidebar
        // logout, engine reconnect) must never overlap.
        await this.withCoalescing(async () => {
            try {
                const { data, error } = await getAccountInfo({
                    baseUrl: this.engineBaseUrl(),
                });
                if (error) {
                    // Engine reachable but reported an error: keep the last
                    // known entitlement rather than flipping to Free. This is
                    // a read of the engine's *cached* state, so an engine
                    // restart mid-poll must not downgrade the UI.
                    this.scheduleTransientRetry();
                    return;
                }
                if (data) {
                    this.applyAccountInfo(data);
                }
            } catch (error) {
                if (error instanceof TypeError) {
                    // Network error during engine startup/shutdown. Keep the
                    // legal gate closed until a later fetch confirms the
                    // persisted account state.
                    this.scheduleTransientRetry();
                    return;
                }
                console.error("Failed to fetch userStore info:", error);
            }
        });
    }

    private engineBaseUrl(): string {
        return `https://127.0.0.1:${engineStore.port}`;
    }

    private transientRetryTimer: ReturnType<typeof setTimeout> | null = null;

    private scheduleTransientRetry() {
        if (this.transientRetryTimer) return;
        this.transientRetryTimer = setTimeout(() => {
            this.transientRetryTimer = null;
            this.fetch();
        }, TRANSIENT_FAILURE_BACKOFF_MS);
    }

    private applyAccountInfo(data: AccountInfoResponse) {
        this.isPro = data.isPro;
        this.tosAccepted = data.tosAccepted ?? false;
        this.accountInfoLoaded = true;
    }
}

export const userStore = new UserStore();
