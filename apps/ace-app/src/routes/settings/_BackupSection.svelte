<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { getLocale } from "$lib/paraglide/runtime";
    import { userStore } from "$lib/stores/user-store.svelte";
    import {
        exportBackup,
        importBackup,
        importBackupFromCloud,
        exportBackupToCloud,
    } from "$lib/engine-client";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import {
        Database,
        CloudUpload,
        RefreshCw,
        FileUp,
        FileDown,
        Save,
        History,
        ShieldAlert,
        Cloud,
    } from "@lucide/svelte";
    import { toast } from "svelte-sonner";
    import UpgradeToProButton from "$lib/components/UpgradeToProButton.svelte";
    import { save, open } from "@tauri-apps/plugin-dialog";
    import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";

    let isExporting = $state(false);
    let isImporting = $state(false);
    let isCloudSyncing = $state(false);
    let isCloudRestoring = $state(false);
    let isImportConfirmOpen = $state(false);
    let lastSyncedDate = $state<string | null>(null);
    let isCloudPasswordDialogOpen = $state(false);
    let cloudOperation = $state<"upload" | "restore" | null>(null);
    let cloudPassword = $state("");
    let cloudPasswordError = $state<string | null>(null);

    async function handleExportLocal() {
        isExporting = true;
        const response = await exportBackup();

        if (response.error || !response.data) {
            console.error(response.error);
            toast.error(m.backup_export_failed());
            isExporting = false;
            return;
        }

        try {
            const path = await save({
                filters: [{ name: "Ace Backup", extensions: ["json"] }],
                defaultPath: `ace-backup-${new Date().toISOString().split("T")[0]}.json`,
            });

            if (path) {
                await writeTextFile(
                    path,
                    JSON.stringify(response.data, null, 2),
                );
                toast.success(m.backup_export_success());
            }
        } catch (error) {
            console.error(error);
            toast.error(m.backup_export_failed());
        } finally {
            isExporting = false;
        }
    }

    async function handleImportLocal() {
        isImporting = true;
        try {
            const path = await open({
                multiple: false,
                filters: [{ name: "Ace Backup", extensions: ["json"] }],
            });

            if (path && typeof path === "string") {
                const content = await readTextFile(path);
                const data = JSON.parse(content);
                const { error } = await importBackup({ body: data });

                if (error) {
                    console.error(error);
                    toast.error(m.backup_import_failed());
                } else {
                    toast.success(m.backup_import_success());
                }
            }
        } catch (error) {
            console.error(error);
            toast.error(m.backup_import_failed());
        } finally {
            isImporting = false;
            isImportConfirmOpen = false;
        }
    }

    function openCloudPasswordDialog(operation: "upload" | "restore") {
        if (!userStore.isPro || isCloudSyncing || isCloudRestoring) return;
        cloudPassword = "";
        cloudPasswordError = null;
        cloudOperation = operation;
        isCloudPasswordDialogOpen = true;
    }

    function closeCloudPasswordDialog() {
        if (isCloudSyncing || isCloudRestoring) return;
        cloudPassword = "";
        cloudPasswordError = null;
        cloudOperation = null;
        isCloudPasswordDialogOpen = false;
    }

    async function handleConfirmCloud(event: SubmitEvent) {
        event.preventDefault();
        if (
            !userStore.isPro ||
            !cloudOperation ||
            !cloudPassword ||
            isCloudSyncing ||
            isCloudRestoring
        ) return;

        const operation = cloudOperation;
        cloudPasswordError = null;
        if (operation === "upload") {
            isCloudSyncing = true;
        } else {
            isCloudRestoring = true;
        }

        try {
            // Capture the password in the RPC argument, then immediately wipe the UI.
            // No password is kept for retry or shared with another operation.
            const request = operation === "upload"
                ? exportBackupToCloud({ body: { password: cloudPassword } })
                : importBackupFromCloud({ body: { password: cloudPassword } });
            cloudPassword = "";
            const { error } = await request;
            if (error) {
                cloudPasswordError = cloudErrorMessage(
                    error,
                    operation === "upload"
                        ? m.cloud_backup_failed()
                        : m.cloud_restore_failed(),
                );
            } else {
                if (operation === "upload") {
                    toast.success(m.cloud_backup_success());
                    lastSyncedDate = new Date().toLocaleString(getLocale());
                } else {
                    toast.success(m.cloud_restore_success());
                }
                cloudPassword = "";
                cloudPasswordError = null;
                cloudOperation = null;
                isCloudPasswordDialogOpen = false;
            }
        } catch {
            // Do not log credential-bearing RPC error objects.
            cloudPasswordError =
                operation === "upload"
                    ? m.cloud_backup_failed()
                    : m.cloud_restore_failed();
        } finally {
            cloudPassword = "";
            isCloudSyncing = false;
            isCloudRestoring = false;
        }
    }

    function cloudErrorMessage(error: unknown, fallbackMessage: string): string {
        const code =
            error && typeof error === "object"
                ? (error as Record<string, unknown>).code
                : undefined;

        // Only match known server messages; never display or log raw RPC errors.
        const reason =
            error && typeof error === "object"
                ? (error as Record<string, unknown>).error
                : undefined;
        if (reason === "Incorrect password or corrupted cloud backup") {
            return m.cloud_backup_decryption_failed();
        }
        if (code === "unauthorized" || code === "capability_required") {
            return m.cloud_backup_not_signed_in();
        }
        if (code === "forbidden" || code === "pro_plan_required") {
            return m.cloud_backup_needs_pro();
        }
        if (
            code === "invalid_password" ||
            code === "incorrect_password" ||
            code === "wrong_password" ||
            code === "invalid_credentials"
        ) {
            return m.cloud_backup_password_incorrect();
        }
        if (code === "decryption_failed" || code === "backup_decryption_failed") {
            return m.cloud_backup_decryption_failed();
        }
        return fallbackMessage;
    }

</script>

<section class="settings-section">
    <header>
        <span class="section-mark" aria-hidden="true"><Database size={16} /></span>
        <div>
            <h2>{m.backup_and_restore()}</h2>
            <p>{m.backup_and_restore_description()}</p>
        </div>
    </header>

    <div class="backup-ledger">
        <div class="backup-row">
            <span class="row-mark" aria-hidden="true"><Save size={17} /></span>
            <div class="row-copy">
                <h3>{m.local_backup()}</h3>
                <p>{m.local_backup_description()}</p>
            </div>
            <div class="row-actions">
                <button type="button" onclick={handleExportLocal} disabled={isExporting}>
                    {#if isExporting}
                        <RefreshCw class="spinner" size={15} aria-hidden="true" />
                    {:else}
                        <FileUp size={15} aria-hidden="true" />
                    {/if}
                    {m.export()}
                </button>
                <button
                    type="button"
                    onclick={() => (isImportConfirmOpen = true)}
                    disabled={isImporting}
                >
                    {#if isImporting}
                        <RefreshCw class="spinner" size={15} aria-hidden="true" />
                    {:else}
                        <FileDown size={15} aria-hidden="true" />
                    {/if}
                    {m.import()}
                </button>
            </div>
        </div>

        <div class:restricted={!userStore.isPro} class="backup-row">
            <span class="row-mark" aria-hidden="true"><Cloud size={17} /></span>
            <div class="row-copy">
                <h3>{m.cloud_backup()}</h3>
                <p>
                    {userStore.isPro
                        ? m.cloud_backup_description()
                        : m.cloud_backup_tagline()}
                </p>
                {#if userStore.isPro && lastSyncedDate}
                    <span class="last-sync">
                        {m.last_backed_up({ date: lastSyncedDate })}
                    </span>
                {/if}
            </div>
            <div class="row-actions">
                {#if userStore.isPro}
                    <button
                        type="button"
                        onclick={() => openCloudPasswordDialog("upload")}
                        disabled={isCloudSyncing || isCloudRestoring}
                    >
                        {#if isCloudSyncing}
                            <RefreshCw class="spinner" size={15} aria-hidden="true" />
                        {:else}
                            <CloudUpload size={15} aria-hidden="true" />
                        {/if}
                        {m.upload_to_cloud()}
                    </button>
                    <button
                        type="button"
                        onclick={() => openCloudPasswordDialog("restore")}
                        disabled={isCloudSyncing || isCloudRestoring}
                    >
                        {#if isCloudRestoring}
                            <RefreshCw class="spinner" size={15} aria-hidden="true" />
                        {:else}
                            <History size={15} aria-hidden="true" />
                        {/if}
                        {m.restore_from_cloud()}
                    </button>
                {:else}
                    <UpgradeToProButton text={m.cloud_backup()} />
                {/if}
            </div>
        </div>
    </div>
</section>

<AlertDialog.Root bind:open={isImportConfirmOpen}>
    <AlertDialog.Content class="rounded-md border-border bg-card shadow-none sm:max-w-md">
        <AlertDialog.Header class="gap-4">
            <div class="import-warning">
                <ShieldAlert size={18} aria-hidden="true" />
            </div>
            <div class="flex flex-col gap-2 text-left">
                <AlertDialog.Title class="font-condensed text-xl font-semibold">
                    {m.import_backup_confirmation_title()}
                </AlertDialog.Title>
                <AlertDialog.Description class="text-sm leading-relaxed">
                    {m.import_backup_confirmation_description()}
                </AlertDialog.Description>
            </div>
        </AlertDialog.Header>
        <AlertDialog.Footer class="mt-2 gap-2 border-t border-border pt-4">
            <AlertDialog.Cancel disabled={isImporting}>
                {m.cancel()}
            </AlertDialog.Cancel>
            <AlertDialog.Action
                onclick={handleImportLocal}
                disabled={isImporting}
                class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
                {#if isImporting}
                    <RefreshCw class="spinner" size={15} aria-hidden="true" />
                    {m.processing()}
                {:else}
                    <FileDown size={15} aria-hidden="true" />
                    {m.confirm()}
                {/if}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

<AlertDialog.Root
    bind:open={isCloudPasswordDialogOpen}
    onOpenChange={(open) => {
        if (!open) {
            cloudPassword = "";
            cloudPasswordError = null;
            cloudOperation = null;
        }
    }}
>
    <AlertDialog.Content
        class="rounded-md border-border bg-card shadow-none sm:max-w-md"
        onEscapeKeydown={(event) => {
            if (isCloudSyncing || isCloudRestoring) event.preventDefault();
        }}
        onOpenAutoFocus={(event) => {
            event.preventDefault();
            document.getElementById("cloud-backup-password")?.focus();
        }}
    >
        <form autocomplete="off" onsubmit={handleConfirmCloud} class="grid gap-4">
            <AlertDialog.Header class="gap-3">
                <div class="flex flex-col gap-2 text-left">
                    <AlertDialog.Title class="font-condensed text-xl font-semibold">
                        {cloudOperation === "upload"
                            ? m.cloud_backup_password_upload_title()
                            : m.cloud_backup_password_restore_title()}
                    </AlertDialog.Title>
                    <AlertDialog.Description class="text-sm leading-relaxed">
                        {cloudOperation === "upload"
                            ? m.cloud_backup_password_upload_description()
                            : m.cloud_backup_password_restore_description()}
                    </AlertDialog.Description>
                </div>
            </AlertDialog.Header>
            <div class="cloud-password-field">
                <label for="cloud-backup-password">{m.cloud_backup_password_label()}</label>
                <input
                    id="cloud-backup-password"
                    type="password"
                    autocomplete="off"
                    autocapitalize="off"
                    spellcheck={false}
                    required
                    bind:value={cloudPassword}
                    disabled={isCloudSyncing || isCloudRestoring}
                    aria-invalid={cloudPasswordError !== null}
                    aria-describedby={cloudPasswordError
                        ? "cloud-backup-password-hint cloud-backup-password-error"
                        : "cloud-backup-password-hint"}
                />
                <p id="cloud-backup-password-hint">{m.cloud_backup_password_hint()}</p>
                {#if cloudPasswordError}
                    <p id="cloud-backup-password-error" class="cloud-password-error" role="alert">
                        {cloudPasswordError}
                    </p>
                {/if}
            </div>
            <AlertDialog.Footer class="mt-1 gap-2 border-t border-border pt-4">
                <AlertDialog.Cancel
                    type="button"
                    disabled={isCloudSyncing || isCloudRestoring}
                    onclick={closeCloudPasswordDialog}
                >
                    {m.cancel()}
                </AlertDialog.Cancel>
                <button
                    class="cloud-password-submit"
                    type="submit"
                    disabled={!cloudPassword || isCloudSyncing || isCloudRestoring}
                >
                    {#if isCloudSyncing || isCloudRestoring}
                        <RefreshCw class="spinner" size={15} aria-hidden="true" />
                        {m.processing()}
                    {:else if cloudOperation === "upload"}
                        <CloudUpload size={15} aria-hidden="true" />
                        {m.upload_to_cloud()}
                    {:else}
                        <History size={15} aria-hidden="true" />
                        {m.restore_from_cloud()}
                    {/if}
                </button>
            </AlertDialog.Footer>
        </form>
    </AlertDialog.Content>
</AlertDialog.Root>

<style>
    .settings-section {
        display: grid;
        gap: 12px;
    }

    header {
        display: grid;
        grid-template-columns: 28px minmax(0, 1fr);
        gap: 8px;
        align-items: start;
    }

    .section-mark {
        width: 24px;
        height: 24px;
        display: grid;
        place-items: center;
        border-left: 3px solid var(--ace-ash);
        color: var(--muted-foreground);
    }

    h2,
    h3 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-weight: 600;
    }

    h2 {
        font-size: 18px;
    }

    header p {
        margin: 3px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    .backup-ledger {
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--card);
    }

    .backup-row {
        min-height: 88px;
        display: grid;
        grid-template-columns: 36px minmax(0, 1fr) auto;
        align-items: center;
        gap: 13px;
        padding: 14px;
    }

    .backup-row + .backup-row {
        border-top: 1px solid var(--border);
    }

    .backup-row.restricted {
        background: color-mix(in srgb, var(--muted) 22%, var(--card));
    }

    .row-mark {
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--muted-foreground);
    }

    .row-copy {
        min-width: 0;
    }

    .row-copy h3 {
        font-size: 15px;
    }

    .row-copy p {
        max-width: 520px;
        margin: 4px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.45;
    }

    .last-sync {
        display: block;
        margin-top: 5px;
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 12px;
    }

    .row-actions {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 7px;
    }

    .row-actions button {
        min-height: 36px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 7px;
        border: 1px solid var(--border);
        border-radius: 4px;
        padding: 0 11px;
        background: transparent;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        cursor: pointer;
    }

    :global(.dark) .row-actions button {
        border-color: var(--input);
    }

    .row-actions button:hover:not(:disabled) {
        background: var(--muted);
    }

    .row-actions button:disabled {
        cursor: not-allowed;
        opacity: 0.45;
    }

    .import-warning {
        width: 36px;
        height: 36px;
        display: grid;
        place-items: center;
        border: 1px solid color-mix(in srgb, var(--ace-danger) 35%, var(--border));
        border-radius: 4px;
        color: var(--ace-danger);
    }

    .cloud-password-field {
        display: grid;
        gap: 7px;
    }

    .cloud-password-field label {
        font-size: 13px;
        font-weight: 600;
    }

    .cloud-password-field input {
        width: 100%;
        min-height: 38px;
        border: 1px solid var(--input);
        border-radius: 4px;
        padding: 8px 11px;
        background: var(--background);
        color: var(--foreground);
        font-size: 14px;
    }

    .cloud-password-field input:focus-visible {
        outline: 2px solid var(--ring);
        outline-offset: 2px;
    }

    .cloud-password-field input[aria-invalid="true"] {
        border-color: var(--ace-danger);
    }

    .cloud-password-field p {
        margin: 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.45;
    }

    .cloud-password-field .cloud-password-error {
        color: var(--ace-danger);
    }

    .cloud-password-submit {
        display: inline-flex;
        min-height: 36px;
        align-items: center;
        justify-content: center;
        gap: 7px;
        border: 1px solid var(--border);
        border-radius: 4px;
        padding: 0 12px;
        background: var(--foreground);
        color: var(--background);
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
    }

    .cloud-password-submit:focus-visible {
        outline: 2px solid var(--ring);
        outline-offset: 2px;
    }

    .cloud-password-submit:disabled {
        cursor: not-allowed;
        opacity: 0.5;
    }

    :global(.spinner) {
        animation: backup-spin 900ms linear infinite;
    }

    @keyframes backup-spin {
        to {
            transform: rotate(360deg);
        }
    }

    @container page (max-width: 600px) {
        .backup-row {
            grid-template-columns: 36px minmax(0, 1fr);
        }

        .row-actions {
            grid-column: 2;
            justify-content: flex-start;
            flex-wrap: wrap;
        }
    }

    @media (prefers-reduced-motion: reduce) {
        :global(.spinner) {
            animation: none;
        }
    }
</style>
