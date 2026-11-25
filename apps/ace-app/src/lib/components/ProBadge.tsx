import { cn } from "$lib/utils.js";
import * as m from "$lib/paraglide/messages.js";

export default function ProBadge(props: { class?: string }) {
    return (
        <span
            class={cn(
                "inline-flex shrink-0 items-center gap-1 font-mono text-[11px] font-medium text-[var(--ace-pro-text)]",
                props.class,
            )}
        >
            <span aria-hidden="true" class="size-2 rounded-sm bg-[var(--ace-pro)]" />
            {m.pro_badge()}
        </span>
    );
}
