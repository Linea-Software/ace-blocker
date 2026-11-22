import { onSettled } from "solid-js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import { Bug, RefreshCcw, ShieldCheck } from "@lucide/solid";
import { toast } from "solid-sonner";
import * as m from "$lib/paraglide/messages.js";
import { userStore } from "$lib/stores/user-store";
import { getDebugProPolicy, resetTos, setDebugProPolicy, type DebugProPolicy } from "$engine-client";
import "./_DebugSection.css";

export default function DebugSection() {
    const state = {
        selectedPolicy: "Default" as DebugProPolicy,
        loadingPolicy: true,
        savingPolicy: false,
        resettingTos: false,
    };
    reactiveFields(state, ["selectedPolicy", "loadingPolicy", "savingPolicy", "resettingTos"]);
    function isPolicy(value: string): value is DebugProPolicy {
        return value === "Default" || value === "Free" || value === "Pro";
    }

    onSettled(() => {
        void (async () => {
            try {
                const { data, error } = await getDebugProPolicy();
                if (error || !data) throw error ?? new Error("Missing debug policy");
                state.selectedPolicy = data;
            } catch {
                toast.error(m.debug_license_load_failed());
            } finally {
                state.loadingPolicy = false;
            }
        })();
    });

    async function changePolicy(value: string) {
        if (!isPolicy(value) || state.savingPolicy || state.loadingPolicy || value === state.selectedPolicy) return;

        const previous = state.selectedPolicy;
        state.savingPolicy = true;
        try {
            const { error } = await setDebugProPolicy({ path: { policy: value } });
            if (error) throw error;
            state.selectedPolicy = value;
            await userStore.fetch();
            toast.success(m.debug_license_changed());
        } catch {
            state.selectedPolicy = previous;
            toast.error(m.debug_license_change_failed());
        } finally {
            state.savingPolicy = false;
        }
    }

    async function replayTos() {
        if (state.resettingTos) return;
        state.resettingTos = true;
        try {
            const { error } = await resetTos();
            if (error) throw error;
            toast.success(m.debug_legal_reset_success());
            // The engine has persisted the reset. Immediately close the legal
            // gate so the non-dismissible modal appears without an app restart.
            userStore.tosAccepted = false;
        } catch {
            toast.error(m.debug_legal_reset_failed());
        } finally {
            state.resettingTos = false;
        }
    }

    return (
      <section class="settings-section ace-settings-debug">
    <header>
        <span class="section-mark" aria-hidden="true"><Bug size={16} /></span>
        <div>
            <h2>{m.debug_settings_title()}</h2>
            <p>{m.debug_settings_description()}</p>
        </div>
    </header>

    <div class="setting-row">
        <div class="setting-copy">
            <label for="debug-license-policy">{m.debug_license_label()}</label>
            <p>{m.debug_license_description()}</p>
        </div>
        <select
            id="debug-license-policy"
            value={state.selectedPolicy}
            disabled={state.loadingPolicy || state.savingPolicy}
            onChange={(event) => void changePolicy(event.currentTarget.value)}
        >
            <option value="Default">{m.debug_license_default()}</option>
            <option value="Free">{m.debug_license_free()}</option>
            <option value="Pro">{m.debug_license_pro()}</option>
        </select>
    </div>

    <div class="setting-row">
        <div class="setting-copy">
            <strong>{m.debug_legal_label()}</strong>
            <p>{m.debug_legal_description()}</p>
        </div>
        <button type="button" disabled={state.resettingTos} onClick={replayTos}>
            {state.resettingTos ? (<>
                <RefreshCcw size={15} class="animate-spin" aria-hidden="true" />
            </>) : (<>
                <ShieldCheck size={15} aria-hidden="true" />
            </>)}
            {m.debug_legal_reset_button()}
        </button>
    </div>
</section>
    );
}
