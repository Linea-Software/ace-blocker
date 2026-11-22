import { reactiveFields } from "$lib/stores/reactive-fields";
import * as m from "$lib/paraglide/messages.js";
import { Switch } from "$lib/components/ui/switch/solid";
import * as AlertDialog from "$lib/components/ui/alert-dialog/solid";
import { Skeleton } from "$lib/components/ui/skeleton/solid";
import type { ResistanceModeSettings } from "$engine-client";
import { formattedLockEnd } from "./lock-format";
import "./StatusCard.css";

type Props = { settings: ResistanceModeSettings | null; engineOnline: boolean; isPro?: boolean; enabled?: boolean; locked?: boolean; controlsDisabled?: boolean; inFlight?: boolean; onToggle: (enabled:boolean) => void | Promise<void>; };
export default function StatusCard(props: Props) {
 const state = { confirmOpen: false };
 reactiveFields(state, ["confirmOpen"]);
 const lockedEndText = () => props.settings?.isLocked && props.settings?.lockEnd ? formattedLockEnd(props.settings.lockEnd) : "";
 const degradedReason = () => {
  const settings = props.settings;
  if (!settings) return "";
  if (settings.enforcementStatus === "not_pro") return m.reason_not_pro();
  if (settings.enforcementStatus === "scripts_missing") return m.reason_scripts_missing();
  if (settings.enforcementStatus === "scripts_invalid") return m.reason_scripts_invalid();
  if (settings.enforcementStatus === "no_scripts_enabled") return m.reason_no_scripts_enabled();
  return m.status_on_degraded_description();
 };
 const statusKind = () => {
  if (!props.engineOnline) return "offline";
  if (!(props.isPro ?? true)) return "pro-required";
  if (!props.settings) return "checking";
  if (props.locked) return "locked";
  if (!props.settings.enabled) return "off";
  if (props.settings.enforcementStatus === "operational") return "operational";
  return "degraded";
 };
 function handleToggle(next: boolean) {
  if (!next) { state.confirmOpen = true; return; }
  void props.onToggle(true);
 }
 function confirmDisable() {
  state.confirmOpen = false;
  void props.onToggle(false);
 }
 return <>
<section
    data-testid="rm-status-card"
    data-state={statusKind()}
    class={[
        "resistance-state",
        "state-" + statusKind(),
        props.locked && "props.locked",
    ]}
>
    <div class="state-rail" aria-hidden="true"></div>

    <div class="state-body">
        <div class="master-row">
            <div class="state-copy">
                <h2>{m.resistance_enforcement_title()}</h2>

                {statusKind() === "checking" ? (<>
                    <div
                        class="loading-state"
                        role="status"
                        aria-live="polite"
                        aria-busy="true"
                    >
                        <span class="sr-only">{m.status_checking()}</span>
                        <Skeleton class="h-4 w-56 max-w-full rounded-sm" />
                    </div>
                </>) : (<>
                    <p
                        data-testid="rm-status"
                        data-status={statusKind()}
                        class="status-line"
                        role="status"
                        aria-live="polite"
                    >
                        {statusKind() === "offline" ? (<>
                            <strong>{m.status_engine_offline()}</strong>
                            <span class="separator" aria-hidden="true">·</span>
                            <span>{m.status_engine_offline_description()}</span>
                        </>) : statusKind() === "pro-required" ? (<>
                            <strong>{m.status_pro_required()}</strong>
                            <span class="separator" aria-hidden="true">·</span>
                            <span>{m.status_pro_required_description()}</span>
                        </>) : statusKind() === "locked" ? (<>
                            <span data-testid="rm-locked-note">
                                {lockedEndText() ? (<>
                                    <span data-testid="rm-lock-expiration">
                                        {m.status_locked_until({ datetime: lockedEndText() })}
                                    </span>
                                </>) : (<>
                                    {m.locked_controls_note()}
                                </>)}
                            </span>
                        </>) : statusKind() === "operational" ? (<>
                            <strong>{m.status_on_operational()}</strong>
                            <span class="separator" aria-hidden="true">·</span>
                            <span>{m.status_on_operational_description()}</span>
                        </>) : statusKind() === "degraded" ? (<>
                            <strong>{m.status_on_degraded()}</strong>
                            <span class="separator" aria-hidden="true">·</span>
                            <span>{degradedReason()}</span>
                        </>) : (<>
                            <strong>{m.status_off()}</strong>
                        </>)}
                    </p>
                </>)}
            </div>

            <Switch
                data-testid="rm-master-switch"
                checked={props.enabled}
                disabled={props.controlsDisabled || props.inFlight || (props.locked && props.enabled)}
                onCheckedChange={handleToggle}
                aria-label={m.resistance_mode()}
            />
        </div>
    </div>
</section>

<AlertDialog.Root open={state.confirmOpen} onOpenChange={(open: boolean) => { state.confirmOpen = open; }}>
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
                onClick={confirmDisable}
                disabled={props.inFlight}
                class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
                {m.confirm()}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>
 </>;
}
