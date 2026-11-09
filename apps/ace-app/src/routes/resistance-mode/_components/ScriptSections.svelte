<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { Button } from "$lib/components/ui/button";
    import { Switch } from "$lib/components/ui/switch";
    import { ChevronDown, ChevronUp, Download, Globe, TriangleAlert, ListFilter, PlaySquare, Palette, Hourglass, LockKeyhole } from "@lucide/svelte";
    import type { ScriptInfo, ResistanceModeSettings } from "$engine-client";
    import { groupInterventions, type InterventionKind } from "./intervention-groups";

    let {
        scripts, settings, engineOnline, isPro, locked, syncing = false, busy = false,
        onToggleScript, onToggleGroup, onSync,
    }: {
        scripts: ScriptInfo[];
        settings: ResistanceModeSettings | null;
        engineOnline: boolean;
        isPro: boolean;
        locked: boolean;
        syncing?: boolean;
        busy?: boolean;
        onToggleScript: (script: ScriptInfo) => void;
        onToggleGroup?: (scripts: ScriptInfo[], enabled: boolean) => void;
        onSync: () => void;
    } = $props();

    let expandedKind = $state<InterventionKind | null>("feeds");
    let expandedSite = $state<string | null>(null);
    const sections = $derived(groupInterventions(scripts));
    const selected = $derived(scripts.filter((script) => script.isEnabled).length);
    const missing = $derived(settings?.enforcementStatus === "scripts_missing");
    const invalid = $derived(settings?.enforcementStatus === "scripts_invalid");
    const editable = $derived(engineOnline && isPro && !busy);

    function title(kind: InterventionKind): string {
        switch (kind) {
            case "feeds": return m.interventions_feed();
            case "video": return m.interventions_video();
            case "visual": return m.interventions_visual();
            case "interruptions": return m.interventions_interruptions();
        }
    }
    function description(kind: InterventionKind): string {
        switch (kind) {
            case "feeds": return m.interventions_feed_desc();
            case "video": return m.interventions_video_desc();
            case "visual": return m.interventions_visual_desc();
            case "interruptions": return m.interventions_interruptions_desc();
        }
    }
</script>

<section class="space-y-4" aria-labelledby="interventions-title" data-testid="rm-interventions">
    <div class="flex items-start justify-between gap-4">
        <div>
            <h2 id="interventions-title" class="font-semibold text-xl">{m.interventions_title()}</h2>
            <p class="mt-1 max-w-2xl text-sm text-muted-foreground">{m.interventions_description()}</p>
        </div>
        {#if scripts.length}
            <span class="shrink-0 font-mono text-xs text-muted-foreground" data-testid="rm-selected-count">
                {m.interventions_selected_count({count:selected})}
            </span>
        {/if}
    </div>

    {#if settings && !settings.enabled && isPro}
        <p class="rounded border border-border bg-muted/40 px-3 py-2 text-sm text-muted-foreground" role="status">
            {m.interventions_off_note()}
        </p>
    {/if}
    {#if locked}
        <p class="flex items-center gap-2 rounded border border-border px-3 py-2 text-sm text-muted-foreground" role="status">
            <LockKeyhole size={16} aria-hidden="true" /> {m.interventions_lock_note()}
        </p>
    {/if}

    {#if !engineOnline}
        <p class="p-5 text-sm text-muted-foreground">{m.error_engine_offline_action()}</p>
    {:else if missing || invalid}
        <div class="flex flex-wrap items-center gap-3 rounded border border-border p-4" data-testid={missing ? "rm-scripts-empty" : "rm-scripts-invalid"}>
            {#if missing}<Download size={21} aria-hidden="true" />{:else}<TriangleAlert size={21} aria-hidden="true" />{/if}
            <div class="min-w-48 flex-1">
                <strong>{missing ? m.scripts_empty_title() : m.scripts_invalid_title()}</strong>
                <p class="text-sm text-muted-foreground">{missing ? m.scripts_empty_description() : m.scripts_invalid_description()}</p>
            </div>
            <Button data-testid="rm-sync-scripts" disabled={syncing || !isPro} onclick={onSync}>
                {syncing ? m.syncing_scripts() : m.sync_scripts_button()}
            </Button>
        </div>
    {:else if scripts.length === 0}
        <p class="p-5 text-sm text-muted-foreground">{m.error_scripts_unavailable()}</p>
    {:else}
        <div class="grid gap-2" data-testid="rm-category-list">
            {#each sections as section (section.kind)}
                {@const open = expandedKind === section.kind}
                <div class="overflow-hidden rounded-md border border-border bg-card" data-testid={"rm-category-" + section.kind}>
                    <button
                        type="button"
                        class="flex min-h-24 w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-muted/40 focus-visible:outline-2 focus-visible:outline-offset-[-3px]"
                        aria-expanded={open}
                        aria-label={m.interventions_expand_category({category:title(section.kind)})}
                        onclick={() => { expandedKind = open ? null : section.kind; expandedSite = null; }}
                    >
                        <span class="flex size-10 shrink-0 items-center justify-center rounded bg-muted text-foreground">
                            {#if section.kind === "feeds"}<ListFilter size={21} aria-hidden="true" />
                            {:else if section.kind === "video"}<PlaySquare size={21} aria-hidden="true" />
                            {:else if section.kind === "visual"}<Palette size={21} aria-hidden="true" />
                            {:else}<Hourglass size={21} aria-hidden="true" />{/if}
                        </span>
                        <span class="min-w-0 flex-1">
                            <strong class="block text-base font-semibold">{title(section.kind)}</strong>
                            <span class="mt-1 block text-sm leading-snug text-muted-foreground">{description(section.kind)}</span>
                        </span>
                        <span class="shrink-0 font-mono text-xs text-muted-foreground">{section.selectedCount}/{section.totalCount}</span>
                        {#if open}<ChevronUp size={17} aria-hidden="true" />{:else}<ChevronDown size={17} aria-hidden="true" />{/if}
                    </button>

                    {#if open}
                        <div class="border-t border-border px-4 pb-2">
                            {#if section.totalCount === 0}
                                <p class="py-4 text-sm text-muted-foreground">{m.interventions_empty_category()}</p>
                            {:else}
                                <div class="flex flex-wrap items-center justify-between gap-2 border-b border-border py-2 text-xs text-muted-foreground">
                                    <span>{m.interventions_sites_count({count:section.sites.length})}</span>
                                    {#if onToggleGroup && editable}
                                        <div class="flex items-center gap-4">
                                            <button type="button" class="underline underline-offset-4 disabled:opacity-40" disabled={section.selectedCount === section.totalCount} onclick={() => onToggleGroup?.(section.sites.flatMap(site => site.scripts),true)}>
                                                {m.interventions_enable_all()}
                                            </button>
                                            <button type="button" class="underline underline-offset-4 disabled:opacity-40" disabled={locked || section.selectedCount === 0} onclick={() => onToggleGroup?.(section.sites.flatMap(site => site.scripts),false)}>
                                                {m.interventions_disable_all()}
                                            </button>
                                        </div>
                                    {/if}
                                </div>
                                {#each section.sites as site (site.domain)}
                                    {@const siteKey = section.kind + ":" + site.domain}
                                    {@const siteOpen = expandedSite === siteKey}
                                    <div class="border-b border-border/60 last:border-b-0" data-testid="rm-site-row">
                                        <button
                                            type="button"
                                            class="flex min-h-12 w-full items-center gap-3 text-left hover:text-foreground focus-visible:outline-2"
                                            aria-expanded={siteOpen}
                                            aria-label={m.interventions_expand_site({domain:site.domain})}
                                            onclick={() => (expandedSite = siteOpen ? null : siteKey)}
                                        >
                                            <span class="flex size-6 shrink-0 items-center justify-center">
                                                {#if site.icon}
                                                    <span class="inline-flex size-5 items-center justify-center [&_svg]:size-5 [&_svg]:fill-current" data-site-brand-icon={site.icon.hex} style:color={site.icon.hex.toUpperCase() === "000000" ? "var(--foreground)" : "#" + site.icon.hex} aria-hidden="true">
                                                        {@html site.icon.svg}
                                                    </span>
                                                {:else}<Globe size={19} aria-hidden="true" />{/if}
                                            </span>
                                            <span class="min-w-0 flex-1 truncate text-sm font-medium">{site.domain}</span>
                                            <span class="font-mono text-xs text-muted-foreground">{site.selectedCount}/{site.totalCount}</span>
                                            {#if siteOpen}<ChevronUp size={15} aria-hidden="true" />{:else}<ChevronDown size={15} aria-hidden="true" />{/if}
                                        </button>
                                        {#if siteOpen}
                                            <div class="ml-8 border-l border-border pl-4 pb-3">
                                                {#each site.scripts as script (script.id)}
                                                    <div class="flex min-h-14 items-center gap-3 border-b border-border/60 py-3 last:border-b-0">
                                                        <div class="min-w-0 flex-1">
                                                            <strong class="block text-sm font-medium">{script.name}</strong>
                                                            {#if script.description}
                                                                <p class="mt-1 text-xs leading-relaxed text-muted-foreground">{script.description}</p>
                                                            {/if}
                                                        </div>
                                                        <Switch data-testid="rm-script-switch" checked={script.isEnabled} disabled={!editable || (locked && script.isEnabled)} onCheckedChange={() => onToggleScript(script)} aria-label={script.name} />
                                                    </div>
                                                {/each}
                                            </div>
                                        {/if}
                                    </div>
                                {/each}
                            {/if}
                        </div>
                    {/if}
                </div>
            {/each}
        </div>
    {/if}

    {#if scripts.length && !missing && !invalid}
        <div class="flex flex-wrap items-center justify-between gap-3 pt-2 text-xs text-muted-foreground" data-testid="rm-operational-details">
            <span>{settings?.scriptsVersion ? m.last_scripts_update({version: settings.scriptsVersion}) : m.scripts_never_updated()}</span>
            <Button size="sm" variant="outline" disabled={!editable || syncing} onclick={onSync} data-testid="rm-sync-scripts">
                {syncing ? m.syncing_scripts() : m.sync_scripts_button()}
            </Button>
        </div>
    {/if}
</section>
