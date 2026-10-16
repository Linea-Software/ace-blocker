import type { SitePackSchema } from "$engine-client";
import categoryCatalog from "../data/site-packs.json";

const ACE_CATEGORY_IDS = new Set(
    categoryCatalog.map((category) => category.id),
);

export type BlockingListKind = "category" | "custom-list";

export function getBlockingListKind(
    pack: Pick<SitePackSchema, "id">,
): BlockingListKind {
    return ACE_CATEGORY_IDS.has(pack.id) ? "category" : "custom-list";
}

export function isAceCategory(pack: Pick<SitePackSchema, "id">): boolean {
    return getBlockingListKind(pack) === "category";
}

export function getBlockingListDisplayName(
    pack: Pick<SitePackSchema, "id" | "name">,
): string {
    if (!isAceCategory(pack)) return pack.name;

    let name = pack.name.replace(/\s+[—-]\s+(Free|Pro)$/i, "");
    name = name.replace(/\s+Websites$/i, "");

    if (/^Adult$/i.test(name)) name = "Adult Content";

    return name;
}
