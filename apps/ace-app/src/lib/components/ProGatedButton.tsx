import "./ProGatedButton.css";
import { createSignal, Show } from "solid-js";
import { Dynamic } from "@solidjs/web";
import type { LucideIcon } from "@lucide/solid";
import { openAccountPage } from "$lib/utils/account-browser";
import { ArrowUpRight, LockKeyhole } from "@lucide/solid";
import { cn } from "$lib/utils";
import * as m from "$lib/paraglide/messages.js";

export default function ProGatedButton(props: {
    isPro: boolean;
    label: string;
    icon?: LucideIcon;
    class?: string;
    onclick?: () => void;
    onClick?: () => void;
    upgradeOnly?: boolean;
}) {
    const [opening, setOpening] = createSignal(false);
    const gated = () => !props.isPro;
    async function handleClick() {
        if (opening()) return;
        if (!gated()) {
            (props.onclick ?? props.onClick)?.();
            return;
        }
        setOpening(true);
        try {
            await openAccountPage("pro");
        } finally {
            setOpening(false);
        }
    }
    return (
        <button
            type="button"
            onClick={handleClick}
            disabled={opening()}
            aria-busy={opening() ? "true" : "false"}
            aria-label={
                gated() && !props.upgradeOnly ? `${props.label} · ${m.get_pro()}` : props.label
            }
            class={cn(
                "pro-action",
                gated() && "pro-action--gated",
                props.upgradeOnly && "pro-action--upgrade",
                props.class,
            )}
        >
            <Show when={props.icon}>
                <Dynamic
                    component={props.icon!}
                    class="action-icon"
                    size={16}
                    aria-hidden="true"
                />
            </Show>
            <span class="action-label">{props.label}</span>
            <Show when={gated()}>
                <Show when={!props.upgradeOnly}>
                    <span class="pro-flag" aria-hidden="true">
                        <LockKeyhole size={12} />
                        {m.pro_badge()}
                    </span>
                </Show>
                <Show when={props.upgradeOnly}>
                    <ArrowUpRight size={15} class="upgrade-arrow" aria-hidden="true" />
                </Show>
            </Show>
        </button>
    );
}
