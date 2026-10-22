import { describe, expect, it } from "vitest";
import {
    IncompleteSitePackImportError,
    waitForSitePackImport,
} from "./_site-pack-import";
import type { SitePackCreationStatus } from "$engine-client";

async function* statuses(...events: SitePackCreationStatus[]) {
    yield* events;
}

const started = { STARTED: { phase: "DOWNLOADING", progress: 0, site_pack_id: "id" } } as SitePackCreationStatus;
const done = { COMPLETED: { phase: "COMPLETED", progress: 100, site_pack_id: "id" } } as SitePackCreationStatus;
const failed = { FAILED: { error: "bad hosts file" } } as SitePackCreationStatus;

describe("site pack import terminal event handling", () => {
    it("accepts COMPLETED", async () => {
        await expect(waitForSitePackImport(statuses(started, done))).resolves.toBeUndefined();
    });
    it("rejects premature EOF", async () => {
        await expect(waitForSitePackImport(statuses(started))).rejects.toBeInstanceOf(IncompleteSitePackImportError);
    });
    it("propagates FAILED", async () => {
        await expect(waitForSitePackImport(statuses(started, failed))).rejects.toThrow("bad hosts file");
    });
    it("propagates iterator errors", async () => {
        async function* broken(): AsyncGenerator<SitePackCreationStatus> {
            yield started;
            throw new Error("pipe disconnected");
        }
        await expect(waitForSitePackImport(broken())).rejects.toThrow("pipe disconnected");
    });
});
