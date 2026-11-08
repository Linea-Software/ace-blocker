/**
 * DEFAULT NETWORK BLOCKS
 *
 * On fresh installations, the engine seeds four disabled Always blocks.
 * Each starter uses an Ace-managed Free category, including the curated
 * Gambling and Streaming starter categories. Pro packs are never preselected.
 * Onboarding explicitly enables selected blocks without changing their modes.
 */
import { setNetworkBlockEnabled } from "$engine-client";
import * as m from "$lib/paraglide/messages.js";

/** IDs of the blocks seeded by the engine migration. Must stay in sync with
 *  `ace-engine/migrations/0021_seed_default_network_blocks/up.sql`. */
export const DEFAULT_BLOCK_IDS = {
    adultContent: "01950000-0000-7000-8000-000000000001",
    socialMedia: "01950000-0000-7000-8000-000000000002",
    gambling: "01950000-0000-7000-8000-000000000003",
    streaming: "01950000-0000-7000-8000-000000000004",
} as const;

/** The blocks we suggest turning on during onboarding, in presentation order. */
export const ONBOARDING_BLOCK_ORDER: string[] = [
    DEFAULT_BLOCK_IDS.adultContent,
    DEFAULT_BLOCK_IDS.socialMedia,
    DEFAULT_BLOCK_IDS.gambling,
    DEFAULT_BLOCK_IDS.streaming,
];

/** Blocks pre-selected in the wizard: the ones nearly every user wants. */
export const RECOMMENDED_BLOCK_IDS: string[] = [DEFAULT_BLOCK_IDS.adultContent];

export function isDefaultBlock(blockId: string): boolean {
    return ONBOARDING_BLOCK_ORDER.includes(blockId);
}

/**
 * Localized description of a seeded block. Uses an explicit switch rather than
 * indexing `m` with a variable (see AGENTS.md — "Safe i18n Patterns").
 */
export function getDefaultBlockDescription(blockId: string): string {
    switch (blockId) {
        case DEFAULT_BLOCK_IDS.adultContent:
            return m.default_block_adult_description();
        case DEFAULT_BLOCK_IDS.socialMedia:
            return m.default_block_social_description();
        case DEFAULT_BLOCK_IDS.gambling:
            return m.default_block_gambling_description();
        case DEFAULT_BLOCK_IDS.streaming:
            return m.default_block_streaming_description();
        default:
            return "";
    }
}

/** Localized name of a seeded block (the engine stores English names). */
export function getDefaultBlockName(blockId: string, fallback: string): string {
    switch (blockId) {
        case DEFAULT_BLOCK_IDS.adultContent:
            return m.default_block_adult_name();
        case DEFAULT_BLOCK_IDS.socialMedia:
            return m.default_block_social_name();
        case DEFAULT_BLOCK_IDS.gambling:
            return m.default_block_gambling_name();
        case DEFAULT_BLOCK_IDS.streaming:
            return m.default_block_streaming_name();
        default:
            return fallback;
    }
}

/** Enables a starter block without altering its activation mode. */
export async function enableStarterBlock(blockId: string): Promise<void> {
    const { error } = await setNetworkBlockEnabled({
        path: { network_block_id: blockId },
        body: { enabled: true },
    });

    if (error) {
        console.error("Failed to enable starter block", blockId, error);
        throw new Error(`Failed to enable network block ${blockId}`);
    }
}
