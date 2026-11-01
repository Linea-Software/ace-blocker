import { paraglideMiddleware } from "$lib/paraglide/server";
import type { Handle } from "@sveltejs/kit/hooks";

export const handle: Handle = ({ event, resolve }) => {
    return paraglideMiddleware(
        event.request,
        ({ request: localizedRequest, locale }) => {
            // SvelteKit 3 exposes request as readonly: pass a localized event
            // to resolve without mutating the original RequestEvent.
            return resolve({ ...event, request: localizedRequest }, {
                transformPageChunk: ({ html }) => {
                    return html.replace("%lang%", locale);
                },
            });
        },
    );
};
