/**
 * Audit English Paraglide messages against the application source.
 *
 * Count both m.key() calls and m.key function references (for example,
 * callbacks supplied to the navigation rail). The generated Paraglide
 * directory is intentionally excluded.
 *
 * Usage: bun run i18n:audit
 */
import { readFileSync, readdirSync } from "node:fs";
import { extname, join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const english = JSON.parse(
    readFileSync(join(root, "messages/en.json"), "utf8"),
) as Record<string, string>;

const referenced = new Set<string>();
const dynamicAccesses: string[] = [];
let scannedFiles = 0;

function scanDirectory(directory: string): void {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
        const path = join(directory, entry.name);

        if (entry.isDirectory()) {
            if (entry.name !== "paraglide") scanDirectory(path);
            continue;
        }

        if (![".svelte", ".ts", ".tsx", ".js"].includes(extname(path))) {
            continue;
        }

        const contents = readFileSync(path, "utf8");
        const imports = [
            ...contents.matchAll(
                /\bimport\s+\*\s+as\s+([A-Za-z_$][\w$]*)\s+from\s+["'][^"']*\/paraglide\/messages(?:\.js)?["']/g,
            ),
        ];
        if (imports.length === 0) continue;

        scannedFiles++;

        for (const [, alias] of imports) {
            const escapedAlias = alias.replace(/[.*+?^\u0024{}()|[\]\\]/g, "\\$&");
            const properties = new RegExp(
                `\\b${escapedAlias}\\.([A-Za-z_$][\\w$]*)\\b`,
                "g",
            );
            const indexing = new RegExp(`\\b${escapedAlias}\\s*\\[`, "g");

            // Indexing with a runtime key cannot be checked statically.
            if (indexing.test(contents)) {
                dynamicAccesses.push(path);
            }

            for (const [, key] of contents.matchAll(properties)) {
                // A local variable named m is also used for minutes in
                // _SoftUnlockModal; its toString() is not a message key.
                if (key !== "toString") referenced.add(key);
            }
        }
    }
}

scanDirectory(join(root, "src"));

const keys = Object.keys(english);
const unused = keys.filter((key) => !referenced.has(key));
const missing = [...referenced].filter(
    (key) => !Object.hasOwn(english, key),
).sort();

console.log(
    `Paraglide EN: ${keys.length} keys, ${referenced.size} referenced, ${scannedFiles} source files scanned.`,
);
if (missing.length) console.error("Missing keys:", missing.join(", "));
if (unused.length) console.error("Unused keys:", unused.join(", "));
if (dynamicAccesses.length) {
    console.error("Dynamic message access in:", dynamicAccesses.join(", "));
}

if (missing.length || unused.length || dynamicAccesses.length) {
    process.exitCode = 1;
} else {
    console.log("All Paraglide keys are referenced and valid.");
}
