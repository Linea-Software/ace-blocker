import { paraglideVitePlugin } from "@inlang/paraglide-js";
import solid from "@solidjs/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// Static Tauri WebView; all privileged IPC stays inside the Rust backend.
export default defineConfig({
    plugins: [
        paraglideVitePlugin({
            project: "./project.inlang",
            outdir: "./src/lib/paraglide",
            strategy: ["localStorage", "cookie", "baseLocale"],
        }),
        tailwindcss(),
        solid(),
    ],
    publicDir: "static", // preserve SvelteKit static asset paths (logos, favicon)
    resolve: {
        alias: {
            $lib: fileURLToPath(new URL("./src/lib", import.meta.url)),
            "$engine-client": fileURLToPath(new URL("./src/lib/engine-client", import.meta.url)),
        },
    },
    build: { outDir: "build", emptyOutDir: true },
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
        watch: { ignored: ["**/src-tauri/**"] },
    },
});
