<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { countMessage } from "$lib/utils/count-message";
    import { engineStore } from "$lib/stores/engine.svelte";
    import { dashboardStore } from "$lib/stores/dashboard.svelte";
    import { onMount } from "svelte";

    const online = $derived(engineStore.isAlive);
    const enforcing = $derived(online && dashboardStore.activeBlocksCount > 0);

    const title = $derived.by(() => {
        if (!online) return m.status_engine_offline();
        return enforcing ? m.status_enforcement_active() : m.status_engine_ready();
    });

    const description = $derived.by(() => {
        if (!online) return m.status_engine_offline_description();
        if (enforcing) {
            return countMessage(
                dashboardStore.activeBlocksCount,
                m.status_enforcement_active_description_zero,
                m.status_enforcement_active_description_one,
                () => m.status_enforcement_active_description({ count: dashboardStore.activeBlocksCount }),
            );
        }
        return m.status_engine_ready_description();
    });

    onMount(() => {
        void dashboardStore.refresh();
        const interval = setInterval(() => {
            if (engineStore.isAlive) void dashboardStore.refresh();
        }, 5000);
        return () => clearInterval(interval);
    });
</script>

<div class:offline={!online} class:enforcing class="engine-state" role="status">
    <span class="state-mark" aria-hidden="true"></span>
    <span class="state-copy">
        <strong>{title}</strong>
        <span>{description}</span>
    </span>
</div>

<style>
    .engine-state {
        min-width: 0;
        display: flex;
        align-items: center;
        gap: 9px;
        color: var(--foreground);
    }

    .state-mark {
        width: 8px;
        height: 8px;
        flex: 0 0 8px;
        border-radius: 50%;
        background: var(--ace-ash);
    }

    .engine-state.enforcing .state-mark {
        background: var(--ace-success);
    }

    .engine-state.offline .state-mark {
        background: var(--ace-danger);
    }

    .state-copy {
        min-width: 0;
        display: grid;
        grid-template-columns: max-content minmax(0, 1fr);
        align-items: baseline;
        gap: 8px;
    }

    strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 13px;
        font-weight: 600;
    }

    .state-copy > span {
        overflow: hidden;
        color: var(--muted-foreground);
        font-size: 12px;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    @media (max-width: 920px) {
        .state-copy > span {
            display: none;
        }
    }
</style>
