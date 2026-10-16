<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { Button } from "$lib/components/ui/button";
    import { Switch } from "$lib/components/ui/switch";
    import {
        ChevronDown,
        ChevronUp,
        Download,
        Globe,
        TriangleAlert,
    } from "@lucide/svelte";
    import { cn } from "$lib/utils";
    import type { ScriptInfo, ResistanceModeSettings } from "$engine-client";
    import { groupScripts, type ScriptGroup } from "./script-groups";

    let {
        scripts,
        settings,
        engineOnline,
        isPro,
        locked,
        syncing = false,
        onToggleScript,
        onToggleDomain,
        onSync,
    }: {
        scripts: ScriptInfo[];
        settings: ResistanceModeSettings | null;
        engineOnline: boolean;
        isPro: boolean;
        locked: boolean;
        syncing?: boolean;
        onToggleScript: (script: ScriptInfo) => void;
        onToggleDomain: (domain: string, enabled: boolean) => void;
        onSync: () => void;
    } = $props();

    let expandedDomain = $state<string | null>(null);

    const groups = $derived(groupScripts(scripts));
    const enabledCount = $derived(scripts.filter((s) => s.isEnabled).length);
    const scriptsMissing = $derived(
        settings?.enforcementStatus === "scripts_missing",
    );
    const scriptsInvalid = $derived(
        settings?.enforcementStatus === "scripts_invalid",
    );

    function toggleExpand(domain: string) {
        expandedDomain = expandedDomain === domain ? null : domain;
    }
</script>

<div class="space-y-4">
    <div class="flex items-center justify-between gap-3">
        <div>
            <h2 class="text-base font-medium text-foreground">
                {m.covered_sites_title()}
            </h2>
            <p class="mt-0.5 text-sm text-muted-foreground">
                {m.covered_sites_hint()}
            </p>
        </div>
        {#if scripts.length > 0}
            <span class="shrink-0 font-mono text-xs text-muted-foreground">
                {m.covered_sites_active_count({ count: enabledCount })}
            </span>
        {/if}
    </div>

    {#if !engineOnline}
        <div
            class="rounded-md border border-dashed border-border bg-muted/25 p-8 text-center"
        >
            <p class="text-sm text-muted-foreground">
                {m.error_engine_offline_action()}
            </p>
        </div>
    {:else if scriptsMissing}
        <div
            data-testid="rm-scripts-empty"
            class="flex flex-wrap items-center gap-3 rounded-md border border-dashed border-border bg-muted/25 px-4 py-4 text-left"
        >
            <Download class="h-5 w-5 shrink-0 text-muted-foreground" aria-hidden="true" />
            <div class="min-w-[220px] flex-1">
                <h3 class="font-semibold text-foreground">
                    {m.scripts_empty_title()}
                </h3>
                <p class="mt-1 text-sm text-muted-foreground">
                    {m.scripts_empty_description()}
                </p>
            </div>
            <Button
                data-testid="rm-sync-scripts"
                class="shrink-0"
                disabled={syncing || !isPro}
                onclick={onSync}
            >
                {syncing ? m.syncing_scripts() : m.sync_scripts_button()}
            </Button>
        </div>
    {:else if scriptsInvalid}
        <div
            data-testid="rm-scripts-invalid"
            class="flex flex-wrap items-center gap-3 rounded-md border border-dashed border-border bg-muted/25 px-4 py-4 text-left"
        >
            <TriangleAlert class="h-5 w-5 shrink-0 text-[var(--ace-information)]" aria-hidden="true" />
            <div class="min-w-[220px] flex-1">
                <h3 class="font-semibold text-foreground">
                    {m.scripts_invalid_title()}
                </h3>
                <p class="mt-1 text-sm text-muted-foreground">
                    {m.scripts_invalid_description()}
                </p>
            </div>
            <Button
                data-testid="rm-sync-scripts"
                class="shrink-0"
                disabled={syncing || !isPro}
                onclick={onSync}
            >
                {syncing ? m.syncing_scripts() : m.sync_scripts_button()}
            </Button>
        </div>
    {:else if scripts.length === 0}
        <div
            class="rounded-md border border-dashed border-border bg-muted/25 px-4 py-4"
        >
            <p class="text-sm text-muted-foreground">
                {m.error_scripts_unavailable()}
            </p>
        </div>
    {:else}
        <div class="space-y-2.5">
            {#each groups as group (group.domain)}
                {@const isExpanded = expandedDomain === group.domain}
                <div
                    class="overflow-hidden rounded-md border border-border bg-card"
                >
                    <div class="flex items-center gap-3 p-3.5">
                        <div
                            class="flex h-9 w-9 shrink-0 items-center justify-center rounded-sm border border-border bg-muted"
                        >
                            <Globe class="h-5 w-5 text-muted-foreground" aria-hidden="true" />
                        </div>
                        <button
                            type="button"
                            class="flex min-w-0 flex-1 items-center gap-3 text-left"
                            onclick={() => toggleExpand(group.domain)}
                            aria-expanded={isExpanded}
                            aria-label={group.domain}
                        >
                            <span
                                class="min-w-0 flex-1 truncate text-sm font-medium text-foreground"
                                >{group.domain}</span
                            >
                            <span
                                class="shrink-0 font-mono text-[11px] font-medium text-muted-foreground"
                                >{m.group_active_count({
                                    count: group.activeCount,
                                })}</span
                            >
                            {#if isExpanded}
                                <ChevronUp class="h-4 w-4 shrink-0 text-muted-foreground" />
                            {:else}
                                <ChevronDown class="h-4 w-4 shrink-0 text-muted-foreground" />
                            {/if}
                        </button>
                        <Switch
                            data-testid="rm-domain-switch"
                            checked={group.activeCount === group.totalCount}
                            disabled={locked}
                            onCheckedChange={(value) =>
                                onToggleDomain(group.domain, value)}
                            aria-label={m.friction_for_domain({
                                domain: group.domain,
                            })}
                        />
                    </div>
                    {#if isExpanded}
                        <div
                            class="grid grid-cols-1 gap-3 border-t border-border p-3.5 sm:grid-cols-2 lg:grid-cols-3"
                        >
                            {#each group.scripts as script (script.id)}
                                <div
                                    class="flex items-start justify-between gap-3 rounded-sm border border-border bg-muted/50 p-3"
                                >
                                    <div class="min-w-0 space-y-1">
                                        <p
                                            class="text-sm font-medium text-foreground"
                                        >
                                            {script.name}
                                        </p>
                                        {#if script.description}
                                            <p
                                                class="text-xs leading-relaxed text-muted-foreground"
                                            >
                                                {script.description}
                                            </p>
                                        {/if}
                                    </div>
                                    <Switch
                                        data-testid="rm-script-switch"
                                        checked={script.isEnabled}
                                        disabled={locked}
                                        onCheckedChange={() =>
                                            onToggleScript(script)}
                                        aria-label={script.name}
                                        class="mt-1 shrink-0"
                                    />
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>
            {/each}
        </div>
    {/if}

    {#if scripts.length > 0 && !scriptsMissing && !scriptsInvalid}
        <div
            data-testid="rm-operational-details"
            class="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-3"
        >
            <p class="text-sm text-muted-foreground">
                {#if settings?.scriptsVersion}
                    {m.last_scripts_update({ version: settings.scriptsVersion })}
                {:else}
                    {m.scripts_never_updated()}
                {/if}
            </p>
            <Button
                data-testid="rm-sync-scripts"
                variant="outline"
                size="sm"
                disabled={syncing || !isPro}
                onclick={onSync}
            >
                {syncing ? m.syncing_scripts() : m.sync_scripts_button()}
            </Button>
        </div>
    {/if}
</div>
