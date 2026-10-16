/**
 * GUIDED TOUR STEPS
 *
 * Each step spotlights one element of the real UI. `selector` is matched against
 * `data-tour` attributes that live on the components themselves, so a step
 * silently degrades to a centered tooltip if the element is not on screen (for
 * example the sidebar, which is collapsed on narrow windows).
 */
import type { Pathname } from "$app/types";
import * as m from "$lib/paraglide/messages.js";

export type TourPlacement = "top" | "right" | "bottom" | "left";

export type TourStep = {
    /** Stable key, used for `{#each}` and for debugging. */
    id: string;
    /** Route to navigate to before showing the step. */
    route: Pathname;
    /** CSS selector of the element to spotlight. */
    selector: string;
    /** Preferred side of the target to place the tooltip on. */
    placement: TourPlacement;
    /** Message getters — called at render time so the tour follows locale changes. */
    title: () => string;
    body: () => string;
};

export const TOUR_STEPS: TourStep[] = [
    {
        id: "blocked-attempts",
        route: "/",
        selector: '[data-tour="blocked-attempts"]',
        placement: "bottom",
        title: () => m.tour_blocked_attempts_title(),
        body: () => m.tour_blocked_attempts_body(),
    },
    {
        id: "engine-status",
        route: "/",
        selector: '[data-tour="engine-status"]',
        placement: "top",
        title: () => m.tour_engine_status_title(),
        body: () => m.tour_engine_status_body(),
    },
    {
        id: "network-blocks",
        route: "/network-blocks",
        selector: '[data-tour="nav-network-blocks"]',
        placement: "right",
        title: () => m.tour_network_blocks_title(),
        body: () => m.tour_network_blocks_body(),
    },
    {
        id: "create-block",
        route: "/network-blocks",
        selector: '[data-tour="create-block"]',
        placement: "bottom",
        title: () => m.tour_create_block_title(),
        body: () => m.tour_create_block_body(),
    },
    {
        id: "site-packs",
        route: "/site-packs",
        selector: '[data-tour="nav-site-packs"]',
        placement: "right",
        title: () => m.tour_site_packs_title(),
        body: () => m.tour_site_packs_body(),
    },
    {
        id: "resistance-mode",
        route: "/resistance-mode",
        selector: '[data-tour="nav-resistance-mode"]',
        placement: "right",
        title: () => m.tour_resistance_mode_title(),
        body: () => m.tour_resistance_mode_body(),
    },
    {
        id: "getting-started",
        route: "/getting-started",
        selector: '[data-tour="nav-getting-started"]',
        placement: "right",
        title: () => m.tour_getting_started_title(),
        body: () => m.tour_getting_started_body(),
    },
];
