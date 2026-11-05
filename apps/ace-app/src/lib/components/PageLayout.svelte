<script lang="ts">
    import type { Snippet } from "svelte";
    import { cn } from "$lib/utils";

    let {
        width = "wide",
        class: className,
        header,
        children,
        ...restProps
    }: {
        width?: "wide" | "narrow" | "dashboard";
        header: Snippet;
        children: Snippet;
        class?: string;
        [key: string]: unknown;
    } = $props();
</script>

<div
    class={cn(
        "page-layout",
        width === "narrow" && "page-layout--narrow",
        width === "dashboard" && "page-layout--dashboard",
        className,
    )}
    {...restProps}
>
    <div class="page-header-slot">
        {@render header()}
    </div>
    <div class="page-content">
        {@render children()}
    </div>
</div>

<style>
    .page-layout {
        width: min(1120px, calc(100% - 44px));
        margin: 0 auto;
        padding: 30px 0 64px;
        container-type: inline-size;
        container-name: page;
    }

    .page-layout--narrow {
        max-width: 920px;
    }

    .page-layout--dashboard {
        max-width: 1280px;
    }

    .page-header-slot {
        padding-bottom: 18px;
        border-bottom: 1px solid var(--border);
    }

    .page-content {
        padding-top: 18px;
    }

    @container ace-workspace (max-width: 760px) {
        .page-layout {
            width: calc(100% - 28px);
            padding-top: 24px;
        }
    }
</style>
