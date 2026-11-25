import type { SitePackSchema } from "$engine-client";
import catalogJson from "$lib/data/site-packs.json";
import {
    getBlockingListDisplayName,
    isAceCategory,
} from "./category-presentation";

/**
 * UI catalog snapshot, not a live server count. An installed pack's
 * entriesCount always takes precedence for its own coverage.
 * Names are used only to match Ace-managed remote packs whose local IDs are UUIDs.
 * Never infer a category from a user's Custom List name.
 */
type CatalogEntry = {
    id: string;
    name: string;
    icon: string;
    requires_pro: boolean;
    domains_count: number;
};
const catalog: CatalogEntry[] = catalogJson;
const catalogById = new Map(catalog.map((pack) => [pack.id, pack]));
const displayNameOf = (pack: Pick<SitePackSchema, "id" | "name">) =>
    getBlockingListDisplayName(pack).toLocaleLowerCase("en-US");
const catalogByNameAndTier = new Map(
    catalog.map((pack) => [
        `${displayNameOf(pack)}:${pack.requires_pro ? "pro" : "free"}`,
        pack,
    ]),
);

type PackPreview = Pick<
    SitePackSchema,
    | "id"
    | "name"
    | "isRemote"
    | "requiresPro"
    | "icon"
    | "isSynced"
    | "entriesCount"
>;

export function getOfficialCatalogEntry(
    pack: Pick<SitePackSchema, "id" | "name" | "isRemote" | "requiresPro">,
): CatalogEntry | undefined {
    if (!isAceCategory(pack)) return undefined;
    return (
        catalogById.get(pack.id) ??
        catalogByNameAndTier.get(
            `${displayNameOf(pack)}:${pack.requiresPro ? "pro" : "free"}`,
        )
    );
}

export function getSitePackIconSlug(
    pack: Pick<
        SitePackSchema,
        "id" | "name" | "isRemote" | "requiresPro" | "icon"
    >,
): string {
    // User-imported icons are never replaced with an Ace category icon.
    return pack.icon || getOfficialCatalogEntry(pack)?.icon || "globe";
}

export type SitePackCoverage = {
    count: number | null;
    source: "installed" | "catalog" | "unknown";
};

export function getSitePackCoverage(pack: PackPreview): SitePackCoverage {
    if (pack.isSynced) {
        return { count: pack.entriesCount, source: "installed" };
    }
    const count = getOfficialCatalogEntry(pack)?.domains_count;
    return count !== undefined
        ? { count, source: "catalog" }
        : { count: null, source: "unknown" };
}

export type SitePackCategoryGroup = {
    name: string;
    packs: SitePackSchema[];
};

export function groupSitePackCategories(
    packs: SitePackSchema[],
): SitePackCategoryGroup[] {
    const groups = new Map<string, SitePackCategoryGroup>();
    for (const pack of packs) {
        if (!isAceCategory(pack)) continue;
        const name = getBlockingListDisplayName(pack);
        const key = name.toLocaleLowerCase("en-US");
        let group = groups.get(key);
        if (!group) {
            group = { name, packs: [] };
            groups.set(key, group);
        }
        group.packs.push(pack);
    }
    return [...groups.values()]
        .map((group) => ({
            ...group,
            packs: group.packs.sort(
                (a, b) =>
                    Number(a.requiresPro) - Number(b.requiresPro) ||
                    a.name.localeCompare(b.name),
            ),
        }))
        .sort((a, b) =>
            Number(a.packs.every((pack) => pack.requiresPro)) -
                Number(b.packs.every((pack) => pack.requiresPro)) ||
            a.name.localeCompare(b.name),
        );
}


/** Choose the entitlement's edition, or the sole locked Pro preview. */
export function preferredCategoryPack(group: SitePackCategoryGroup, isPro: boolean): SitePackSchema {
    return (isPro ? group.packs.find((pack) => pack.requiresPro) : group.packs.find((pack) => !pack.requiresPro)) ?? group.packs[0]!;
}

export function extendedCategoryPack(group: SitePackCategoryGroup): SitePackSchema | undefined {
    return group.packs.some((pack) => !pack.requiresPro)
        ? group.packs.find((pack) => pack.requiresPro)
        : undefined;
}
