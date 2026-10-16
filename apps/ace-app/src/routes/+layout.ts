import { engineStore } from "$lib/stores/engine.svelte";

// Tauri doesn't have a Node.js server to do proper SSR
// so we will use adapter-static to prerender the app (SSG)
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const prerender = true;
export const ssr = false;

export const load = async () => {
    // Ensure the engine store is initialized before any page load
    if (!engineStore.isReady) {
        await engineStore.refresh();
    }
    return {};
};
