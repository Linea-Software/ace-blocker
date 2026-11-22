import { cn } from "$lib/utils.js";
import * as m from "$lib/paraglide/messages.js";

export default function ProBadge(props: { class?: string }) {
  return <span class={cn("shrink-0 border-l-2 border-border pl-2 font-mono text-[11px] font-medium text-muted-foreground", props.class)}>{m.pro_badge()}</span>;
}
