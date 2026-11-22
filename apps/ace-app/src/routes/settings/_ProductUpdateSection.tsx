import { onSettled } from "solid-js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import * as m from "$lib/paraglide/messages.js";
import { checkUpdates, getVersion } from "$lib/engine-client";
import { ArrowDownToLine, RefreshCw, ShieldCheck } from "@lucide/solid";
import "./_ProductUpdateSection.css";

export default function ProductUpdateSection() {
    const state = {
        updateState: "checking" as CheckState,
        installedVersion: null as string | null,
        releaseVersion: null as string | null,
        busy: false,
    };
    reactiveFields(state, [
        "updateState",
        "installedVersion",
        "releaseVersion",
        "busy",
    ]);
    type CheckState =
        | "checking"
        | "disabled"
        | "upToDate"
        | "available"
        | "downloading"
        | "verifying"
        | "downloaded"
        | "error";

    async function refreshStatus(polling = false) {
        if (state.busy) return;
        state.busy = true;
        if (!polling) state.updateState = "checking";
        try {
            // Read installed version even when the remote feed is offline.
            const installed = await getVersion();
            if (installed.data) state.installedVersion = installed.data.version;

            const response = await checkUpdates();
            if (response.error || !response.data) {
                state.updateState = "error";
                return;
            }
            state.installedVersion = response.data.engine.currentVersion;
            state.releaseVersion = response.data.engine.latestVersion ?? null;
            state.updateState = response.data.engine.status;
        } catch (error) {
            console.error("Product update check failed", error);
            state.updateState = "error";
        } finally {
            state.busy = false;
        }
    }

    onSettled(() => {
        void refreshStatus();
        // Poll only while a signed MSI is in progress; do not make the UI
        // block navigation or start a second downloader.
        const interval = setInterval(() => {
            if (
                ["checking", "downloading", "verifying"].includes(
                    state.updateState,
                ) &&
                !state.busy
            ) {
                void refreshStatus(true);
            }
        }, 15_000);
        return () => clearInterval(interval);
    });

    return (
        <section
            class="settings-section ace-settings-product-update"
            aria-labelledby="product-update-title"
        >
            <header>
                <span class="section-mark" aria-hidden="true">
                    <ArrowDownToLine size={16} />
                </span>
                <div>
                    <h2 id="product-update-title">
                        {m.product_updates_title()}
                    </h2>
                    <p>{m.product_updates_description()}</p>
                </div>
            </header>

            <div class="update-row">
                <div class="update-details">
                    <div class="version-label">
                        {m.product_updates_installed()}
                    </div>
                    <strong class="version-number">
                        {state.installedVersion ?? "—"}
                    </strong>
                    <p role="status" aria-live="polite">
                        {state.updateState === "checking" ? (
                            <>{m.product_updates_checking()}</>
                        ) : state.updateState === "available" ? (
                            <>
                                <ShieldCheck size={14} aria-hidden="true" />
                                {m.product_updates_available({
                                    version: state.releaseVersion ?? "—",
                                })}
                            </>
                        ) : state.updateState === "downloading" ? (
                            <>
                                {m.product_updates_downloading({
                                    version: state.releaseVersion ?? "—",
                                })}
                            </>
                        ) : state.updateState === "verifying" ? (
                            <>
                                {m.product_updates_verifying({
                                    version: state.releaseVersion ?? "—",
                                })}
                            </>
                        ) : state.updateState === "downloaded" ? (
                            <>
                                <ShieldCheck size={14} aria-hidden="true" />
                                {m.product_updates_downloaded({
                                    version: state.releaseVersion ?? "—",
                                })}
                            </>
                        ) : state.updateState === "upToDate" ? (
                            <>{m.product_updates_current()}</>
                        ) : state.updateState === "disabled" ? (
                            <>{m.product_updates_disabled()}</>
                        ) : (
                            <>{m.product_updates_error()}</>
                        )}
                    </p>
                    {state.updateState === "available" ||
                    state.updateState === "downloaded" ? (
                        <>
                            <p class="install-notice">
                                {m.product_updates_not_installable()}
                            </p>
                        </>
                    ) : null}
                </div>

                <button
                    class="refresh-button"
                    type="button"
                    disabled={state.busy}
                    onClick={() => void refreshStatus()}
                    aria-label={m.product_updates_retry()}
                >
                    <RefreshCw
                        size={15}
                        aria-hidden="true"
                        class={state.busy ? "spinning" : undefined}
                    />
                    <span>{m.product_updates_retry()}</span>
                </button>
            </div>
        </section>
    );
}
