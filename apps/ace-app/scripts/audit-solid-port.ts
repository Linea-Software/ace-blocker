/**
 * Reject framework migration leftovers before merging the Solid 2 branch.
 *
 * This is deliberately small, dependency-free and independent of the
 * TypeScript compiler: a Vite bundle can succeed while dead Svelte code
 * and obsolete runtime dependencies remain in a repository.
 */
import { readdir, readFile } from "node:fs/promises";
import { extname, join, relative } from "node:path";

const appRoot = new URL("../", import.meta.url);
const sourceRoot = new URL("../src/", import.meta.url);
const oldFrameworkModules = [
    /(?:from\s+|import\s*\()\s*["'](?:svelte(?:\/|["'])|@sveltejs\/|bits-ui(?:\/|["'])|mode-watcher(?:\/|["'])|svelte-sonner(?:\/|["'])|@lucide\/svelte(?:\/|["'])|@tabler\/icons-svelte(?:\/|["'])|lucide-solid(?:\/|["'])|solid-js\/(?:web|store)(?:\/|["']))/,
    /["']\$app\//,
    /["']\$env\//,
    /["'][^"']*\.svelte(?:\.ts)?["']/,
];

async function sourceFiles(dir: URL): Promise<URL[]> {
    const result: URL[] = [];
    for (const entry of await readdir(dir, { withFileTypes: true })) {
        if (entry.name === "paraglide" && dir.href.endsWith("/src/lib/"))
            continue;
        const child = new URL(
            entry.name + (entry.isDirectory() ? "/" : ""),
            dir,
        );
        if (entry.isDirectory()) result.push(...(await sourceFiles(child)));
        else if (entry.isFile()) result.push(child);
    }
    return result;
}

const failures: string[] = [];
for (const file of await sourceFiles(sourceRoot)) {
    const path = fileURL(file);
    const extension = extname(path);
    if (path.endsWith(".svelte") || path.endsWith(".svelte.ts")) {
        failures.push(`Svelte source remains: ${path}`);
        continue;
    }
    if (![".ts", ".tsx", ".js", ".jsx"].includes(extension)) continue;
    const text = await readFile(file, "utf8");
    for (const pattern of oldFrameworkModules) {
        if (pattern.test(text))
            failures.push(`Legacy framework import: ${path} (${pattern})`);
    }
}

const pkg = JSON.parse(
    await readFile(new URL("../package.json", import.meta.url), "utf8"),
) as {
    dependencies: Record<string, string>;
    devDependencies: Record<string, string>;
};
for (const name of [
    "svelte",
    "svelte-check",
    "@sveltejs/kit",
    "@sveltejs/adapter-static",
    "@sveltejs/vite-plugin-svelte",
    "bits-ui",
    "svelte-sonner",
    "mode-watcher",
    "@lucide/svelte",
    "@tabler/icons-svelte",
    "lucide-solid",
]) {
    if (pkg.dependencies[name] || pkg.devDependencies[name]) {
        failures.push(`Obsolete dependency: ${name}`);
    }
}

function fileURL(url: URL): string {
    // URL -> slash-separated app-relative path; works on Windows too.
    return relative(appRoot.pathname, url.pathname).replaceAll("\\", "/");
}

if (failures.length) {
    console.error("Solid 2 migration gate failed:\n" + failures.join("\n"));
    process.exitCode = 1;
} else {
    console.log(
        "Solid 2 migration gate: no legacy Svelte source/imports/dependencies.",
    );
}
