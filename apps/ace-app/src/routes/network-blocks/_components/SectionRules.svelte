<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Trash2, Plus } from "@lucide/svelte";
    import CircleAlert from "@lucide/svelte/icons/circle-alert";
    import * as m from "$lib/paraglide/messages.js";
    import type { UserDefinedRulesDelta } from "$lib/deltas/user-defined-rules-delta.svelte.ts";
    import type { TempId } from "$lib/deltas/common";
    import { websiteHostname, parseWebsiteList } from "./website-input";

    let {
        delta,
        disabled = false,
        isLocked = false,
    }: {
        delta: UserDefinedRulesDelta;
        disabled?: boolean;
        isLocked?: boolean;
    } = $props();

    let domainInput = $state("");
    let regexInput = $state("");
    let bulkInput = $state("");
    let bulkOpen = $state(false);
    let showAllWebsites = $state(false);

    let domainValidation = $derived.by(() => {
        const input = domainInput.trim();
        if (!input) return { domain: null, error: null };
        const domain = websiteHostname(input);
        if (!domain) {
            return { domain: null, error: m.error_invalid_domain_format() };
        }
        if (delta.currentRules.some(
            (item) => item.ruleType === "domain" && item.rule.toLowerCase() === domain,
        )) {
            return { domain: null, error: m.website_already_added() };
        }
        return { domain, error: null };
    });

    let activeRules = $derived(
        delta.currentRules.map((rule) => ({
            id: rule.id,
            rule: rule.rule,
            isTemp: !delta.isInitialRule(rule.id),
            type: rule.ruleType,
        })),
    );
    const bulkParsed = $derived(parseWebsiteList(bulkInput, activeRules.filter((item) => item.type === "domain").map((item) => item.rule)));
    let domainRules = $derived(
        activeRules.filter((rule) => rule.type === "domain"),
    );
    let regexRules = $derived(
        activeRules.filter((rule) => rule.type === "regex"),
    );

    function addDomainRule() {
        if (!domainValidation.domain) return;
        delta.addRule(domainValidation.domain, "domain");
        domainInput = "";
    }

    function addBulkRules() {
        for (const site of bulkParsed.websites) delta.addRule(site, "domain");
        bulkInput = "";
        bulkOpen = false;
    }

    function addRegexRule() {
        const value = regexInput.trim();
        if (!value) return;
        delta.addRule(value, "regex");
        regexInput = "";
    }

    function removeRule(id: string | TempId, isTemp: boolean) {
        if (isTemp) {
            delta.removeTempRule(id as TempId);
        } else {
            delta.removeRule(id.toString());
        }
    }
</script>

<div class="space-y-4" id="rules">
    <div>
        <h3 class="font-condensed text-base font-semibold text-foreground">
            {m.custom_rules()}
        </h3>
        <p class="text-muted-foreground mt-0.5 text-sm leading-relaxed">
            {m.section_rules_description()}
        </p>
    </div>

    <div class="space-y-2">
        <div class="flex flex-wrap gap-2">
            <Input
                placeholder={m.website_url_placeholder()}
                bind:value={domainInput}
                onkeydown={(event) =>
                    event.key === "Enter" && addDomainRule()}
                class="bg-card h-11 min-w-44 flex-1 rounded-sm"
                aria-label={m.specific_website_input_label()}
                {disabled}
            />
            <Button
                onclick={addDomainRule}
                disabled={disabled || !domainValidation.domain}
                class="h-11 shrink-0 gap-2 rounded-sm shadow-none"
                aria-label={m.add_specific_website()}
            >
                <Plus class="size-4" aria-hidden="true" />
                {m.add_specific_website()}
            </Button>
        </div>

        {#if domainValidation.error}
            <p class="text-destructive px-1 text-xs">{domainValidation.error}</p>
        {:else}
            <p class="text-muted-foreground px-1 text-xs leading-relaxed">
                {m.website_url_scope_hint()}
            </p>
        {/if}
    </div>

    {#if domainRules.length > 0}
        <div class="border-border divide-border divide-y border-y">
            {#each (showAllWebsites ? domainRules : domainRules.slice(0, 4)) as item (item.id)}
                <div class="group flex min-w-0 items-center gap-2 py-2">
                    <code
                        class="min-w-0 flex-1 truncate font-mono text-xs"
                        title={item.rule}>{item.rule}</code
                    >
                    <Button
                        variant="ghost"
                        size="icon"
                        class="text-muted-foreground hover:text-destructive size-7 shrink-0"
                        title={!item.isTemp && isLocked
                            ? m.cannot_remove_rule_locked()
                            : m.delete()}
                        aria-label={!item.isTemp && isLocked
                            ? m.cannot_remove_rule_locked()
                            : m.delete()}
                        disabled={!item.isTemp && isLocked}
                        onclick={() => removeRule(item.id as string | TempId, item.isTemp)}
                    >
                        <Trash2 class="size-3.5" aria-hidden="true" />
                    </Button>
                </div>
            {/each}
        </div>
        {#if domainRules.length > 4}
            <button type="button" class="text-sm font-medium underline underline-offset-4"
                onclick={() => (showAllWebsites = !showAllWebsites)}>
                {showAllWebsites ? m.block_show_less_sites() : m.block_show_more_sites({ count: domainRules.length - 4 })}
            </button>
        {/if}
    {:else}
        <p class="text-muted-foreground text-xs">{m.no_rules_added()}</p>
    {/if}

    <div class="border-border border-t pt-3">
        <button type="button" class="text-sm font-medium underline underline-offset-4"
            aria-expanded={bulkOpen} onclick={() => (bulkOpen = !bulkOpen)}>
            {m.block_bulk_add_title()}
        </button>
        {#if bulkOpen}
            <div class="mt-3 space-y-2">
                <textarea class="bg-card border-border w-full rounded-sm border p-3 text-sm"
                    rows="4" bind:value={bulkInput}
                    placeholder={m.block_bulk_add_hint()} disabled={disabled}
                    aria-label={m.block_bulk_add_title()}></textarea>
                <p class="text-muted-foreground text-xs">{bulkParsed.websites.length} · {bulkParsed.skipped.length} {m.block_bulk_skipped()}</p>
                <Button onclick={addBulkRules} disabled={disabled || bulkParsed.websites.length === 0}>
                    {m.block_bulk_add_action()}
                </Button>
            </div>
        {/if}
    </div>

    <details class="border-border border-t pt-3">
        <summary
            class="text-muted-foreground hover:text-foreground cursor-pointer select-none text-sm font-medium transition-colors"
        >
            {m.use_regex()}
        </summary>

        <div class="mt-3 space-y-3">
            <p class="text-muted-foreground text-xs leading-relaxed">
                {m.advanced_matching_description()}
            </p>

            <div class="flex gap-2">
                <Input
                    placeholder={m.advanced_mode_placeholder()}
                    bind:value={regexInput}
                    onkeydown={(event) =>
                        event.key === "Enter" && addRegexRule()}
                    class="bg-card rounded-sm font-mono text-xs"
                    aria-label={m.advanced_matching_input_label()}
                    {disabled}
                />
                <Button
                    onclick={addRegexRule}
                    size="icon"
                    disabled={disabled || !regexInput.trim()}
                    class="shrink-0 rounded-sm shadow-none"
                    aria-label={m.add_advanced_match()}
                >
                    <Plus class="size-4" aria-hidden="true" />
                </Button>
            </div>

            <div
                class="text-muted-foreground flex items-start gap-1.5 text-xs leading-relaxed"
            >
                <CircleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
                <span>{m.advanced_mode_warning()}</span>
            </div>

            {#if regexRules.length > 0}
                <div class="border-border divide-border divide-y border-y">
                    {#each regexRules as item (item.id)}
                        <div class="group flex min-w-0 items-center gap-2 py-2">
                            <code
                                class="min-w-0 flex-1 truncate font-mono text-xs"
                                title={item.rule}>{item.rule}</code
                            >
                            <Button
                                variant="ghost"
                                size="icon"
                                class="text-muted-foreground hover:text-destructive size-7 shrink-0"
                                title={!item.isTemp && isLocked
                                    ? m.cannot_remove_rule_locked()
                                    : m.delete()}
                                aria-label={!item.isTemp && isLocked
                                    ? m.cannot_remove_rule_locked()
                                    : m.delete()}
                                disabled={!item.isTemp && isLocked}
                                onclick={() =>
                                    removeRule(
                                        item.id as string | TempId,
                                        item.isTemp,
                                    )}
                            >
                                <Trash2 class="size-3.5" aria-hidden="true" />
                            </Button>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    </details>
</div>
