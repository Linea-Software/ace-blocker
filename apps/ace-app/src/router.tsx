import { lazy } from "solid-js";
import { createRouter } from "@solidjs/router";

// Keep the desktop's exact paths. Chunk routes to preserve startup performance.
export const Router = createRouter({
    routes: [
        { path: "/", component: lazy(() => import("./routes/page")) },
        { path: "/network-blocks", component: lazy(() => import("./routes/network-blocks/+page")) },
        { path: "/network-blocks/editor", component: lazy(() => import("./routes/network-blocks/editor/+page")) },
        { path: "/resistance-mode", component: lazy(() => import("./routes/resistance-mode/page")) },
        { path: "/site-packs", component: lazy(() => import("./routes/site-packs/+page")) },
        { path: "/settings", component: lazy(() => import("./routes/settings/page")) },
        { path: "/getting-started", component: lazy(() => import("./routes/getting-started/page")) },
        { path: "/help", component: lazy(() => import("./routes/help/page")) },
        { path: "*404", component: lazy(() => import("./routes/not-found")) },
    ],
});
