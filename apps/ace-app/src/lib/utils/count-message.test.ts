import { describe, expect, it, vi } from "vitest";
import { countMessage } from "./count-message";

describe("localized count messages", () => {
    it.each([
        [0, "none"],
        [1, "singular"],
        [2, "plural"],
        [17, "plural"],
    ] as const)("chooses the correct form for %i", (count, expected) => {
        const zero = vi.fn(() => "none");
        const one = vi.fn(() => "singular");
        const other = vi.fn(() => "plural");
        expect(countMessage(count, zero, one, other)).toBe(expected);
        expect(
            zero.mock.calls.length +
                one.mock.calls.length +
                other.mock.calls.length,
        ).toBe(1);
    });
});

import * as m from "$lib/paraglide/messages.js";

it("renders accurate Italian and English network-block counts", () => {
    for (const locale of ["it", "en"] as const) {
        const expected =
            locale === "it"
                ? [
                      "Nessun blocco di rete sta applicando le proprie regole.",
                      "1 blocco di rete sta applicando le proprie regole.",
                      "2 blocchi di rete stanno applicando le proprie regole.",
                  ]
                : [
                      "No Network Blocks are currently applying their rules.",
                      "1 Network Block is currently applying its rules.",
                      "2 Network Blocks are currently applying their rules.",
                  ];
        for (const count of [0, 1, 2]) {
            expect(
                countMessage(
                    count,
                    () =>
                        m.control_protection_active_detail_zero({}, { locale }),
                    () =>
                        m.control_protection_active_detail_one({}, { locale }),
                    () =>
                        m.control_protection_active_detail(
                            { count },
                            { locale },
                        ),
                ),
            ).toBe(expected[count]);
        }
    }
});

it("uses grammatical Italian completion summaries for 0, 1 and many", () => {
    const expected = [
        "Nessun blocco abilitato.",
        "1 blocco abilitato.",
        "3 blocchi abilitati.",
    ];
    for (const [index, count] of [0, 1, 3].entries()) {
        expect(
            countMessage(
                count,
                () => m.onboarding_done_summary_zero({}, { locale: "it" }),
                () => m.onboarding_done_summary_one({}, { locale: "it" }),
                () => m.onboarding_done_summary({ count }, { locale: "it" }),
            ),
        ).toBe(expected[index]);
    }
});
