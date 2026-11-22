import { createEffect, onSettled } from "solid-js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import * as m from "$lib/paraglide/messages.js";
import { countMessage } from "$lib/utils/count-message";
import { getAllScripts, getResistanceModeSettings, setResistanceModeEnabled, setResistanceModeLock, setScriptEnabled, type ScriptInfo, type ResistanceModeSettings } from "$engine-client";
import { engineStore } from "$lib/stores/engine";
import { userStore } from "$lib/stores/user-store";
import UpgradeToProButton from "$lib/components/UpgradeToProButton";
import { LockKeyhole } from "@lucide/solid";
import { toast } from "solid-sonner";
import StatusCard from "./_components/StatusCard";
import ScriptSections from "./_components/ScriptSections";
import TimeLockDialog from "./_components/TimeLockDialog";
import PageLayout from "$lib/components/PageLayout";
import PageHeader from "$lib/components/PageHeader";
import "./page.css";

export default function ResistanceModePage() {
 const state = {
  settings: null as ResistanceModeSettings | null,
  scripts: [] as ScriptInfo[],
  loading: true,
  loadError: false,
  mounted: false,
  isMutating: false,
  lockDialogOpen: false,
  pageVisible: true,
 };
 reactiveFields(state, ["settings", "scripts", "loading", "loadError", "mounted", "isMutating", "lockDialogOpen", "pageVisible"]);
 let refreshSeq = 0;
 const isPro = () => userStore.isPro;
 const engineOnline = () => engineStore.isAlive;
 const isLocked = () => state.settings?.isLocked ?? false;
 const controlsDisabled = () => !engineOnline() || !isPro();
 onSettled(() => { document.title = `${m.resistance_mode()} · ${m.app_name()}`; });
    async function refresh() {
        const seq = ++refreshSeq;
        if (!engineOnline() || !state.mounted || (state.isMutating && state.settings)) return;
        // Polling an already-loaded view should not flash a state.loading state.
        if (!state.settings) state.loading = true;
        state.loadError = false;
        try {
            const [scriptsRes, settingsRes] = await Promise.all([
                getAllScripts(),
                getResistanceModeSettings(),
            ]);
            if (seq !== refreshSeq || (state.isMutating && state.settings)) return;
            if (scriptsRes.error || settingsRes.error || !scriptsRes.data || !settingsRes.data) {
                throw new Error("Resistance data unavailable");
            }
            state.scripts = scriptsRes.data.scripts;
            state.settings = settingsRes.data;
        } catch {
            if (seq !== refreshSeq) return;
            state.scripts = [];
            state.settings = null;
            state.loadError = true;
        } finally {
            if (seq === refreshSeq) state.loading = false;
        }
    }

    // Reachability changes invalidate pending reads and reload on reconnect.
    createEffect(() => [state.mounted, engineOnline()] as const, () => {
        if (!state.mounted) return;
        if (engineOnline()) {
            void refresh();
        } else {
            ++refreshSeq;
            state.loading = false;
            state.loadError = false;
            state.settings = null;
            state.scripts = [];
        }
    });

    // Poll while visible. The engine store owns reachability detection, so the
    // interval simply skips work while offline and resumes automatically.
    onSettled(() => {
        state.mounted = true;
        state.pageVisible = document.visibilityState === "visible";
        const onVisibility = () => {
            state.pageVisible = document.visibilityState === "visible";
            if (state.pageVisible && engineStore.isAlive && !state.loadError) void refresh();
        };
        document.addEventListener("visibilitychange", onVisibility);

        const id = setInterval(() => {
            if (state.pageVisible && engineStore.isAlive && !state.loading && !state.loadError) void refresh();
        }, 5000);

        return () => {
            state.mounted = false;
            ++refreshSeq;
            document.removeEventListener("visibilitychange", onVisibility);
            clearInterval(id);
        };
    });

    // ─── Error mapping (stable engine codes, see engine docs) ────────
    function errorCode(error: unknown): string | undefined {
        if (error && typeof error === "object") {
            const candidate = (error as Record<string, unknown>).code;
            if (typeof candidate === "string") return candidate;
        }
        return undefined;
    }

    function showMutationError(error: { code: string; status: number }) {
        const code = error.code;
        if (code === "resistance_locked") {
            toast.error(m.locked_one_way_notice());
        } else if (code === "unsupported_lock_state") {
            toast.error(m.error_lock_state_unsupported());
        } else if (code === "script_download_failed") {
            toast.error(m.failed_to_download_scripts());
        } else if (code === "scripts_unavailable") {
            toast.error(m.error_scripts_unavailable());
        } else if (code === "pro_plan_required" || code === "pro_status_check_failed") {
            toast.error(m.status_pro_required());
        } else if (code === "invalid_request_body") {
            toast.error(m.error_invalid_request());
        } else if (error.status === 401 || code === "capability_required") {
            toast.error(m.error_unexpected());
        } else if (!engineOnline()) {
            toast.error(m.error_engine_offline_action());
        } else {
            toast.error(m.error_unexpected());
        }
    }

    // ─── Mutations ───────────────────────────────────────────────────
    async function handleToggleEnabled(enabled: boolean) {
        if (state.isMutating) return; // prevent duplicated mutations from rapid clicks
        state.isMutating = true;
        const { error } = await setResistanceModeEnabled({
            body: { enabled },
        });
        state.isMutating = false;
        if (error) {
            showMutationError(error);
            return;
        }
        await refresh();
        toast.success(
            enabled ? m.resistance_mode_active() : m.resistance_mode_inactive(),
        );
    }

    async function handleToggleScript(script: ScriptInfo) {
        if (state.isMutating || !engineOnline() || !isPro()) return;
        if (isLocked() && script.isEnabled) {
            toast.error(m.locked_one_way_notice());
            return;
        }
        state.isMutating = true;
        const next = !script.isEnabled;
        state.scripts = state.scripts.map((s) => s.id === script.id ? { ...s, isEnabled: next } : s);
        const { error } = await setScriptEnabled({
            path: { script_id: script.id },
            body: { enabled: next },
        });
        if (error) {
            state.scripts = state.scripts.map((s) => s.id === script.id ? { ...s, isEnabled: script.isEnabled } : s);
            showMutationError(error);
        } else {
            toast.success(next ? m.script_activated({ name: script.name }) : m.script_deactivated({ name: script.name }));
        }
        state.isMutating = false;
        await refresh();
    }

    async function handleToggleGroup(group: ScriptInfo[], enabled: boolean) {
        if (state.isMutating || !engineOnline() || !isPro()) return;
        if (isLocked() && !enabled) {
            toast.error(m.locked_one_way_notice());
            return;
        }
        const targets = group.filter((s) => s.isEnabled !== enabled);
        if (!targets.length) return;
        state.isMutating = true;
        const ids = new Set(targets.map((s) => s.id));
        const original = new Map(targets.map((s) => [s.id, s.isEnabled]));
        state.scripts = state.scripts.map((s) => ids.has(s.id) ? { ...s, isEnabled: enabled } : s);

        const failed = new Set<string>();
        // Keep write operations bounded rather than flooding the engine's RPC channel.
        for (let i = 0; i < targets.length; i += 6) {
            const chunk = targets.slice(i, i + 6);
            const responses = await Promise.all(chunk.map(async (script) => ({
                id: script.id,
                result: await setScriptEnabled({path: {script_id: script.id}, body: {enabled}}),
            })));
            for (const item of responses) {
                if (item.result.error) failed.add(item.id);
            }
        }
        if (failed.size) {
            state.scripts = state.scripts.map((s) => failed.has(s.id) ? { ...s, isEnabled: original.get(s.id) ?? s.isEnabled } : s);
            toast.error(countMessage(
                failed.size,
                m.items_failed_to_update_zero,
                m.items_failed_to_update_one,
                () => m.items_failed_to_update({count:failed.size}),
            ));
        } else {
            toast.success(m.domain_policies_updated({domain: m.resistance_mode()}));
        }
        state.isMutating = false;
        await refresh();
    }

    async function handleCommitLock(isoEnd: string) {
        if (state.isMutating) return;
        state.isMutating = true;
        const { error } = await setResistanceModeLock({
            body: { lockState: "active_until_lock_end", lockEnd: isoEnd },
        });
        state.isMutating = false;
        if (error) {
            showMutationError(error);
            return;
        }
        state.lockDialogOpen = false;
        toast.success(m.engine_locked_success());
        await refresh();
    }

 return (
<PageLayout header={<PageHeader
            title={m.resistance_mode()}
            subtitle={m.resistance_mode_beta_description()}
        >
            <span data-testid="rm-beta-badge" class="beta-note">
                {m.beta_badge()}
            </span>
            {engineOnline() && !state.loadError && isPro() && state.settings?.enabled && !isLocked() ? (<>
                <button
                    type="button"
                    class="lock-action"
                    data-testid="rm-open-lock-dialog"
                    onClick={() => (state.lockDialogOpen = true)}
                    title={m.time_lock_description()}
                >
                    <LockKeyhole size={16} aria-hidden="true" />
                    {m.time_lock_title()}
                </button>
            </>) : null}
        </PageHeader>}>


    {!engineOnline() ? (<>
        <div class="resistance-stack">
            <StatusCard
                settings={state.settings}
                engineOnline={engineOnline()}
                isPro={isPro()}
                enabled={state.settings?.enabled ?? false}
                locked={isLocked()}
                controlsDisabled={true}
                inFlight={state.isMutating}
                onToggle={handleToggleEnabled}
            />
        </div>
    </>) : state.loadError || !state.settings ? (<>
        <section class="load-state" aria-busy={state.loading ? "true" : "false"} data-testid="rm-load-state">
            <div role="status" aria-live="polite">
                <h2>{state.loadError ? m.resistance_load_error_title() : m.resistance_loading()}</h2>
                {state.loadError ? (<>
                    <p>{m.resistance_load_error_description()}</p>
                </>) : null}
            </div>
            {state.loadError ? (<>
                <button type="button" class="retry-action" onClick={() => void refresh()} disabled={state.loading}>
                    {m.retry()}
                </button>
            </>) : null}
        </section>
    </>) : (<>
        <div class="resistance-stack" aria-busy={state.loading ? "true" : "false"}>
            {state.loading ? (<>
                <p class="refresh-status" role="status">{m.resistance_loading()}</p>
            </>) : null}
            {!isPro() ? (<>
                <div class="pro-gate" data-testid="rm-pro-gate">
                    <div class="pro-gate-copy">
                        <h2>
                            <LockKeyhole size={17} aria-hidden="true" />
                            {m.resistance_pro_preview_title()}
                        </h2>
                        <p>{m.resistance_pro_preview_description()}</p>
                    </div>
                    <UpgradeToProButton text={m.resistance_mode()} />
                </div>
            </>) : null}

            <section class="rounded-md border border-border bg-card px-5 py-4">
                <h2 class="font-semibold text-lg">{m.resistance_examples_title()}</h2>
                <p class="mt-1 max-w-3xl text-sm leading-relaxed text-muted-foreground">{m.resistance_examples_body()}</p>
            </section>

            <StatusCard
                settings={state.settings}
                engineOnline={engineOnline()}
                isPro={isPro()}
                enabled={isPro() && (state.settings?.enabled ?? false)}
                locked={isPro() && isLocked()}
                controlsDisabled={controlsDisabled()}
                inFlight={state.isMutating}
                onToggle={handleToggleEnabled}
            />

            <div class="coverage-ledger">
                <ScriptSections
                        scripts={state.scripts}
                        settings={state.settings}
                        engineOnline={engineOnline()}
                        isPro={isPro()}
                        locked={isLocked()}
                        busy={state.isMutating}
                        onToggleScript={handleToggleScript}
                        onToggleGroup={handleToggleGroup}
                />
            </div>
        </div>
    </>)}

    <TimeLockDialog
        open={state.lockDialogOpen} onOpenChange={(open: boolean) => { state.lockDialogOpen = open; }}
        isLocked={isLocked()}
        lockEndRaw={state.settings?.lockEnd ?? null}
        disabled={!engineOnline() || state.loadError || !state.settings}
        committing={state.isMutating}
        onCommit={handleCommitLock}
    />
</PageLayout>
 );
}
