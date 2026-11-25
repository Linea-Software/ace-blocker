import "./SitePackRow.css";
import { createMemo, Show, Switch, Match } from "solid-js";
import { Dynamic } from "@solidjs/web";
import type { SitePackSchema } from "$engine-client";
import * as m from "$lib/paraglide/messages.js";
import { getLocale } from "$lib/paraglide/runtime";
import { getBlockingListDisplayName, requiresProForConnection } from "$lib/utils/category-presentation";
import { getPackIcon } from "../solid-pack-icons";
import {
    getSitePackCoverage,
    getSitePackIconSlug,
} from "$lib/utils/site-pack-presentation";
import {
    Check,
    ChevronRight,
    CircleAlert,
    LoaderCircle,
    LockKeyhole,
} from "@lucide/solid";

export default function SitePackRow(props: {
    pack: SitePackSchema;
    subtitle?: string;
    selected?: boolean;
    extendedPack?: SitePackSchema;
    basicPack?: SitePackSchema;
    isPro: boolean;
    status?: "queued" | "downloading" | "completed" | "failed";
    progress?: number;
    phase?: string | null;
    onSelect: (pack: SitePackSchema) => void;
}) {
    const icon = createMemo(() => getPackIcon(getSitePackIconSlug(props.pack)));
    const coverage = createMemo(() => getSitePackCoverage(props.pack));
    const proRequired = createMemo(
        () => requiresProForConnection(props.pack) && !props.isPro,
    );
    const name = () => getBlockingListDisplayName(props.pack);
    const paired = () => !!props.extendedPack;
    const comparison = () => paired() && !props.isPro && !props.pack.requiresPro;
    const extendedCoverage = () => props.extendedPack ? getSitePackCoverage(props.extendedPack).count : null;
    const basicCoverage = () => props.basicPack ? getSitePackCoverage(props.basicPack).count : null;
    const syncing = () =>
        props.status === "queued" || props.status === "downloading";
    const stateLabel = createMemo(() => {
        if (proRequired()) return m.status_pro_required();
        if (props.pack.isLocked) return m.locked();
        if (props.status === "failed") return m.pack_sync_failed_title();
        if (props.status === "completed") return m.pack_sync_completed();
        if (props.status === "queued") return m.pack_sync_queued();
        if (props.status === "downloading") return props.phase ?? m.syncing();
        return null;
    });
    return (
        <button
            type="button"
            class={[
                "site-pack-row",
                {
                    selected: !!props.selected,
                    locked: !!props.pack.isLocked,
                    "pro-required": proRequired(),
                    failed: props.status === "failed",
                    syncing: syncing(),
                },
            ]}
            data-testid="site-pack-row"
            data-sync-status={props.status ?? "idle"}
            aria-pressed={!!props.selected ? "true" : "false"}
            title={props.pack.isLocked ? m.pack_locked_tooltip() : undefined}
            onClick={() => props.onSelect(props.pack)}
        >
            <span
                class="pack-icon"
                data-testid="site-pack-row-icon"
                aria-hidden="true"
            >
                <Dynamic component={icon()} size={18} strokeWidth={1.75} />
            </span>
            <span class="identity">
                <span class="identity-heading">
                    <strong>{name()}</strong>
                    <Show when={!paired() && !proRequired()}>
                        <span class="tier-label">
                            {requiresProForConnection(props.pack)
                                ? m.pro_badge()
                                : m.pack_access_free()}
                        </span>
                    </Show>
                </span>
                <Show when={paired()}>
                    <span class="subtitle">{props.pack.requiresPro ? m.catalog_pro_extended() : m.catalog_free_essential()}</span>
                </Show>
                <Show
                    when={
                        !!props.subtitle &&
                        !props.pack.isRemote
                    }
                >
                    <span class="subtitle">{props.subtitle}</span>
                </Show>
            </span>
            <span class="coverage" data-testid="site-pack-row-coverage">
                <span>{coverage().count !== null ? coverage().count!.toLocaleString(getLocale()) + " " + m.domains_lowercase() : "—"}</span>
                <Show when={comparison()}>
                    <span class="coverage-pro" data-testid="site-pack-tier-comparison"><LockKeyhole size={12} aria-hidden="true" />{m.catalog_count_with_pro({ count: extendedCoverage()?.toLocaleString(getLocale()) ?? "—" })}</span>
                </Show>
                <Show when={paired() && props.pack.requiresPro && props.basicPack}>
                    <span class="coverage-basic">{m.catalog_count_with_free({ count: basicCoverage()?.toLocaleString(getLocale()) ?? "—" })}</span>
                </Show>
            <Show when={stateLabel()}>
                <span class="state">
                    <Show when={proRequired() || props.pack.isLocked}>
                        <LockKeyhole size={14} aria-hidden="true" />
                    </Show>
                    <Show when={!proRequired() && !props.pack.isLocked}>
                        <Switch>
                            <Match when={props.status === "failed"}>
                                <CircleAlert size={14} aria-hidden="true" />
                            </Match>
                            <Match when={props.status === "completed"}>
                                <Check size={14} aria-hidden="true" />
                            </Match>
                            <Match when={syncing()}>
                                <span class="spinner">
                                    <LoaderCircle
                                        size={14}
                                        aria-hidden="true"
                                    />
                                </span>
                            </Match>
                        </Switch>
                    </Show>
                    <span>{stateLabel()}</span>
                    <Show when={props.status === "downloading"}>
                        <span>{Math.round(props.progress ?? 0)}%</span>
                    </Show>
                </span>
            </Show>
            </span>
            <span class="chevron">
                <ChevronRight size={16} aria-hidden="true" />
            </span>
            <Show when={syncing()}>
                <span
                    class="progress-track"
                    role="progressbar"
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={Math.round(props.progress ?? 0)}
                    aria-label={m.pack_sync_progress_aria({
                        name: props.pack.name,
                    })}
                >
                    <span
                        style={{ width: Math.round(props.progress ?? 0) + "%" }}
                    />
                </span>
            </Show>
        </button>
    );
}
