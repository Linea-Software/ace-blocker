import { createEffect, createMemo, createSignal, Show } from "solid-js";
import { useLocation, useNavigate } from "@solidjs/router";
import * as m from "$lib/paraglide/messages.js";
import BlockEditor from "../_BlockEditor";
import LockModal from "../_components/_LockModal";
import { loadBlockEditor, type BlockEditorLoad } from "./_data";

export default function BlockEditorPage() {
    const navigate = useNavigate();
    const location = useLocation();
    const requestedId = createMemo(() => new URLSearchParams(location.search).get("block"));
    const section = createMemo(() => new URLSearchParams(location.search).get("section"));
    const [data, setData] = createSignal<BlockEditorLoad>({
        block: null, requestedId: requestedId(), engineOffline: false, loadError: false,
    });
    const [loading, setLoading] = createSignal(true);
    const [lockOpen, setLockOpen] = createSignal(false);
    const [editorStep, setEditorStep] = createSignal<0 | 1 | 2>(
        section() === "schedule" ? 1 : 0,
    );
    const returnToList = () => navigate("/network-blocks");

    // Solid Router keeps a matched component mounted when only search params
    // change. Every request carries its identity, and stale RPC responses must
    // never overwrite the draft for the currently selected block.
    let requestGeneration = 0;
    async function refresh(id: string | null = requestedId()) {
        const generation = ++requestGeneration;
        setLoading(true);
        try {
            const loaded = await loadBlockEditor(id);
            if (generation !== requestGeneration) return;
            setData(loaded);
        } catch (error) {
            if (generation !== requestGeneration) return;
            console.error("Failed to load network block editor", error);
            setData({ block: null, requestedId: id, engineOffline: false, loadError: true });
        } finally {
            if (generation === requestGeneration) setLoading(false);
        }
    }

    createEffect(
        () => ({ id: requestedId(), section: section() }),
        ({ id, section: selectedSection }) => {
            setLockOpen(false);
            setEditorStep(selectedSection === "schedule" ? 1 : 0);
            void refresh(id);
        },
    );

    return (
        <>
            <Show when={!loading() && data().requestedId === requestedId()} fallback={<div role="status" class="editor-error">{m.loading_block_configuration()}</div>}>
                <Show when={!data().engineOffline && !data().loadError && (!requestedId() || data().block)}
                    fallback={
                        <main class="editor-error" role="alert" data-requested-block={requestedId() ?? "new"}>
                            <h1>{m.network_blocks_load_failed_title()}</h1>
                            <p>{m.network_blocks_load_failed_description()}</p>
                            <a href="/network-blocks">{m.cancel()}</a>
                        </main>
                    }>
                    <BlockEditor block={data().block} step={editorStep()} onStepChange={setEditorStep}
                        onClose={returnToList}
                        onProtect={() => { if (data().block) setLockOpen(true); }}
                        onSave={() => { void returnToList(); }} />
                </Show>
            </Show>
            <LockModal open={lockOpen()} onOpenChange={setLockOpen}
                block={data().block} onSave={refresh}
                onEditSchedule={() => {
                    setLockOpen(false);
                    setEditorStep(1);
                }} />
            <style>{`
                .editor-error { max-width: 660px; margin: 50px auto; padding: 26px; border: 1px solid var(--border); background: var(--card); }
                .editor-error h1 { font: 600 26px "IBM Plex Sans Condensed", sans-serif; }
                .editor-error p { margin: 12px 0; color: var(--muted-foreground); }
                .editor-error a { text-decoration: underline; text-underline-offset: 3px; }
            `}</style>
        </>
    );
}
