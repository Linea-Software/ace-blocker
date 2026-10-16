import { getAllSitePacks } from "$engine-client";
import type { PageLoad } from "./$types";

export const load: PageLoad = async () => {
    const response = await getAllSitePacks();
    return {
        sitePacks: response.data?.sitePacks ?? [],
    };
};
