<script lang="ts">
    import type { Snippet } from "svelte";
    import { X } from "@lucide/svelte";
    import { onMount } from "svelte";

    let {
        title,
        description,
        closeLabel,
        onClose,
        navigation,
        children,
        footer,
    }: {
        title: string;
        description?: string;
        closeLabel: string;
        onClose: () => void;
        navigation?: Snippet;
        children: Snippet;
        footer?: Snippet;
    } = $props();

    const uid = $props.id();
    let closeButton: HTMLButtonElement;

    onMount(() => {
        const previousFocus =
            document.activeElement instanceof HTMLElement ? document.activeElement : null;
        closeButton?.focus();

        return () => {
            if (previousFocus?.isConnected) previousFocus.focus();
        };
    });

    function handleEscape(event: KeyboardEvent) {
        if (
            event.key === "Escape" &&
            !event.defaultPrevented &&
            document.activeElement?.closest(".inspector")
        ) {
            event.preventDefault();
            onClose();
        }
    }
</script>

<svelte:window onkeydown={handleEscape} />

<aside class="inspector" role="region" aria-labelledby={uid + "-title"}>
    <header class="inspector-header">
        <div class="title-block">
            <h2 id={uid + "-title"}>{title}</h2>
            {#if description}
                <p>{description}</p>
            {/if}
        </div>
        <button
            type="button"
            class="close-button"
            bind:this={closeButton}
            onclick={onClose}
            aria-label={closeLabel}
            title={closeLabel}
        >
            <X size={18} aria-hidden="true" />
        </button>
    </header>

    {#if navigation}
        <div class="inspector-nav">
            {@render navigation()}
        </div>
    {/if}

    <div class="inspector-scroll ace-scrollbar">
        {@render children()}
    </div>

    {#if footer}
        <footer class="inspector-footer">
            {@render footer()}
        </footer>
    {/if}
</aside>

<style>
    .inspector {
        position: fixed;
        z-index: 40;
        top: 36px;
        right: 0;
        bottom: 0;
        width: min(560px, calc(100vw - 72px));
        display: grid;
        grid-template-rows: auto auto minmax(0, 1fr) auto;
        border-left: 1px solid var(--border);
        background: var(--background);
        container-type: inline-size;
        container-name: inspector;
        animation: inspector-in 160ms ease-out;
    }

    .inspector-header {
        min-width: 0;
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 20px;
        padding: 20px 22px 17px;
        border-bottom: 1px solid var(--border);
        background: var(--card);
    }

    .title-block {
        min-width: 0;
    }

    h2 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 26px;
        font-weight: 600;
        line-height: 1;
    }

    p {
        margin: 7px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    .close-button {
        width: 36px;
        height: 36px;
        flex: 0 0 36px;
        display: grid;
        place-items: center;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: transparent;
        color: var(--muted-foreground);
        cursor: pointer;
    }

    .close-button:hover {
        background: var(--muted);
        color: var(--foreground);
    }

    .inspector-nav {
        min-width: 0;
        border-bottom: 1px solid var(--border);
        background: color-mix(in srgb, var(--card) 65%, var(--background));
    }

    .inspector-scroll {
        min-height: 0;
        overflow: auto;
        padding: 22px;
    }

    .inspector-footer {
        padding: 14px 18px;
        border-top: 1px solid var(--border);
        background: var(--card);
    }

    @keyframes inspector-in {
        from {
            transform: translateX(18px);
            opacity: 0;
        }
        to {
            transform: translateX(0);
            opacity: 1;
        }
    }

    @media (max-width: 720px) {
        .inspector {
            width: calc(100vw - 62px);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .inspector {
            animation: none;
        }
    }
</style>
