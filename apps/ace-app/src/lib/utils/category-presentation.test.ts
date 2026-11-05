import { describe, expect, it } from "vitest";
import {
    getBlockingListDisplayName,
    getBlockingListKind,
    isAceCategory,
} from "./category-presentation";

describe("category presentation", () => {
    it("identifies Ace catalog entries as categories", () => {
        const item = {
            id: "pack-social-free",
            name: "Social Media Websites — Free",
        };

        expect(isAceCategory(item)).toBe(true);
        expect(getBlockingListKind(item)).toBe("category");
        expect(getBlockingListDisplayName(item)).toBe("Social Media");
    });

    it("treats server-managed Pro packs with generated local IDs as categories", () => {
        const official = {
            id: "019bd28a-9417-7aef-a30e-44a95a7e7e2c",
            name: "Gaming Websites — Pro",
            isRemote: true,
            requiresPro: true,
        };
        expect(getBlockingListKind(official)).toBe("category");
        expect(getBlockingListDisplayName(official)).toBe("Gaming");
        expect(isAceCategory(official)).toBe(true);
    });

    it("keeps user-created lists out of official categories even when Pro-only", () => {
        const custom = {
            id: "019bd28a-9417-7aef-a30e-44a95a7e7e2d",
            name: "My Pro Collection",
            isRemote: false,
            requiresPro: true,
        };
        expect(getBlockingListKind(custom)).toBe("custom-list");
        expect(getBlockingListDisplayName(custom)).toBe(custom.name);
    });

    it("normalizes the adult category to product language", () => {
        expect(
            getBlockingListDisplayName({
                id: "pack-adult-pro",
                name: "Adult Websites — Pro",
            }),
        ).toBe("Adult Content");
    });

    it("treats non-catalog pack ids as Custom Lists without renaming them", () => {
        const item = { id: "user-import-123", name: "My Focus List" };

        expect(isAceCategory(item)).toBe(false);
        expect(getBlockingListKind(item)).toBe("custom-list");
        expect(getBlockingListDisplayName(item)).toBe("My Focus List");
    });
});
