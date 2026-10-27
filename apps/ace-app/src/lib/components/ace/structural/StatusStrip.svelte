<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { engineStore } from "$lib/stores/engine.svelte";
    import { AlertTriangle } from "@lucide/svelte";
</script>

{#if engineStore.isTauri && engineStore.isReady && !engineStore.isAlive}
    <div class="status-strip" role="alert">
        <AlertTriangle size={16} aria-hidden="true" />
        <strong>{m.status_engine_offline()}</strong>
        <span>{m.status_engine_offline_description()}</span>
    </div>
{/if}

<style>
    .status-strip {
        min-height: 40px;
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 22px;
        border-bottom: 1px solid var(--border);
        border-left: 3px solid var(--ace-danger);
        background: color-mix(in srgb, var(--ace-danger) 8%, var(--background));
        color: var(--foreground);
    }

    .status-strip :global(svg) {
        flex-shrink: 0;
        color: var(--ace-danger);
    }

    strong {
        flex-shrink: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 14px;
        font-weight: 600;
    }

    span {
        min-width: 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    @media (max-width: 560px) {
        .status-strip {
            align-items: flex-start;
            padding-inline: 12px;
            flex-wrap: wrap;
        }

        span {
            flex-basis: 100%;
            padding-left: 26px;
        }
    }
</style>
