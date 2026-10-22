import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { rpcParameters } from "./rpc-parameters";

/**
 * This test intentionally reads the owning Rust bridge rather than another
 * manually maintained list of operations. It detects new/renamed operations
 * and omitted Tauri JSON fields before release.
 */
const bridgeSource = readFileSync(
    resolve(process.cwd(), "../../crates/ace-engine/src/service/api/bridge.rs"),
    "utf8",
);
const matches = [
    ...bridgeSource.matchAll(/^\s{8}"([A-Za-z][A-Za-z0-9]*)"\s*=>/gm),
];
const operations = new Map<string, Set<string>>();
for (const [index, match] of matches.entries()) {
    const op = match[1];
    const next = matches[index + 1];
    // The Rust arms are the only source of argument-field requirements.
    const arm = bridgeSource.slice(match.index, next?.index ?? bridgeSource.length);
    const fields = new Set(
        [...arm.matchAll(/args\.get\("([^"]+)"\)/g)].map((field) => field[1]),
    );
    const existing = operations.get(op) ?? new Set<string>();
    fields.forEach((field) => existing.add(field));
    operations.set(op, existing);
}

describe("Rust/Tauri/TypeScript operation contract", () => {
    it("lists exactly the Rust bridge's operation names", () => {
        expect([...operations.keys()].sort()).toEqual(
            Object.keys(rpcParameters).sort(),
        );
    });

    it("maps all required Rust JSON argument keys for every operation", () => {
        for (const [name, rustFields] of operations) {
            expect(
                new Set(rpcParameters[name as keyof typeof rpcParameters]),
                name,
            ).toEqual(rustFields);
        }
    });
});
