import { cn } from "$lib/utils.js";
import * as m from "$lib/paraglide/messages.js";

export default function ProBadge(props: { class?: string }) {
    return (
        <span
            class={cn(
                "shrink-0 font-mono text-[11px] font-medium text-[var(--ace-pro)]",
                props.class,
            )}
        >
            {m.pro_badge()}
        </span>
    );
}
