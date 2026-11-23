import { createEffect } from "solid-js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import * as m from "$lib/paraglide/messages.js";
import {
    downloadResistanceScripts,
    getResistanceModeSettings,
} from "$lib/engine-client";
import { engineStore } from "$lib/stores/engine";
import { ListFilter, RefreshCw } from "@lucide/solid";
import { toast } from "solid-sonner";
import "./_ResistanceScriptsUpdateSection.css";

export default function ResistanceScriptsUpdateSection() {
    type VersionState = "loading" | "ready" | "missing" | "unavailable";
    const state = {
        installedVersion: null as string | null,
        versionState: "loading" as VersionState,
        updating: false,
    };
    reactiveFields(state, ["installedVersion", "versionState", "updating"]);
    let requestSeq = 0;
    const engineOnline = () => engineStore.isAlive;
    const canUpdate = () => engineOnline() && !state.updating;
    async function loadInstalledVersion() {
        if (!engineStore.isAlive) {
            state.versionState = "unavailable";
            return;
        }
        const seq = ++requestSeq;
        state.versionState = "loading";
        try {
            const { data, error } = await getResistanceModeSettings();
            if (seq !== requestSeq) return;
            if (error || !data) {
                state.versionState = "unavailable";
                return;
            }
            state.installedVersion = data.scriptsVersion ?? null;
            state.versionState =
                data.enforcementStatus === "scripts_missing"
                    ? "missing"
                    : "ready";
        } catch {
            if (seq === requestSeq) state.versionState = "unavailable";
        }
    }

    createEffect(
        () => engineOnline(),
        () => {
            if (engineOnline()) {
                void loadInstalledVersion();
            } else {
                ++requestSeq;
                state.installedVersion = null;
                state.versionState = "unavailable";
            }
            return () => {
                ++requestSeq;
            };
        },
    );

    async function handleUpdate() {
        if (!canUpdate()) return;
        state.updating = true;
        try {
            const { error } = await downloadResistanceScripts();
            if (error) {
                toast.error(m.failed_to_download_scripts());
                return;
            }
            toast.success(m.scripts_sync_success());
            await loadInstalledVersion();
        } catch {
            toast.error(m.failed_to_download_scripts());
        } finally {
            state.updating = false;
        }
    }

    return (
        <section
            id="resistance-scripts-update"
            class="settings-section ace-settings-resistance-scripts-update scroll-mt-6"
            aria-labelledby="resistance-scripts-update-title"
        >
            <header>
                <span class="section-mark" aria-hidden="true">
                    <ListFilter size={16} />
                </span>
                <div>
                    <h2 id="resistance-scripts-update-title">
                        {m.settings_resistance_scripts_title()}
                    </h2>
                    <p>{m.settings_resistance_scripts_description()}</p>
                </div>
            </header>

            <div class="update-row">
                <div class="update-details">
                    <div class="version-label">
                        {m.settings_resistance_scripts_installed()}
                    </div>
                    <strong
                        class="version-number"
                        data-testid="settings-resistance-scripts-version"
                    >
                        {state.installedVersion ? (
                            <>{state.installedVersion}</>
                        ) : state.versionState === "missing" ? (
                            <>{m.settings_resistance_scripts_missing()}</>
                        ) : (
                            <>{m.settings_resistance_scripts_unavailable()}</>
                        )}
                    </strong>
                    {!engineOnline() ? (
                        <>
                            <p>
                                {m.settings_resistance_scripts_engine_offline()}
                            </p>
                        </>
                    ) : state.versionState === "missing" ? (
                        <>
                            <p>{m.scripts_empty_description()}</p>
                        </>
                    ) : state.versionState === "unavailable" ? (
                        <>
                            <p>
                                {m.settings_resistance_scripts_status_unavailable()}
                            </p>
                        </>
                    ) : null}
                </div>
                <button
                    type="button"
                    class="refresh-button"
                    data-testid="settings-resistance-scripts-update"
                    disabled={!canUpdate()}
                    onClick={handleUpdate}
                >
                    <RefreshCw
                        size={15}
                        aria-hidden="true"
                        class={state.updating ? "spinning" : undefined}
                    />
                    {state.updating
                        ? m.settings_resistance_scripts_updating()
                        : m.settings_resistance_scripts_action()}
                </button>
            </div>
        </section>
    );
}
