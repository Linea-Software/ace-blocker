import { createMemo, createSignal, For, Show } from "solid-js";
import { Button } from "$lib/components/ui/button/solid";
import { Input } from "$lib/components/ui/input/solid";
import { Trash2, Plus, CircleAlert } from "@lucide/solid";
import * as m from "$lib/paraglide/messages.js";
import type { UserDefinedRulesDelta } from "$lib/deltas/user-defined-rules-delta";
import type { TempId } from "$lib/deltas/common";
import { websiteHostname, parseWebsiteList } from "./website-input";

export interface SectionRulesProps {
    delta: UserDefinedRulesDelta;
    disabled?: boolean;
    isLocked?: boolean;
}

const WEBSITES_PER_PAGE = 20;

export default function SectionRules(props: SectionRulesProps) {
    const [domainInput, setDomainInput] = createSignal("");
    const [regexInput, setRegexInput] = createSignal("");
    const [bulkInput, setBulkInput] = createSignal("");
    const [bulkOpen, setBulkOpen] = createSignal(false);
    const [websiteSearch, setWebsiteSearch] = createSignal("");
    const [websitePage, setWebsitePage] = createSignal(0);

    const domainValidation = createMemo(() => {
        const input = domainInput().trim();
        if (!input) return { domain: null, error: null };
        const domain = websiteHostname(input);
        if (!domain)
            return { domain: null, error: m.error_invalid_domain_format() };
        if (
            props.delta.currentRules.some(
                (item) =>
                    item.ruleType === "domain" &&
                    item.rule.toLowerCase() === domain,
            )
        ) {
            return { domain: null, error: m.website_already_added() };
        }
        return { domain, error: null };
    });
    const activeRules = createMemo(() =>
        props.delta.currentRules.map((rule) => ({
            id: rule.id,
            rule: rule.rule,
            isTemp: !props.delta.isInitialRule(rule.id),
            type: rule.ruleType,
        })),
    );
    const bulkParsed = createMemo(() =>
        parseWebsiteList(
            bulkInput(),
            activeRules()
                .filter((item) => item.type === "domain")
                .map((item) => item.rule),
        ),
    );
    const domainRules = createMemo(() =>
        activeRules().filter((item) => item.type === "domain"),
    );
    const filteredWebsites = createMemo(() =>
        domainRules().filter((item) =>
            item.rule
                .toLowerCase()
                .includes(websiteSearch().trim().toLowerCase()),
        ),
    );
    const websitePages = createMemo(() =>
        Math.max(1, Math.ceil(filteredWebsites().length / WEBSITES_PER_PAGE)),
    );
    const displayedWebsitePage = createMemo(() =>
        Math.min(websitePage(), websitePages() - 1),
    );
    const visibleWebsites = createMemo(() =>
        filteredWebsites().slice(
            displayedWebsitePage() * WEBSITES_PER_PAGE,
            (displayedWebsitePage() + 1) * WEBSITES_PER_PAGE,
        ),
    );
    const regexRules = createMemo(() =>
        activeRules().filter((item) => item.type === "regex"),
    );

    function addDomainRule() {
        const domain = domainValidation().domain;
        if (!domain || props.disabled) return;
        props.delta.addRule(domain, "domain");
        setDomainInput("");
    }
    function addBulkRules() {
        if (props.disabled) return;
        for (const site of bulkParsed().websites)
            props.delta.addRule(site, "domain");
        setBulkInput("");
        setBulkOpen(false);
    }
    function addRegexRule() {
        const value = regexInput().trim();
        if (!value || props.disabled) return;
        props.delta.addRule(value, "regex");
        setRegexInput("");
    }
    function removeRule(id: string | TempId, isTemp: boolean) {
        if (isTemp) props.delta.removeTempRule(id as TempId);
        else if (!props.isLocked) props.delta.removeRule(id.toString());
    }
    const renderRule = (item: {
        id: string;
        rule: string;
        isTemp: boolean;
    }) => (
        <div class="group flex min-w-0 items-center gap-2 py-2">
            <code
                class="min-w-0 flex-1 truncate font-mono text-xs"
                title={item.rule}
            >
                {item.rule}
            </code>
            <Button
                variant="ghost"
                size="icon"
                class="text-muted-foreground hover:text-destructive size-7 shrink-0"
                title={
                    !item.isTemp && props.isLocked
                        ? m.cannot_remove_rule_locked()
                        : m.delete()
                }
                aria-label={
                    !item.isTemp && props.isLocked
                        ? m.cannot_remove_rule_locked()
                        : m.delete()
                }
                disabled={
                    !!props.disabled || (!item.isTemp && !!props.isLocked)
                }
                onClick={() => removeRule(item.id, item.isTemp)}
            >
                <Trash2 class="size-3.5" aria-hidden="true" />
            </Button>
        </div>
    );

    return (
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
                        value={domainInput()}
                        onInput={(event) =>
                            setDomainInput(event.currentTarget.value)
                        }
                        onKeyDown={(event) => {
                            if (event.key === "Enter") addDomainRule();
                        }}
                        class="bg-card h-11 min-w-44 flex-1 rounded-sm"
                        aria-label={m.specific_website_input_label()}
                        disabled={props.disabled}
                    />
                    <Button
                        onClick={addDomainRule}
                        disabled={
                            !!props.disabled || !domainValidation().domain
                        }
                        class="h-11 shrink-0 gap-2 rounded-sm shadow-none"
                        aria-label={m.add_specific_website()}
                    >
                        <Plus class="size-4" aria-hidden="true" />
                        {m.add_specific_website()}
                    </Button>
                </div>
                <Show
                    when={domainValidation().error}
                    fallback={
                        <p class="text-muted-foreground px-1 text-xs leading-relaxed">
                            {m.website_url_scope_hint()}
                        </p>
                    }
                >
                    <p class="text-destructive px-1 text-xs">
                        {domainValidation().error}
                    </p>
                </Show>
            </div>
            <Show
                when={domainRules().length > 0}
                fallback={
                    <p class="text-muted-foreground text-xs">
                        {m.no_rules_added()}
                    </p>
                }
            >
                <Show when={domainRules().length > 5}>
                    <Input
                        placeholder={m.block_websites_search()}
                        aria-label={m.block_websites_search()}
                        value={websiteSearch()}
                        onInput={(event) => {
                            setWebsiteSearch(event.currentTarget.value);
                            setWebsitePage(0);
                        }}
                        class="bg-card h-10 rounded-sm"
                    />
                </Show>
                <div class="border-border divide-border divide-y border-y">
                    <For each={visibleWebsites()}>
                        {(item) => renderRule(item)}
                    </For>
                </div>
                <Show
                    when={
                        domainRules().length > WEBSITES_PER_PAGE ||
                        !!websiteSearch().trim()
                    }
                >
                    <div class="flex flex-wrap items-center justify-between gap-3">
                        <p class="text-muted-foreground text-xs">
                            {m.block_websites_showing({
                                count: visibleWebsites().length,
                                total: filteredWebsites().length,
                            })}
                        </p>
                        <Show when={websitePages() > 1}>
                            <div class="flex items-center gap-2">
                                <Button
                                    size="sm"
                                    variant="outline"
                                    disabled={displayedWebsitePage() === 0}
                                    onClick={() =>
                                        setWebsitePage(
                                            Math.max(
                                                0,
                                                displayedWebsitePage() - 1,
                                            ),
                                        )
                                    }
                                >
                                    {m.block_websites_prev()}
                                </Button>
                                <span class="text-muted-foreground text-xs">
                                    {m.block_websites_page({
                                        page: displayedWebsitePage() + 1,
                                        pages: websitePages(),
                                    })}
                                </span>
                                <Button
                                    size="sm"
                                    variant="outline"
                                    disabled={
                                        displayedWebsitePage() >=
                                        websitePages() - 1
                                    }
                                    onClick={() =>
                                        setWebsitePage(
                                            displayedWebsitePage() + 1,
                                        )
                                    }
                                >
                                    {m.block_websites_next()}
                                </Button>
                            </div>
                        </Show>
                    </div>
                </Show>
            </Show>
            <div class="border-border border-t pt-3">
                <button
                    type="button"
                    class="text-sm font-medium underline underline-offset-4"
                    aria-expanded={bulkOpen() ? "true" : "false"}
                    onClick={() => setBulkOpen(!bulkOpen())}
                >
                    {m.block_bulk_add_title()}
                </button>
                <Show when={bulkOpen()}>
                    <div class="mt-3 space-y-2">
                        <textarea
                            class="bg-card border-border w-full rounded-sm border p-3 text-sm"
                            rows={4}
                            value={bulkInput()}
                            onInput={(event) =>
                                setBulkInput(event.currentTarget.value)
                            }
                            placeholder={m.block_bulk_add_hint()}
                            disabled={props.disabled}
                            aria-label={m.block_bulk_add_title()}
                        />
                        <p class="text-muted-foreground text-xs">
                            {bulkParsed().websites.length} ·{" "}
                            {bulkParsed().skipped.length}{" "}
                            {m.block_bulk_skipped()}
                        </p>
                        <Button
                            onClick={addBulkRules}
                            disabled={
                                !!props.disabled ||
                                bulkParsed().websites.length === 0
                            }
                        >
                            {m.block_bulk_add_action()}
                        </Button>
                    </div>
                </Show>
            </div>
            <details class="border-border border-t pt-3">
                <summary class="text-muted-foreground hover:text-foreground cursor-pointer select-none text-sm font-medium transition-colors">
                    {m.use_regex()}
                </summary>
                <div class="mt-3 space-y-3">
                    <p class="text-muted-foreground text-xs leading-relaxed">
                        {m.advanced_matching_description()}
                    </p>
                    <div class="flex gap-2">
                        <Input
                            placeholder={m.advanced_mode_placeholder()}
                            value={regexInput()}
                            onInput={(event) =>
                                setRegexInput(event.currentTarget.value)
                            }
                            onKeyDown={(event) => {
                                if (event.key === "Enter") addRegexRule();
                            }}
                            class="bg-card rounded-sm font-mono text-xs"
                            aria-label={m.advanced_matching_input_label()}
                            disabled={props.disabled}
                        />
                        <Button
                            onClick={addRegexRule}
                            size="icon"
                            disabled={!!props.disabled || !regexInput().trim()}
                            class="shrink-0 rounded-sm shadow-none"
                            aria-label={m.add_advanced_match()}
                        >
                            <Plus class="size-4" aria-hidden="true" />
                        </Button>
                    </div>
                    <div class="text-muted-foreground flex items-start gap-1.5 text-xs leading-relaxed">
                        <CircleAlert
                            class="mt-0.5 size-3.5 shrink-0"
                            aria-hidden="true"
                        />
                        <span>{m.advanced_mode_warning()}</span>
                    </div>
                    <Show when={regexRules().length > 0}>
                        <div class="border-border divide-border divide-y border-y">
                            <For each={regexRules()}>
                                {(item) => renderRule(item)}
                            </For>
                        </div>
                    </Show>
                </div>
            </details>
        </div>
    );
}
