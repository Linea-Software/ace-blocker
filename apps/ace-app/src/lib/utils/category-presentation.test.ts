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
