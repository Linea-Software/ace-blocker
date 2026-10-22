// Site Pack retrieval is owned by +page.svelte: it has an explicit
// loading/error/retry lifecycle and must not duplicate an invisible route fetch.
export const load = () => ({});
