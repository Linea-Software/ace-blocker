<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { Switch } from "$lib/components/ui/switch";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { Skeleton } from "$lib/components/ui/skeleton";
    import type { ResistanceModeSettings } from "$engine-client";
    import { formattedLockEnd } from "../_components/lock-format";

    let {
        settings,
        engineOnline,
        isPro = true,
        enabled = false,
        locked = false,
        controlsDisabled = false,
        inFlight = false,
        onToggle,
    }: {
        settings: ResistanceModeSettings | null;
        engineOnline: boolean;
        isPro?: boolean;
        enabled?: boolean;
        locked?: boolean;
        controlsDisabled?: boolean;
        inFlight?: boolean;
        onToggle: (enabled: boolean) => void | Promise<void>;
    } = $props();

    let confirmOpen = $state(false);

    const lockedEndText = $derived(
        settings?.isLocked && settings?.lockEnd
            ? formattedLockEnd(settings.lockEnd)
            : "",
    );

    const degradedReason = $derived.by(() => {
        if (!settings) return "";
        if (settings.enforcementStatus === "not_pro") return m.reason_not_pro();
        if (settings.enforcementStatus === "scripts_missing") {
            return m.reason_scripts_missing();
        }
        if (settings.enforcementStatus === "scripts_invalid") {
            return m.reason_scripts_invalid();
        }
        if (settings.enforcementStatus === "no_scripts_enabled") {
            return m.reason_no_scripts_enabled();
        }
        if (settings.enforcementStatus === "intensity_none") {
            return m.reason_intensity_none();
        }
        return m.status_on_degraded_description();
    });

    const statusKind = $derived.by(() => {
        if (!engineOnline) return "offline";
        if (!isPro) return "pro-required";
        if (!settings) return "checking";
        if (locked) return "locked";
        if (!settings.enabled) return "off";
        if (settings.enforcementStatus === "operational") return "operational";
        return "degraded";
    });

    function handleToggle(next: boolean) {
        if (!next) {
            confirmOpen = true;
            return;
        }
        void onToggle(true);
    }

    function confirmDisable() {
        confirmOpen = false;
        void onToggle(false);
    }
</script>

<section
    data-testid="rm-status-card"
    data-state={statusKind}
    class={[
        "resistance-state",
        "state-" + statusKind,
        locked && "locked",
    ]}
>
    <div class="state-rail" aria-hidden="true"></div>

    <div class="state-body">
        <div class="master-row">
            <div class="state-copy">
                <h2>{m.resistance_enforcement_title()}</h2>

                {#if statusKind === "checking"}
                    <div
                        class="loading-state"
                        role="status"
                        aria-live="polite"
                        aria-busy="true"
                    >
                        <span class="sr-only">{m.status_checking()}</span>
                        <Skeleton class="h-4 w-56 max-w-full rounded-sm" />
                    </div>
                {:else}
                    <p
                        data-testid="rm-status"
                        data-status={statusKind}
                        class="status-line"
                        role="status"
                        aria-live="polite"
                    >
                        {#if statusKind === "offline"}
                            <strong>{m.status_engine_offline()}</strong>
                            <span class="separator" aria-hidden="true">·</span>
                            <span>{m.status_engine_offline_description()}</span>
                        {:else if statusKind === "pro-required"}
                            <strong>{m.status_pro_required()}</strong>
                            <span class="separator" aria-hidden="true">·</span>
                            <span>{m.status_pro_required_description()}</span>
                        {:else if statusKind === "locked"}
                            <span data-testid="rm-locked-note">
                                {#if lockedEndText}
                                    <span data-testid="rm-lock-expiration">
                                        {m.status_locked_until({ datetime: lockedEndText })}
                                    </span>
                                {:else}
                                    {m.locked_controls_note()}
                                {/if}
                            </span>
                        {:else if statusKind === "operational"}
                            <strong>{m.status_on_operational()}</strong>
                            <span class="separator" aria-hidden="true">·</span>
                            <span>{m.status_on_operational_description()}</span>
                        {:else if statusKind === "degraded"}
                            <strong>{m.status_on_degraded()}</strong>
                            <span class="separator" aria-hidden="true">·</span>
                            <span>{degradedReason}</span>
                        {:else}
                            <strong>{m.status_off()}</strong>
                        {/if}
                    </p>
                {/if}
            </div>

            <Switch
                data-testid="rm-master-switch"
                checked={enabled}
                disabled={controlsDisabled || inFlight || (locked && enabled)}
                onCheckedChange={handleToggle}
                aria-label={m.resistance_mode()}
            />
        </div>
    </div>
</section>

<AlertDialog.Root bind:open={confirmOpen}>
    <AlertDialog.Content class="rounded-md border-border bg-card shadow-none">
        <AlertDialog.Header>
            <AlertDialog.Title class="font-condensed text-2xl font-semibold">
                {m.disable_confirmation_title()}
            </AlertDialog.Title>
            <AlertDialog.Description class="text-sm leading-relaxed">
                {m.disable_confirmation_description()}
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer class="mt-6 gap-2">
            <AlertDialog.Cancel>{m.cancel()}</AlertDialog.Cancel>
            <AlertDialog.Action
                data-testid="rm-disable-confirm"
                onclick={confirmDisable}
                disabled={inFlight}
                class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
                {m.confirm()}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

<style>
    .resistance-state {
        display: grid;
        grid-template-columns: 6px minmax(0, 1fr);
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 6px;
        background: var(--card);
    }

    .state-rail {
        background: var(--ace-ash);
    }

    .state-operational .state-rail {
        background: var(--ace-success);
    }

    .state-degraded .state-rail,
    .state-pro-required .state-rail {
        background: var(--ace-information);
    }

    .state-locked .state-rail {
        background: var(--ace-signal);
    }

    .state-offline .state-rail {
        background: var(--ace-danger);
    }

    .resistance-state.locked:not(.state-offline) {
        border-color: color-mix(in srgb, var(--ace-signal) 48%, var(--border));
    }

    .state-body,
    .state-copy {
        min-width: 0;
    }

    .master-row {
        min-height: 76px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 20px;
        padding: 14px 18px;
    }

    h2 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 19px;
        font-weight: 600;
        line-height: 1.1;
    }

    .status-line {
        display: flex;
        flex-wrap: wrap;
        align-items: baseline;
        gap: 5px;
        margin: 5px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    .status-line strong {
        color: var(--foreground);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 13px;
        font-weight: 600;
    }

    .state-operational .status-line strong {
        color: var(--ace-success);
    }

    .state-degraded .status-line strong,
    .state-pro-required .status-line strong {
        color: var(--ace-information);
    }

    .state-offline .status-line strong {
        color: var(--ace-danger);
    }

    .separator {
        color: var(--ace-ash);
    }

    .loading-state {
        max-width: 280px;
        margin-top: 8px;
    }
</style>
