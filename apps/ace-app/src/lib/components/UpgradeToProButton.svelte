<script lang="ts">
    import ProGatedButton from "$lib/components/ProGatedButton.svelte";
    import * as m from "$lib/paraglide/messages.js";

    let {
        class: className = "",
        text = m.get_pro(),
    }: {
        class?: string;
        text?: string;
    } = $props();
</script>

<ProGatedButton isPro={false} label={text} class={className} />
