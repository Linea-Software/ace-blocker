import type { SitePackSchema } from "$engine-client";
import categoryCatalog from "../data/site-packs.json";

const ACE_CATEGORY_IDS = new Set(
    categoryCatalog.map((category) => category.id),
);

export type BlockingListKind = "category" | "custom-list";

/** Remote packs are curated by Ace, even if their local ID is a UUID.
 * User-imported packs have no remote origin. Bundled canonical IDs remain
 * categories while older or offline engine snapshots are being loaded. */
type PackOrigin = Pick<SitePackSchema, "id"> & { isRemote?: boolean };

export function getBlockingListKind(pack: PackOrigin): BlockingListKind {
    return pack.isRemote || ACE_CATEGORY_IDS.has(pack.id)
        ? "category"
        : "custom-list";
}

export function isAceCategory(pack: PackOrigin): boolean {
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
