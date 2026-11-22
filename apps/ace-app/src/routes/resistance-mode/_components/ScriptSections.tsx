import { For, createMemo } from "solid-js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import * as m from "$lib/paraglide/messages.js";
import { Switch } from "$lib/components/ui/switch/solid";
import { Checkbox } from "$lib/components/ui/checkbox/solid";
import {
    ChevronDown,
    ChevronUp,
    Download,
    Globe,
    TriangleAlert,
    ListFilter,
    PlaySquare,
    Palette,
    Hourglass,
    ShieldQuestionMark,
    LockKeyhole,
} from "@lucide/solid";
import type { ScriptInfo, ResistanceModeSettings } from "$engine-client";
import {
    groupInterventions,
    type InterventionKind,
} from "./intervention-groups";

type Props = {
    scripts: ScriptInfo[];
    settings: ResistanceModeSettings | null;
    engineOnline: boolean;
    isPro: boolean;
    locked: boolean;
    busy?: boolean;
    onToggleScript: (script: ScriptInfo) => void;
    onToggleGroup?: (scripts: ScriptInfo[], enabled: boolean) => void;
};
export default function ScriptSections(props: Props) {
    const state = {
        expandedKind: null as InterventionKind | null,
        expandedSite: null as string | null,
    };
    reactiveFields(state, ["expandedKind", "expandedSite"]);
    const sections = createMemo(() => groupInterventions(props.scripts));
    const selected = () => props.scripts.filter((s) => s.isEnabled).length;
    const missing = () =>
        props.settings?.enforcementStatus === "scripts_missing";
    const invalid = () =>
        props.settings?.enforcementStatus === "scripts_invalid";
    const editable = () =>
        props.engineOnline && props.isPro && !(props.busy ?? false);
    function title(kind: InterventionKind): string {
        switch (kind) {
            case "feeds":
                return m.interventions_feed();
            case "video":
                return m.interventions_video();
            case "visual":
                return m.interventions_visual();
            case "interruptions":
                return m.interventions_interruptions();
            case "captcha":
                return m.interventions_captcha();
        }
    }
    function description(kind: InterventionKind): string {
        switch (kind) {
            case "feeds":
                return m.interventions_feed_desc();
            case "video":
                return m.interventions_video_desc();
            case "visual":
                return m.interventions_visual_desc();
            case "interruptions":
                return m.interventions_interruptions_desc();
            case "captcha":
                return m.interventions_captcha_desc();
        }
    }

    return (
        <section
            class="space-y-4"
            aria-labelledby="interventions-title"
            data-testid="rm-interventions"
        >
            <div class="flex items-start justify-between gap-4">
                <div>
                    <h2 id="interventions-title" class="font-semibold text-xl">
                        {m.interventions_title()}
                    </h2>
                    <p class="mt-1 max-w-2xl text-sm text-muted-foreground">
                        {m.interventions_description()}
                    </p>
                </div>
                {props.scripts.length ? (
                    <>
                        <span
                            class="shrink-0 font-mono text-xs text-muted-foreground"
                            data-testid="rm-selected()-count"
                        >
                            {m.interventions_selected_count({
                                count: selected(),
                            })}
                        </span>
                    </>
                ) : null}
            </div>

            {props.settings && !props.settings.enabled && props.isPro ? (
                <>
                    <p
                        class="rounded border border-border bg-muted/40 px-3 py-2 text-sm text-muted-foreground"
                        role="status"
                    >
                        {m.interventions_off_note()}
                    </p>
                </>
            ) : null}
            {props.locked ? (
                <>
                    <p
                        class="flex items-center gap-2 rounded border border-border px-3 py-2 text-sm text-muted-foreground"
                        role="status"
                    >
                        <LockKeyhole size={16} aria-hidden="true" />{" "}
                        {m.interventions_lock_note()}
                    </p>
                </>
            ) : null}

            {!props.engineOnline ? (
                <>
                    <p class="p-5 text-sm text-muted-foreground">
                        {m.error_engine_offline_action()}
                    </p>
                </>
            ) : missing() || invalid() ? (
                <>
                    <div
                        class="flex flex-wrap items-center gap-3 rounded border border-border p-4"
                        data-testid={
                            missing()
                                ? "rm-props.scripts-empty"
                                : "rm-props.scripts-invalid()"
                        }
                    >
                        {missing() ? (
                            <>
                                <Download size={21} aria-hidden="true" />
                            </>
                        ) : (
                            <>
                                <TriangleAlert size={21} aria-hidden="true" />
                            </>
                        )}
                        <div class="min-w-48 flex-1">
                            <strong>
                                {missing()
                                    ? m.scripts_empty_title()
                                    : m.scripts_invalid_title()}
                            </strong>
                            <p class="text-sm text-muted-foreground">
                                {missing()
                                    ? m.scripts_empty_description()
                                    : m.scripts_invalid_description()}
                            </p>
                        </div>
                        <a
                            href="/props.settings#resistance-props.scripts-update"
                            class="text-sm underline underline-offset-4 hover:text-foreground"
                        >
                            {m.interventions_manage_scripts_in_settings()}
                        </a>
                    </div>
                </>
            ) : props.scripts.length === 0 ? (
                <>
                    <p class="p-5 text-sm text-muted-foreground">
                        {m.error_scripts_unavailable()}
                    </p>
                </>
            ) : (
                <>
                    <div class="grid gap-2" data-testid="rm-category-list">
                        <For each={sections()}>
                            {(section) => (
                                <>
                                    <div
                                        class="overflow-hidden rounded-md border border-border bg-card"
                                        data-testid={
                                            "rm-category-" + section.kind
                                        }
                                    >
                                        <div class="flex items-stretch">
                                            <button
                                                type="button"
                                                data-testid={
                                                    "rm-category-disclosure-" +
                                                    section.kind
                                                }
                                                class="flex min-h-24 min-w-0 flex-1 items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-muted/40 focus-visible:outline-2 focus-visible:outline-offset-[-3px]"
                                                aria-expanded={
                                                    state.expandedKind ===
                                                    section.kind
                                                        ? "true"
                                                        : "false"
                                                }
                                                aria-label={m.interventions_expand_category(
                                                    {
                                                        category: title(
                                                            section.kind,
                                                        ),
                                                    },
                                                )}
                                                onClick={() => {
                                                    state.expandedKind =
                                                        state.expandedKind ===
                                                        section.kind
                                                            ? null
                                                            : section.kind;
                                                    state.expandedSite = null;
                                                }}
                                            >
                                                <span class="flex size-10 shrink-0 items-center justify-center rounded bg-muted text-foreground">
                                                    {section.kind ===
                                                    "feeds" ? (
                                                        <>
                                                            <ListFilter
                                                                size={21}
                                                                aria-hidden="true"
                                                            />
                                                        </>
                                                    ) : section.kind ===
                                                      "video" ? (
                                                        <>
                                                            <PlaySquare
                                                                size={21}
                                                                aria-hidden="true"
                                                            />
                                                        </>
                                                    ) : section.kind ===
                                                      "visual" ? (
                                                        <>
                                                            <Palette
                                                                size={21}
                                                                aria-hidden="true"
                                                            />
                                                        </>
                                                    ) : section.kind ===
                                                      "interruptions" ? (
                                                        <>
                                                            <Hourglass
                                                                size={21}
                                                                aria-hidden="true"
                                                            />
                                                        </>
                                                    ) : (
                                                        <>
                                                            <ShieldQuestionMark
                                                                size={21}
                                                                aria-hidden="true"
                                                            />
                                                        </>
                                                    )}
                                                </span>
                                                <span class="min-w-0 flex-1">
                                                    <strong class="block text-base font-semibold">
                                                        {title(section.kind)}
                                                    </strong>
                                                    <span class="mt-1 block text-sm leading-snug text-muted-foreground">
                                                        {description(
                                                            section.kind,
                                                        )}
                                                    </span>
                                                </span>
                                                <span class="shrink-0 font-mono text-xs text-muted-foreground">
                                                    {section.selectedCount}/
                                                    {section.totalCount}
                                                </span>
                                                {state.expandedKind ===
                                                section.kind ? (
                                                    <>
                                                        <ChevronUp
                                                            size={17}
                                                            aria-hidden="true"
                                                        />
                                                    </>
                                                ) : (
                                                    <>
                                                        <ChevronDown
                                                            size={17}
                                                            aria-hidden="true"
                                                        />
                                                    </>
                                                )}
                                            </button>
                                            {props.onToggleGroup ? (
                                                <>
                                                    <div class="flex shrink-0 items-center border-l border-border/70">
                                                        <label
                                                            for={
                                                                "rm-select-all-" +
                                                                section.kind
                                                            }
                                                            class="flex min-h-16 w-14 cursor-pointer items-center justify-center has-[button:disabled]:cursor-not-allowed sm:w-16"
                                                            title={m.interventions_select_all_category(
                                                                {
                                                                    category:
                                                                        title(
                                                                            section.kind,
                                                                        ),
                                                                },
                                                            )}
                                                        >
                                                            <Checkbox
                                                                id={
                                                                    "rm-select-all-" +
                                                                    section.kind
                                                                }
                                                                data-testid={
                                                                    "rm-select-all-" +
                                                                    section.kind
                                                                }
                                                                class="size-5"
                                                                checked={
                                                                    section.totalCount >
                                                                        0 &&
                                                                    section.selectedCount ===
                                                                        section.totalCount
                                                                }
                                                                indeterminate={
                                                                    section.selectedCount >
                                                                        0 &&
                                                                    !(
                                                                        section.totalCount >
                                                                            0 &&
                                                                        section.selectedCount ===
                                                                            section.totalCount
                                                                    )
                                                                }
                                                                disabled={
                                                                    !editable() ||
                                                                    section.totalCount ===
                                                                        0 ||
                                                                    (props.locked &&
                                                                        section.totalCount >
                                                                            0 &&
                                                                        section.selectedCount ===
                                                                            section.totalCount)
                                                                }
                                                                aria-label={m.interventions_select_all_category(
                                                                    {
                                                                        category:
                                                                            title(
                                                                                section.kind,
                                                                            ),
                                                                    },
                                                                )}
                                                                onCheckedChange={(
                                                                    checked,
                                                                ) => {
                                                                    // Mixed means select remaining. Time Lock forbids
                                                                    // disabling enabled interventions from the header.
                                                                    const enable =
                                                                        (section.selectedCount >
                                                                            0 &&
                                                                            !(
                                                                                section.totalCount >
                                                                                    0 &&
                                                                                section.selectedCount ===
                                                                                    section.totalCount
                                                                            )) ||
                                                                        checked;
                                                                    if (
                                                                        props.locked &&
                                                                        !enable
                                                                    )
                                                                        return;
                                                                    props.onToggleGroup?.(
                                                                        section.sites.flatMap(
                                                                            (
                                                                                site,
                                                                            ) =>
                                                                                site.scripts,
                                                                        ),
                                                                        enable,
                                                                    );
                                                                }}
                                                            />
                                                        </label>
                                                    </div>
                                                </>
                                            ) : null}
                                        </div>

                                        {state.expandedKind === section.kind ? (
                                            <>
                                                <div class="border-t border-border px-4 pb-2">
                                                    {section.totalCount ===
                                                    0 ? (
                                                        <>
                                                            <p class="py-4 text-sm text-muted-foreground">
                                                                {m.interventions_empty_category()}
                                                            </p>
                                                        </>
                                                    ) : (
                                                        <>
                                                            <div class="flex min-h-11 items-center border-b border-border py-2 text-xs text-muted-foreground">
                                                                <span>
                                                                    {m.interventions_sites_count(
                                                                        {
                                                                            count: section
                                                                                .sites
                                                                                .length,
                                                                        },
                                                                    )}
                                                                </span>
                                                            </div>
                                                            <For
                                                                each={
                                                                    section.sites
                                                                }
                                                            >
                                                                {(site) => (
                                                                    <>
                                                                        <div
                                                                            class="border-b border-border/60 last:border-b-0"
                                                                            data-testid="rm-site-row"
                                                                        >
                                                                            <button
                                                                                type="button"
                                                                                class="flex min-h-12 w-full items-center gap-3 text-left hover:text-foreground focus-visible:outline-2"
                                                                                aria-expanded={
                                                                                    state.expandedSite ===
                                                                                    section.kind +
                                                                                        ":" +
                                                                                        site.domain
                                                                                        ? "true"
                                                                                        : "false"
                                                                                }
                                                                                aria-label={m.interventions_expand_site(
                                                                                    {
                                                                                        domain: site.domain,
                                                                                    },
                                                                                )}
                                                                                onClick={() =>
                                                                                    (state.expandedSite =
                                                                                        state.expandedSite ===
                                                                                        section.kind +
                                                                                            ":" +
                                                                                            site.domain
                                                                                            ? null
                                                                                            : section.kind +
                                                                                              ":" +
                                                                                              site.domain)
                                                                                }
                                                                            >
                                                                                <span class="flex size-6 shrink-0 items-center justify-center">
                                                                                    {site.icon ? (
                                                                                        <>
                                                                                            <span
                                                                                                class="inline-flex size-5 items-center justify-center [&_svg]:size-5 [&_svg]:fill-current"
                                                                                                data-site-brand-icon={
                                                                                                    site
                                                                                                        .icon
                                                                                                        .hex
                                                                                                }
                                                                                                style={{
                                                                                                    color:
                                                                                                        site.icon.hex.toUpperCase() ===
                                                                                                        "000000"
                                                                                                            ? "var(--foreground)"
                                                                                                            : "#" +
                                                                                                              site
                                                                                                                  .icon
                                                                                                                  .hex,
                                                                                                    "--ace-brand-color":
                                                                                                        site.icon.hex.toUpperCase() ===
                                                                                                        "000000"
                                                                                                            ? "var(--foreground)"
                                                                                                            : "#" +
                                                                                                              site
                                                                                                                  .icon
                                                                                                                  .hex,
                                                                                                }}
                                                                                                aria-hidden="true"
                                                                                            >
                                                                                                <span
                                                                                                    innerHTML={
                                                                                                        site
                                                                                                            .icon
                                                                                                            .svg
                                                                                                    }
                                                                                                />
                                                                                            </span>
                                                                                        </>
                                                                                    ) : (
                                                                                        <>
                                                                                            <Globe
                                                                                                size={
                                                                                                    19
                                                                                                }
                                                                                                aria-hidden="true"
                                                                                            />
                                                                                        </>
                                                                                    )}
                                                                                </span>
                                                                                <span class="min-w-0 flex-1 truncate text-sm font-medium">
                                                                                    {
                                                                                        site.domain
                                                                                    }
                                                                                </span>
                                                                                <span class="font-mono text-xs text-muted-foreground">
                                                                                    {
                                                                                        site.selectedCount
                                                                                    }

                                                                                    /
                                                                                    {
                                                                                        site.totalCount
                                                                                    }
                                                                                </span>
                                                                                {state.expandedSite ===
                                                                                section.kind +
                                                                                    ":" +
                                                                                    site.domain ? (
                                                                                    <>
                                                                                        <ChevronUp
                                                                                            size={
                                                                                                15
                                                                                            }
                                                                                            aria-hidden="true"
                                                                                        />
                                                                                    </>
                                                                                ) : (
                                                                                    <>
                                                                                        <ChevronDown
                                                                                            size={
                                                                                                15
                                                                                            }
                                                                                            aria-hidden="true"
                                                                                        />
                                                                                    </>
                                                                                )}
                                                                            </button>
                                                                            {state.expandedSite ===
                                                                            section.kind +
                                                                                ":" +
                                                                                site.domain ? (
                                                                                <>
                                                                                    <div class="ml-8 border-l border-border pl-4 pb-3">
                                                                                        <For
                                                                                            each={
                                                                                                site.scripts
                                                                                            }
                                                                                        >
                                                                                            {(
                                                                                                script,
                                                                                            ) => (
                                                                                                <>
                                                                                                    <div class="flex min-h-14 items-center gap-3 border-b border-border/60 py-3 last:border-b-0">
                                                                                                        <div class="min-w-0 flex-1">
                                                                                                            <strong class="block text-sm font-medium">
                                                                                                                {
                                                                                                                    script.name
                                                                                                                }
                                                                                                            </strong>
                                                                                                            {script.description ? (
                                                                                                                <>
                                                                                                                    <p class="mt-1 text-xs leading-relaxed text-muted-foreground">
                                                                                                                        {
                                                                                                                            script.description
                                                                                                                        }
                                                                                                                    </p>
                                                                                                                </>
                                                                                                            ) : null}
                                                                                                        </div>
                                                                                                        <Switch
                                                                                                            data-testid="rm-script-switch"
                                                                                                            checked={
                                                                                                                script.isEnabled
                                                                                                            }
                                                                                                            disabled={
                                                                                                                !editable() ||
                                                                                                                (props.locked &&
                                                                                                                    script.isEnabled)
                                                                                                            }
                                                                                                            onCheckedChange={() =>
                                                                                                                props.onToggleScript(
                                                                                                                    script,
                                                                                                                )
                                                                                                            }
                                                                                                            aria-label={
                                                                                                                script.name
                                                                                                            }
                                                                                                        />
                                                                                                    </div>
                                                                                                </>
                                                                                            )}
                                                                                        </For>
                                                                                    </div>
                                                                                </>
                                                                            ) : null}
                                                                        </div>
                                                                    </>
                                                                )}
                                                            </For>
                                                        </>
                                                    )}
                                                </div>
                                            </>
                                        ) : null}
                                    </div>
                                </>
                            )}
                        </For>
                    </div>
                </>
            )}

            {props.scripts.length && !missing() && !invalid() ? (
                <>
                    <p
                        class="pt-2 text-xs text-muted-foreground"
                        data-testid="rm-operational-details"
                    >
                        {props.settings?.scriptsVersion
                            ? m.last_scripts_update({
                                  version: props.settings.scriptsVersion,
                              })
                            : m.scripts_never_updated()}
                    </p>
                </>
            ) : null}
        </section>
    );
}
