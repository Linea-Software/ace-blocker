import { fileURLToPath } from "node:url";
import type { Options } from "@wdio/types";

// Target only the debug Tauri binary; its WebDriver plugins are not
// registered in release builds. This runner never targets production.
const appBinary = fileURLToPath(
    new URL("../../target/debug/ace-app", import.meta.url),
) + (process.platform === "win32" ? ".exe" : "");

export const config: Options.Testrunner = {
    runner: "local",
    specs: ["./e2e/native/**/*.spec.ts"],
    maxInstances: 1,
    services: [
        [
            "tauri",
            {
                appBinaryPath: appBinary,
                driverProvider: "embedded",
                captureFrontendLogs: true,
            },
        ],
    ],
    capabilities: [
        {
            browserName: "tauri",
            "tauri:options": { application: appBinary },
        },
    ],
    logLevel: "warn",
    framework: "mocha",
    reporters: ["spec"],
    waitforTimeout: 15000,
    connectionRetryTimeout: 90000,
    connectionRetryCount: 1,
    mochaOpts: { ui: "bdd", timeout: 60000 },
};
