// Data transfer types for the typed Tauri/remoc engine client.

export type AccountInfoResponse = {
    isPro: boolean;
    tosAccepted: boolean;
};

export type AddNetListToSitePackRequest = {
    sitesListId: string;
};

export type AddNetListToSitePackResponse = {
    connection: NetworkBlockToSitePackConnection;
};

export type AddScheduleEntriesRequest = {
    scheduleEntries: Array<BaseScheduleEntrySchema>;
};

export type AddSitePackSourceRequest = {
    format: SitePackFormat;
    source: string;
    sourceType: SitePackSourceType;
};

export type AddSitePackSourceResponse = {
    source: SitePackSourceSchema;
};

export type AddUserDefinedRulesRequest = {
    rules: Array<UserDefinedRule>;
};

export type AddUserDefinedRulesResponse = {
    userDefinedRules: Array<UserDefinedRuleSchema>;
};

/**
 * Application configuration constants
 */
export type ApiConstants = {
    /**
     * The configured datetime format string
     */
    datetime_format: '%Y-%m-%dT%H:%M:%S';
    /**
     * The configured time format string
     */
    time_format: '%H:%M:%S';
};

export type ScheduleMode = 'always' | 'scheduled';

export type BackupNetworkBlock = {
    connectedSitePacks: Array<string>;
    enabled?: boolean;
    icon?: string | null;
    id: string;
    name: string;
    safeSearch: boolean;
    /** Policy's persisted IANA timezone (not the current Windows timezone). */
    scheduleTimezone: string;
    scheduleMode: ScheduleMode;
    schedules: Array<BackupScheduleEntry>;
    userRules: Array<UserDefinedRule>;
};

export type BackupScheduleEntry = {
    endTime: string;
    friday: boolean;
    monday: boolean;
    saturday: boolean;
    startTime: string;
    sunday: boolean;
    thursday: boolean;
    tuesday: boolean;
    wednesday: boolean;
};

export type BackupSitePack = {
    description?: string | null;
    icon?: string | null;
    /**
     * The pack's local row id. For built-in/default and updater-registered
     * remote packs this can be the canonical catalog id (`pack-adult-free`,
     * ...) — never assume a UUID — or a UUIDv7 for engine-created rows.
     */
    id: string;
    name: string;
    remoteId?: string | null;
    requiresPro?: boolean;
    sources: Array<BackupSitePackSource>;
    version?: string | null;
};

export type BackupSitePackSource = {
    format: string;
    source: string;
    sourceType: string;
};

/**
 * Represents the data of a ScheduleEntry without the ID
 */
export type BaseScheduleEntrySchema = {
    endTime: string;
    friday: boolean;
    monday: boolean;
    saturday: boolean;
    startTime: string;
    sunday: boolean;
    thursday: boolean;
    tuesday: boolean;
    wednesday: boolean;
};

export type BaseSitePackSourceSchema = {
    format: SitePackFormat;
    source: string;
    sourceType: SitePackSourceType;
};

export type BinaryUpdateInfo = {
    available: boolean;
    currentVersion: string;
    latestVersion?: string | null;
    status: "checking" | "disabled" | "upToDate" | "available" | "downloading" | "verifying" | "downloaded" | "error";
};

export type ContentUpdateInfo = {
    resistanceScriptsNeedUpdate: boolean;
    sitePacksNeedingUpdate: Array<SitePackUpdateEntry>;
};

export type CreateNetworkBlockRequest = {
    blockName: string;
    icon?: string | null;
};

export type CreateNetworkBlockResponse = {
    networkBlock: NetworkBlockSchema;
};

/** The complete editable state of a block, saved atomically by Ace Engine. */
export type SaveNetworkBlockRequest = {
    id: string | null;
    name: string;
    icon: string | null;
    safeSearch: boolean;
    rules: Array<UserDefinedRule>;
    scheduleEntries: Array<BaseScheduleEntrySchema>;
    scheduleMode: ScheduleMode;
    sitePackIds: Array<string>;
};

export type SaveNetworkBlockResponse = {
    networkBlock: NetworkBlockSchema;
};

export type CreateSitePackRequest = {
    customFolder?: string | null;
    description?: string | null;
    icon?: string | null;
    name: string;
    sources: Array<BaseSitePackSourceSchema>;
};

/**
 * Single-call response that returns all data the dashboard needs.
 */
export type DashboardSnapshotResponse = {
    /**
     * Today's aggregate blocked-attempts count.
     */
    blockedAttempts: number;
    blocks: Array<NetworkBlockSnapshot>;
    resistanceModeSettings?: null | ResistanceModeSettings;
};

export type DebugProPolicy = 'Pro' | 'Free' | 'Default';

export type DebugResistanceLevel = 'None' | 'Low' | 'Medium' | 'High' | 'Extreme' | 'Default';

/**
 * Machine-readable enforcement status for Resistance Mode.
 *
 * The engine never reports `operational` merely because the master setting
 * is enabled: scripts must be loaded, eligible for injection, and a
 * non-`none` intensity must be configured. Priority order when multiple
 * conditions fail: `disabled` > `not_pro` > `scripts_missing` >
 * `scripts_invalid` > `no_scripts_enabled` > `intensity_none`.
 */
export type EnforcementStatus = 'operational' | 'disabled' | 'not_pro' | 'scripts_missing' | 'scripts_invalid' | 'no_scripts_enabled' | 'intensity_none';

export type EngineBackup = {
    customSitePacks: Array<BackupSitePack>;
    deactivatedResistanceScripts: Array<string>;
    networkBlocks: Array<BackupNetworkBlock>;
    version: string;
};

export type GetAllNetworkBlocksResponse = {
    networkBlocks: Array<NetworkBlockSchema>;
};

export type GetAllScriptsResponse = {
    scripts: Array<ScriptInfo>;
};

export type GetAllSitePacksResponse = {
    sitePacks: Array<SitePackSchema>;
};

export type GetNetListToSitePacksResponse = {
    connections: Array<NetworkBlockToSitePackConnection>;
};

export type GetNetworkLockResponse = {
    lock: NetworkListLockSchema;
};

export type GetScheduleEntriesResponse = {
    scheduleEntries: Array<ScheduleEntrySchema>;
};

export type GetSitePackSourcesResponse = {
    sources: Array<SitePackSourceSchema>;
};

export type GetUserDefinedRulesResponse = {
    userDefinedRules: Array<UserDefinedRuleSchema>;
};

export type LockState = {
    end?: string | null;
    state: string;
};

export type NetworkBlockSchema = {
    enabled: boolean;
    icon?: string | null;
    id: string;
    isActive: boolean;
    isLocked: boolean;
    lockState: LockState;
    name: string;
    safeSearch: boolean;
    scheduleMode: ScheduleMode;
};

/**
 * A flattened snapshot of a single network block with all nested data included.
 */
export type NetworkBlockSnapshot = {
    block: NetworkBlockSchema;
    lock?: null | NetworkListLockSchema;
    scheduleEntries: Array<ScheduleEntrySchema>;
    sitePackConnectionCount: number;
    userRulesCount: number;
};

export type NetworkBlockToSitePackConnection = {
    networkBlockId: string;
    sitePackId: string;
};

export type NetworkListLockSchema = {
    isActive: boolean;
    lockEnd?: string | null;
    lockState: LockState;
    networkBlockId: string;
};

export type Percentage = number;

/**
 * Represents the resistance intensity levels.
 */
export type PunishmentIntensity = 'none' | 'low' | 'medium' | 'high' | 'extreme';

/**
 * The intensity scale is a static, user-configured level. There is no
 * automatic escalation/de-escalation ladder: the dynamic mode (and the whole
 * tracking pipeline that fed it) has been removed (MGR-24).
 */
export type PunishmentMode = {
    intensity?: PunishmentIntensity;
    type: 'static';
};

export type RenameNetworkBlockRequest = {
    newName: string;
};

export type ResistanceModeSettings = {
    /**
     * Currently effective intensity (static intensity)
     */
    currentIntensity: PunishmentIntensity;
    enabled: boolean;
    /**
     * Whether enforcement is operational right now, plus a stable reason.
     */
    enforcementStatus: EnforcementStatus;
    isLocked: boolean;
    lockEnd?: string | null;
    lockState: LockState;
    /**
     * Current mode (static only in v1 beta)
     */
    mode: PunishmentMode;
    /**
     * Whether the user is currently eligible under the Pro state (cached).
     */
    proEligible: boolean;
    /**
     * Number of loaded scripts that are enabled.
     */
    scriptsEnabled: number;
    /**
     * True when a valid script catalog/tree is loaded in memory.
     */
    scriptsLoaded: boolean;
    /**
     * Total number of loaded scripts (all domains, including generic).
     */
    scriptsTotal: number;
    /**
     * Installed scripts version (from the last successful download).
     */
    scriptsVersion?: string | null;
    /**
     * Static mode intensity
     */
    staticIntensity: PunishmentIntensity;
};

export type ResistanceModeTypeResponse = {
    currentIntensity: PunishmentIntensity;
    mode: PunishmentMode;
};

export type ScheduleEntrySchema = {
    endTime: string;
    friday: boolean;
    id: string;
    monday: boolean;
    networkBlockId: string;
    saturday: boolean;
    startTime: string;
    sunday: boolean;
    thursday: boolean;
    tuesday: boolean;
    wednesday: boolean;
};

export type ScriptInfo = {
    description: string;
    domain: string;
    id: string;
    isEnabled: boolean;
    name: string;
};

export type SetApiKeyRequest = {
    apiKey: string;
};

export type SetEnabledRequest = {
    enabled: boolean;
};

export type SetIconRequest = {
    icon?: string | null;
};

export type SetNetworkLockRequest = {
    lockEnd?: string | null;
    lockState: string;
};

export type SetNetworkLockResponse = {
    lock: NetworkListLockSchema;
};

export type SitePackCreationStatus = {
    STARTED: {
        phase: SitePackImportPhase;
        progress: Percentage;
        site_pack_id: string;
    };
} | {
    IN_PROGRESS: {
        phase: SitePackImportPhase;
        progress: Percentage;
        site_pack_id: string;
    };
} | {
    COMPLETED: {
        phase: SitePackImportPhase;
        progress: Percentage;
        site_pack_id: string;
    };
} | {
    FAILED: {
        error: string;
        phase?: null | SitePackImportPhase;
    };
};

export type SitePackFormat = 'standard' | 'wildcarddomains' | 'auto';

/**
 * Represents the current phase of a site pack import/resync operation
 */
export type SitePackImportPhase = 'INITIALIZING' | 'DOWNLOADING' | 'EXTRACTING' | 'IMPORTING' | 'RESTORING' | 'FINALIZING' | 'COMPLETED';

export type SitePackSchema = {
    description: string;
    entriesCount: number;
    icon?: string | null;
    id: string;
    /**
     * True when the local data was installed from the installer's offline
     * bundle (a `.bundled` marker next to `pack.db`). Cleared automatically
     * whenever the engine swaps the pack directory with downloaded data.
     */
    isBundled: boolean;
    /**
     * Lock-state exposure (canonical invariant): true when the pack is
     * synchronized AND at least one currently-locked network block
     * references it. Locked packs must not be deleted or resynced; the
     * engine enforces this server-side regardless of the UI state.
     */
    isLocked: boolean;
    isSynced: boolean;
    /** True for site packs managed by Ace's remote release catalogue. */
    isRemote: boolean;
    lastUpdatedAt?: string | null;
    name: string;
    requiresPro: boolean;
    /**
     * Content version of the local pack data (`site_packs.version`): the
     * server release version when downloaded/updated, or the bundle content
     * version (`YYYY.MM.DD`) when installed from the installer.
     */
    version?: string | null;
};

export type SitePackSourceSchema = {
    format: SitePackFormat;
    id: string;
    sitePackId: string;
    source: string;
    sourceType: SitePackSourceType;
};

export type SitePackSourceType = 'url' | 'file';

export type SitePackUpdateEntry = {
    currentVersion: string;
    id: string;
    latestVersion: string;
    name: string;
};

/**
 * Body for the 425 Too Early response — includes how many seconds to wait.
 */
export type TooEarlyResponse = {
    error: string;
    retryAfterSeconds: number;
};

export type UnlockChallengeConfirmRequest = {
    challengeId: string;
};

export type UnlockChallengeConfirmResponse = {
    networkBlock: NetworkBlockSchema;
};

export type UnlockChallengeInitiateResponse = {
    challengeId: string;
    cooldownSeconds: number;
    initiatedAt: string;
    minWords: number;
    reflectionPrompt: string;
};

export type UpdateCheckIntervalResponse = {
    intervalSeconds: number;
};

export type UpdateCheckResponse = {
    content: ContentUpdateInfo;
    engine: BinaryUpdateInfo;
};

export type UserDefinedRule = {
    rule: string;
    ruleType: UserDefinedRuleType;
};

export type UserDefinedRuleSchema = {
    id: string;
    networkBlockId: string;
    rule: string;
    ruleType: UserDefinedRuleType;
};

export type UserDefinedRuleType = 'domain' | 'regex';

export type VersionInfo = {
    /**
     * Build profile: "debug" or "release"
     */
    build: string;
    /**
     * Semantic version of the engine (from Cargo.toml)
     */
    version: string;
};

export type AcceptTosData = {
    body?: never;
    path?: never;
    query?: never;
};

export type AcceptTosResponses = {
    /**
     * Terms of Service accepted
     */
    200: unknown;
};

export type SetApiKeyData = {
    body: SetApiKeyRequest;
    path?: never;
    query?: never;
};

export type SetApiKeyResponses = {
    /**
     * Set API key and fetch account details
     */
    200: unknown;
};

export type GetAccountInfoData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetAccountInfoResponses = {
    /**
     * Get current account information
     */
    200: AccountInfoResponse;
};

export type GetAccountInfoResponse = GetAccountInfoResponses[keyof GetAccountInfoResponses];

export type LogoutData = {
    body?: never;
    path?: never;
    query?: never;
};

export type LogoutResponses = {
    /**
     * Logout and revoke API key
     */
    200: unknown;
};

export type RefreshAccountData = {
    body?: never;
    path?: never;
    query?: never;
};

export type RefreshAccountResponses = {
    /**
     * Re-read account status; remote re-verification runs only when the last successful check is older than one day
     */
    200: AccountInfoResponse;
};

export type RefreshAccountResponse = RefreshAccountResponses[keyof RefreshAccountResponses];

export type ExportBackupToCloudData = {
    body?: never;
    path?: never;
    query?: never;
};

export type ExportBackupToCloudResponses = {
    /**
     * Export backup to the cloud
     */
    200: unknown;
};

export type ImportBackupFromCloudData = {
    body?: never;
    path?: never;
    query?: never;
};

export type ImportBackupFromCloudResponses = {
    /**
     * Import backup from the cloud
     */
    200: unknown;
};

export type ExportBackupData = {
    body?: never;
    path?: never;
    query?: never;
};

export type ExportBackupResponses = {
    /**
     * Export engine configuration backup
     */
    200: EngineBackup;
};

export type ExportBackupResponse = ExportBackupResponses[keyof ExportBackupResponses];

export type ImportBackupData = {
    body: EngineBackup;
    path?: never;
    query?: never;
};

export type ImportBackupResponses = {
    /**
     * Import engine configuration backup
     */
    200: unknown;
};

export type GetDebugProPolicyData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetDebugProPolicyResponses = {
    /**
     * Current debug licence policy
     */
    200: DebugProPolicy;
};

export type GetDebugProPolicyResponse = GetDebugProPolicyResponses[keyof GetDebugProPolicyResponses];

export type SetDebugProPolicyData = {
    body?: never;
    path: {
        /**
         * The debug licence policy to set
         */
        policy: DebugProPolicy;
    };
    query?: never;
};

export type SetDebugProPolicyResponses = {
    /**
     * Debug licence policy set successfully
     */
    200: unknown;
};

export type HideFilesData = {
    body?: never;
    path?: never;
    query?: never;
};

export type HideFilesResponses = {
    /**
     * Files hidden successfully
     */
    200: unknown;
};

export type ShowFilesData = {
    body?: never;
    path?: never;
    query?: never;
};

export type ShowFilesResponses = {
    /**
     * Files restored successfully
     */
    200: unknown;
};

export type ResetTosData = {
    body?: never;
    path?: never;
    query?: never;
};

export type ResetTosResponses = {
    /**
     * TOS and privacy policy acceptance has been reset
     */
    200: unknown;
};

export type GetDebugResistanceLevelData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetDebugResistanceLevelResponses = {
    /**
     * Current debug resistance level
     */
    200: DebugResistanceLevel;
};

export type GetDebugResistanceLevelResponse = GetDebugResistanceLevelResponses[keyof GetDebugResistanceLevelResponses];

export type SetDebugResistanceLevelData = {
    body?: never;
    path: {
        /**
         * The debug resistance level to set
         */
        level: DebugResistanceLevel;
    };
    query?: never;
};

export type SetDebugResistanceLevelResponses = {
    /**
     * Debug resistance level set successfully
     */
    200: unknown;
};

export type GetUpdateCheckIntervalData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetUpdateCheckIntervalResponses = {
    /**
     * Current background update-check interval in seconds
     */
    200: UpdateCheckIntervalResponse;
};

export type GetUpdateCheckIntervalResponse = GetUpdateCheckIntervalResponses[keyof GetUpdateCheckIntervalResponses];

export type SetUpdateCheckIntervalData = {
    body?: never;
    path: {
        /**
         * New interval in seconds for the background update check
         */
        seconds: number;
    };
    query?: never;
};

export type SetUpdateCheckIntervalResponses = {
    /**
     * Interval updated successfully
     */
    200: unknown;
};

export type GetAllNetworkBlocksData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetAllNetworkBlocksResponses = {
    /**
     * List of all network blocks
     */
    200: GetAllNetworkBlocksResponse;
};

export type GetAllNetworkBlocksResponse2 = GetAllNetworkBlocksResponses[keyof GetAllNetworkBlocksResponses];

export type CreateNetworkBlockData = {
    body: CreateNetworkBlockRequest;
    path?: never;
    query?: never;
};

export type CreateNetworkBlockErrors = {
    /**
     * Internal server error
     */
    500: unknown;
};

export type CreateNetworkBlockResponses = {
    /**
     * Create a new network block
     */
    200: CreateNetworkBlockResponse;
};

export type CreateNetworkBlockResponse2 = CreateNetworkBlockResponses[keyof CreateNetworkBlockResponses];

export type SaveNetworkBlockData = {
    body: SaveNetworkBlockRequest;
    path?: never;
    query?: never;
};

export type SaveNetworkBlockResponses = {
    200: SaveNetworkBlockResponse;
};

export type GetDashboardSnapshotData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetDashboardSnapshotResponses = {
    /**
     * Full dashboard snapshot with all block data and settings
     */
    200: DashboardSnapshotResponse;
};

export type GetDashboardSnapshotResponse = GetDashboardSnapshotResponses[keyof GetDashboardSnapshotResponses];

export type DeleteNetworkBlockData = {
    body?: never;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type DeleteNetworkBlockErrors = {
    /**
     * Internal server error
     */
    500: unknown;
};

export type DeleteNetworkBlockResponses = {
    /**
     * Delete a network block
     */
    200: unknown;
};

export type RenameNetworkBlockData = {
    body: RenameNetworkBlockRequest;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type RenameNetworkBlockErrors = {
    /**
     * Network block is locked
     */
    403: unknown;
    /**
     * Network block not found
     */
    404: unknown;
};

export type RenameNetworkBlockResponses = {
    /**
     * Network block renamed successfully
     */
    200: unknown;
};

export type SetNetworkBlockEnabledData = {
    body: SetEnabledRequest;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type SetNetworkBlockEnabledErrors = {
    /**
     * Network block is locked and cannot be disabled
     */
    403: unknown;
    /**
     * Network block not found
     */
    404: unknown;
};

export type SetNetworkBlockEnabledResponses = {
    /**
     * Network block enabled state updated successfully
     */
    200: unknown;
};

export type SetNetworkBlockIconData = {
    body: SetIconRequest;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type SetNetworkBlockIconErrors = {
    /**
     * Network block not found
     */
    404: unknown;
};

export type SetNetworkBlockIconResponses = {
    /**
     * Icon updated successfully
     */
    200: unknown;
};

export type GetLockData = {
    body?: never;
    path: {
        /**
         * Network List ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type GetLockResponses = {
    /**
     * Get lock status of a network list
     */
    200: GetNetworkLockResponse;
};

export type GetLockResponse = GetLockResponses[keyof GetLockResponses];

export type SetLockData = {
    body: SetNetworkLockRequest;
    path: {
        /**
         * Network block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type SetLockResponses = {
    /**
     * Set lock status of a network list
     */
    200: SetNetworkLockResponse;
};

export type SetLockResponse = SetLockResponses[keyof SetLockResponses];

export type SetNetworkBlockSafeSearchData = {
    body: SetEnabledRequest;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type SetNetworkBlockSafeSearchErrors = {
    /**
     * Network block is locked
     */
    403: unknown;
    /**
     * Network block not found
     */
    404: unknown;
};

export type SetNetworkBlockSafeSearchResponses = {
    /**
     * Safe search setting updated successfully
     */
    200: unknown;
};

export type GetScheduleEntriesData = {
    body?: never;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type GetScheduleEntriesResponses = {
    /**
     * Get schedule entries for a network list
     */
    200: GetScheduleEntriesResponse;
};

export type GetScheduleEntriesResponse2 = GetScheduleEntriesResponses[keyof GetScheduleEntriesResponses];

export type AddScheduleEntriesData = {
    body: AddScheduleEntriesRequest;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type AddScheduleEntriesResponses = {
    /**
     * Add schedule entries to a network list
     */
    200: unknown;
};

export type GetNetBlockToSitePackConnectionsData = {
    body?: never;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type GetNetBlockToSitePackConnectionsResponses = {
    /**
     * Get site lists connected to a network block
     */
    200: GetNetListToSitePacksResponse;
};

export type GetNetBlockToSitePackConnectionsResponse = GetNetBlockToSitePackConnectionsResponses[keyof GetNetBlockToSitePackConnectionsResponses];

export type AddNetBlockToSitePackConnectionData = {
    body: AddNetListToSitePackRequest;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type AddNetBlockToSitePackConnectionResponses = {
    /**
     * Add a connection between a network block and a sites list
     */
    200: AddNetListToSitePackResponse;
};

export type AddNetBlockToSitePackConnectionResponse = AddNetBlockToSitePackConnectionResponses[keyof AddNetBlockToSitePackConnectionResponses];

export type RemoveNetBlockToSitePackConnectionData = {
    body?: never;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
        /**
         * Sites List ID
         */
        site_pack_id: string;
    };
    query?: never;
};

export type RemoveNetBlockToSitePackConnectionResponses = {
    /**
     * Remove the connection between a network block and a sites list
     */
    200: unknown;
};

export type ConfirmUnlockData = {
    body: UnlockChallengeConfirmRequest;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type ConfirmUnlockErrors = {
    /**
     * Challenge ID mismatch
     */
    400: unknown;
    /**
     * No active challenge for this block
     */
    404: unknown;
    /**
     * Cooldown not yet elapsed
     */
    425: TooEarlyResponse;
};

export type ConfirmUnlockError = ConfirmUnlockErrors[keyof ConfirmUnlockErrors];

export type ConfirmUnlockResponses = {
    /**
     * Unlock successful
     */
    200: UnlockChallengeConfirmResponse;
};

export type ConfirmUnlockResponse = ConfirmUnlockResponses[keyof ConfirmUnlockResponses];

export type InitiateUnlockData = {
    body?: never;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type InitiateUnlockErrors = {
    /**
     * Block lock is not SoftUnlockable
     */
    403: unknown;
    /**
     * Network block not found
     */
    404: unknown;
};

export type InitiateUnlockResponses = {
    /**
     * Challenge initiated
     */
    200: UnlockChallengeInitiateResponse;
};

export type InitiateUnlockResponse = InitiateUnlockResponses[keyof InitiateUnlockResponses];

export type GetUserRulesData = {
    body?: never;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type GetUserRulesResponses = {
    /**
     * Get user defined rules for a network list
     */
    200: GetUserDefinedRulesResponse;
};

export type GetUserRulesResponse = GetUserRulesResponses[keyof GetUserRulesResponses];

export type AddUserRulesData = {
    body: AddUserDefinedRulesRequest;
    path: {
        /**
         * Network Block ID
         */
        network_block_id: string;
    };
    query?: never;
};

export type AddUserRulesResponses = {
    /**
     * Add user defined rules to a network list
     */
    200: AddUserDefinedRulesResponse;
};

export type AddUserRulesResponse = AddUserRulesResponses[keyof AddUserRulesResponses];

export type OpenProPageData = {
    body?: never;
    path?: never;
    query?: never;
};

export type OpenProPageErrors = {
    /**
     * Failed to open browser
     */
    500: unknown;
};

export type OpenProPageResponses = {
    /**
     * Open the Pro page in the default browser
     */
    200: unknown;
};

export type PingData = {
    body?: never;
    path?: never;
    query?: never;
};

export type PingResponses = {
    /**
     * Ping the engine
     */
    200: string;
};

export type PingResponse = PingResponses[keyof PingResponses];

export type DownloadResistanceScriptsData = {
    body?: never;
    path?: never;
    query?: never;
};

export type DownloadResistanceScriptsResponses = {
    /**
     * Resistance scripts downloaded successfully
     */
    200: unknown;
};

export type GetAllScriptsData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetAllScriptsResponses = {
    /**
     * Get all scripts
     */
    200: GetAllScriptsResponse;
};

export type GetAllScriptsResponse2 = GetAllScriptsResponses[keyof GetAllScriptsResponses];

export type GetScriptData = {
    body?: never;
    path: {
        /**
         * Script ID
         */
        script_id: string;
    };
    query?: never;
};

export type GetScriptErrors = {
    /**
     * Script not found
     */
    404: unknown;
};

export type GetScriptResponses = {
    /**
     * Get script by ID
     */
    200: ScriptInfo;
};

export type GetScriptResponse = GetScriptResponses[keyof GetScriptResponses];

export type SetScriptEnabledData = {
    body: SetEnabledRequest;
    path: {
        /**
         * Script ID
         */
        script_id: string;
    };
    query?: never;
};

export type SetScriptEnabledErrors = {
    /**
     * Resistance engine is locked
     */
    403: unknown;
    /**
     * Script not found
     */
    404: unknown;
};

export type SetScriptEnabledResponses = {
    /**
     * Script enabled/disabled successfully
     */
    200: unknown;
};

export type SetResistanceModeEnabledData = {
    body: SetEnabledRequest;
    path?: never;
    query?: never;
};

export type SetResistanceModeEnabledErrors = {
    /**
     * Resistance engine is locked
     */
    403: unknown;
};

export type SetResistanceModeEnabledResponses = {
    /**
     * Resistance engine enabled/disabled successfully
     */
    200: unknown;
};

export type SetResistanceModeLockData = {
    body: SetNetworkLockRequest;
    path?: never;
    query?: never;
};

export type SetResistanceModeLockErrors = {
    /**
     * Resistance engine is locked or disabled
     */
    403: unknown;
};

export type SetResistanceModeLockResponses = {
    /**
     * Resistance engine lock updated successfully
     */
    200: unknown;
};

export type GetResistanceModeSettingsData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetResistanceModeSettingsResponses = {
    /**
     * Get resistance engine settings
     */
    200: ResistanceModeSettings;
};

export type GetResistanceModeSettingsResponse = GetResistanceModeSettingsResponses[keyof GetResistanceModeSettingsResponses];

export type GetResistanceModeTypeData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetResistanceModeTypeResponses = {
    /**
     * Get current resistance mode type
     */
    200: ResistanceModeTypeResponse;
};

export type GetResistanceModeTypeResponse = GetResistanceModeTypeResponses[keyof GetResistanceModeTypeResponses];

export type SetResistanceModeTypeData = {
    body: PunishmentMode;
    path?: never;
    query?: never;
};

export type SetResistanceModeTypeErrors = {
    /**
     * Invalid mode type or unsupported mode (dynamic is not available in v1 beta)
     */
    400: unknown;
    /**
     * Resistance mode is locked
     */
    403: unknown;
};

export type SetResistanceModeTypeResponses = {
    /**
     * Set resistance mode type
     */
    200: ResistanceModeTypeResponse;
};

export type SetResistanceModeTypeResponse = SetResistanceModeTypeResponses[keyof SetResistanceModeTypeResponses];

export type DeleteScheduleEntryData = {
    body?: never;
    path: {
        /**
         * Schedule Entry ID
         */
        schedule_entry_id: string;
    };
    query?: never;
};

export type DeleteScheduleEntryResponses = {
    /**
     * Delete a schedule entry
     */
    200: unknown;
};

export type EditScheduleEntryData = {
    body: BaseScheduleEntrySchema;
    path: {
        /**
         * Schedule Entry ID
         */
        schedule_entry_id: string;
    };
    query?: never;
};

export type EditScheduleEntryResponses = {
    /**
     * Edit a schedule entry
     */
    200: unknown;
};

export type GetAllSitePacksData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetAllSitePacksResponses = {
    /**
     * List of all site lists
     */
    200: GetAllSitePacksResponse;
};

export type GetAllSitePacksResponse2 = GetAllSitePacksResponses[keyof GetAllSitePacksResponses];

export type CreateSitePackData = {
    body: CreateSitePackRequest;
    path?: never;
    query?: never;
};

export type CreateSitePackResponses = {
    /**
     * Create a new site list, returns a typed IPC event stream
     */
    200: SitePackCreationStatus;
};

export type CreateSitePackResponse = CreateSitePackResponses[keyof CreateSitePackResponses];

export type ReEnableRemoteSitePacksData = {
    body?: never;
    path?: never;
    query?: never;
};

export type ReEnableRemoteSitePacksResponses = {
    /**
     * Re-enable all ignored remote site lists and re-fetch their metadata
     */
    200: unknown;
};

export type DeleteSitePackSourceData = {
    body?: never;
    path: {
        /**
         * Site List Source ID
         */
        source_id: string;
    };
    query?: never;
};

export type DeleteSitePackSourceResponses = {
    /**
     * Delete a site list source
     */
    200: unknown;
};

export type DeleteSitePackData = {
    body?: never;
    path: {
        /**
         * Site List ID
         */
        site_pack_id: string;
    };
    query?: never;
};

export type DeleteSitePackErrors = {
    /**
     * Site list is in use by a network block and cannot be deleted
     */
    409: unknown;
};

export type DeleteSitePackResponses = {
    /**
     * Delete a site list. For remote site lists, marks them as ignored instead of deleting.
     */
    200: unknown;
};

export type ResyncSitePackData = {
    body?: never;
    path: {
        /**
         * Site List ID
         */
        site_pack_id: string;
    };
    query?: never;
};

export type ResyncSitePackResponses = {
    /**
     * Resync a site list, returns a typed IPC event stream
     */
    200: SitePackCreationStatus;
};

export type ResyncSitePackResponse = ResyncSitePackResponses[keyof ResyncSitePackResponses];

export type GetSitePackSourcesData = {
    body?: never;
    path: {
        /**
         * Site List ID
         */
        site_pack_id: string;
    };
    query?: never;
};

export type GetSitePackSourcesResponses = {
    /**
     * Get sources of a site list
     */
    200: GetSitePackSourcesResponse;
};

export type GetSitePackSourcesResponse2 = GetSitePackSourcesResponses[keyof GetSitePackSourcesResponses];

export type AddSitePackSourceData = {
    body: AddSitePackSourceRequest;
    path: {
        /**
         * Site List ID
         */
        site_pack_id: string;
    };
    query?: never;
};

export type AddSitePackSourceResponses = {
    /**
     * Add a source to a site list
     */
    200: AddSitePackSourceResponse;
};

export type AddSitePackSourceResponse2 = AddSitePackSourceResponses[keyof AddSitePackSourceResponses];

export type ShutdownData = {
    body?: never;
    path?: never;
    query?: never;
};

export type ShutdownErrors = {
    /**
     * Cannot shut down: locks are active
     */
    423: unknown;
};

export type ShutdownResponses = {
    /**
     * Shutdown initiated
     */
    200: unknown;
};

export type CheckUpdatesData = {
    body?: never;
    path?: never;
    query?: never;
};

export type CheckUpdatesResponses = {
    /**
     * Aggregated update status for engine and content
     */
    200: UpdateCheckResponse;
};

export type CheckUpdatesResponse = CheckUpdatesResponses[keyof CheckUpdatesResponses];

export type TriggerContentUpdateData = {
    body?: never;
    path?: never;
    query?: never;
};

export type TriggerContentUpdateErrors = {
    /**
     * Update check or application failed
     */
    500: unknown;
};

export type TriggerContentUpdateResponses = {
    /**
     * Content update check triggered and applied
     */
    200: unknown;
};

export type GetVersionData = {
    body?: never;
    path?: never;
    query?: never;
};

export type GetVersionResponses = {
    /**
     * Engine version info
     */
    200: VersionInfo;
};

export type GetVersionResponse = GetVersionResponses[keyof GetVersionResponses];

export type DeleteUserRuleData = {
    body?: never;
    path: {
        /**
         * User Rule ID
         */
        user_rule_id: string;
    };
    query?: never;
};

export type DeleteUserRuleResponses = {
    /**
     * Delete a user defined rule
     */
    200: unknown;
};
