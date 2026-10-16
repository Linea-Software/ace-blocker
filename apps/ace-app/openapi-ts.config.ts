import { defineConfig } from "@hey-api/openapi-ts";

export default defineConfig({
    input: "./.temp/openapi.json",
    output: {
        path: "src/lib/engine-client",
        tsConfigPath: null,
    },
    plugins: ["@hey-api/client-fetch"],
});
