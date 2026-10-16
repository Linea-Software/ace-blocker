<script lang="ts">
    import { cn } from "$lib/utils.js";
    import * as m from "$lib/paraglide/messages.js";

    let { class: className = "" } = $props();
</script>

<span
    class={cn(
        "shrink-0 border-l-2 border-border pl-2 font-mono text-[11px] font-medium text-muted-foreground",
        className,
    )}
>
    {m.pro_badge()}
</span>
