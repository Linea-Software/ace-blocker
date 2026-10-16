<script lang="ts">
    import type { Snippet } from "svelte";
    import { cn } from "$lib/utils";

    let {
        title,
        subtitle,
        align = "end",
        mobileCentered = false,
        class: className,
        children,
    }: {
        title: string;
        subtitle?: string;
        align?: "center" | "end";
        mobileCentered?: boolean;
        class?: string;
        children?: Snippet;
    } = $props();
</script>

<div
    class={cn(
        "page-header",
        align === "end" ? "page-header--end" : "page-header--center",
        className,
    )}
>
    <div
        class={cn(
            "page-header__identity",
            mobileCentered && "page-header__identity--mobile-centered",
        )}
    >
        <div class="page-header__copy">
            <h1>{title}</h1>
            {#if subtitle}
                <p>{subtitle}</p>
            {/if}
        </div>
    </div>

    {#if children}
        <div class="page-header__actions">
            {@render children()}
        </div>
    {/if}
</div>

<style>
    .page-header {
        display: flex;
        justify-content: space-between;
        gap: 24px;
    }

    .page-header--end {
        align-items: flex-end;
    }

    .page-header--center {
        align-items: center;
    }

    .page-header__identity {
        min-width: 0;
    }

    .page-header__copy {
        min-width: 0;
    }

    h1 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: clamp(32px, 5cqi, 42px);
        font-weight: 600;
        line-height: 0.98;
        letter-spacing: -0.02em;
    }

    p {
        max-width: 680px;
        margin: 8px 0 0;
        color: var(--muted-foreground);
        font-size: 14px;
        line-height: 1.5;
    }

    .page-header__actions {
        flex: 0 0 auto;
        display: flex;
        align-items: center;
        gap: 8px;
    }

    @container page (max-width: 620px) {
        .page-header {
            align-items: stretch;
            flex-direction: column;
            gap: 16px;
        }

        .page-header__actions {
            width: 100%;
            flex-wrap: wrap;
        }
    }
</style>
