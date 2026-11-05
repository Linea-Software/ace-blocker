import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

// Vite is started by tauri dev. Linux development needs the callback
// receiver registered in the *user* session before the app opens a browser
// for sign-in. The installed production handler is never overwritten.
if (process.platform === "linux") {
    const repoRoot = resolve(import.meta.dir, "../../..");
    const commands: [string, string[]][] = [
        [
            "cargo",
            ["build", "--locked", "-p", "ace-engine", "--bin", "ace-redirect"],
        ],
        [
            "bash",
            [resolve(repoRoot, "installer/linux/register-dev-handler.sh")],
        ],
    ];

    for (const [command, args] of commands) {
        const result = spawnSync(command, args, {
            cwd: repoRoot,
            stdio: "inherit",
        });
        if (result.error || result.status !== 0) {
            const message =
                result.error?.message ??
                `${command} exited with ${result.status}`;
            throw new Error(
                `Linux development login callback preparation failed: ${message}`,
            );
        }
    }
}
