import { describe, expect, it } from "vitest";
import { formattedLockEnd } from "./lock-format";

describe("localized Resistance lock deadlines", () => {
    it("preserves the instant and formats it in Spanish", () => {
        const instant = "2026-12-11T18:30:00+02:00";
        expect(formattedLockEnd(instant, "es")).toBe(
            new Date(instant).toLocaleString("es"),
        );
        expect(formattedLockEnd(instant, "it")).toBe(
            new Date(instant).toLocaleString("it"),
        );
    });

    it("does not invent a date for malformed input", () => {
        expect(formattedLockEnd("invalid", "es")).toBe("invalid");
    });
});
