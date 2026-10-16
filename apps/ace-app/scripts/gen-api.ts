import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

// The committed engine spec is the source of truth for client generation.
// Fetching from a live engine is an explicit opt-in (--live), for verifying
// engine changes before committing the updated spec.
const COMMITTED_SPEC = path.resolve(
    import.meta.dir,
    "../../../crates/ace-engine/docs/openapi.json",
);
const LIVE_API_URL = "https://127.0.0.1:10567/api-docs/openapi.json";
const OUTPUT_FILE = path.resolve(import.meta.dir, "../.temp/openapi.json");

function copyCommittedSpec() {
    console.log(`Reading committed API spec from ${COMMITTED_SPEC}...`);
    if (!fs.existsSync(COMMITTED_SPEC)) {
        console.warn(
            `Warning: committed spec not found at ${COMMITTED_SPEC}. ` +
                `Run the engine's gen-openapi-spec bin (cargo run --bin gen-openapi-spec) ` +
                `or use --live to fetch from a running engine.`,
        );
        return false;
    }
    fs.mkdirSync(path.dirname(OUTPUT_FILE), { recursive: true });
    fs.copyFileSync(COMMITTED_SPEC, OUTPUT_FILE);
    console.log(`API spec copied to ${OUTPUT_FILE}`);
    return true;
}

async function fetchLiveSpec() {
    console.log(`Fetching API spec from ${LIVE_API_URL}...`);
    try {
        // Disable TLS validation for self-signed certificates
        process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";
        const response = await fetch(LIVE_API_URL);

        if (!response.ok) {
            throw new Error(
                `Failed to fetch API spec: ${response.status} ${response.statusText}`,
            );
        }
        const data = await response.text();

        // Ensure docs directory exists
        const docsDir = path.dirname(OUTPUT_FILE);
        if (!fs.existsSync(docsDir)) {
            fs.mkdirSync(docsDir, { recursive: true });
        }

        fs.writeFileSync(OUTPUT_FILE, data);
        console.log(`API spec saved to ${OUTPUT_FILE}`);
        return true;
    } catch (error) {
        console.warn(
            `Warning: Could not fetch API spec from live engine. Using existing file if available.`,
        );
        console.warn(
            `Error details: ${error instanceof Error ? error.message : String(error)}`,
        );
        return false;
    }
}

async function main() {
    const useLive = process.argv.includes("--live");

    const specReady = useLive
        ? await fetchLiveSpec()
        : copyCommittedSpec();

    if (!specReady && !fs.existsSync(OUTPUT_FILE)) {
        console.error(
            "No API spec available. Commit the engine's docs/openapi.json or run with --live against a running engine.",
        );
        process.exit(1);
    }

    console.log("Generating API client...");
    try {
        execSync("bun x openapi-ts", {
            stdio: "inherit",
            cwd: path.resolve(import.meta.dir, ".."),
        });
        console.log("API client generated successfully.");
    } catch (error) {
        console.error(
            "Failed to generate API client:",
            error instanceof Error ? error.message : String(error),
        );
        process.exit(1);
    }
}

main();
