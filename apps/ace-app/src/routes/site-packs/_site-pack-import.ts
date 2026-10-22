import type { SitePackCreationStatus } from "$engine-client";

/** A successful Tauri stream invocation does not imply a completed import. */
export class IncompleteSitePackImportError extends Error {
    constructor() {
        super("Site pack import stream ended without a terminal event");
        this.name = "IncompleteSitePackImportError";
    }
}

export async function waitForSitePackImport(
    stream: AsyncIterable<SitePackCreationStatus>,
): Promise<void> {
    for await (const event of stream) {
        if ("FAILED" in event) throw new Error(event.FAILED.error);
        if ("COMPLETED" in event) return;
    }
    throw new IncompleteSitePackImportError();
}
