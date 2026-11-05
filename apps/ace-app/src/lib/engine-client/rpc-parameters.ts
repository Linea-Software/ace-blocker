// Stable bindings to the generated Rust RPC trait; no HTTP endpoint mapping.
export const rpcParameters = {
  "getAccountInfo": [],
  "refreshAccount": [],
  "acceptTos": [],
  "setApiKey": [
    "payload"
  ],
  "logout": [],
  "exportBackup": [],
  "exportBackupToCloud": [],
  "importBackupFromCloud": [],
  "importBackup": [
    "payload"
  ],
  "getDashboardSnapshot": [],
  "getDebugProPolicy": [],
  "setDebugProPolicy": [
    "policy"
  ],
  "getDebugResistanceLevel": [],
  "setDebugResistanceLevel": [
    "level"
  ],
  "hideFiles": [],
  "showFiles": [],
  "getUpdateCheckInterval": [],
  "setUpdateCheckInterval": [
    "seconds"
  ],
  "resetTos": [],
  "getLock": [
    "network_block_id"
  ],
  "setLock": [
    "network_block_id",
    "payload"
  ],
  "getNetBlockToSitePackConnections": [
    "network_block_id"
  ],
  "addNetBlockToSitePackConnection": [
    "network_block_id",
    "payload"
  ],
  "removeNetBlockToSitePackConnection": [
    "network_block_id",
    "site_pack_id"
  ],
  "renameNetworkBlock": [
    "network_block_id",
    "payload"
  ],
  "getAllNetworkBlocks": [],
  "createNetworkBlock": [
    "payload"
  ],
  "saveNetworkBlock": [
    "payload"
  ],
  "deleteNetworkBlock": [
    "network_block_id"
  ],
  "setNetworkBlockSafeSearch": [
    "network_block_id",
    "payload"
  ],
  "setNetworkBlockEnabled": [
    "network_block_id",
    "payload"
  ],
  "setNetworkBlockIcon": [
    "network_block_id",
    "payload"
  ],
  "getResistanceModeSettings": [],
  "setResistanceModeEnabled": [
    "payload"
  ],
  "setResistanceModeLock": [
    "payload"
  ],
  "getAllScripts": [],
  "getScript": [
    "script_id"
  ],
  "setScriptEnabled": [
    "script_id",
    "payload"
  ],
  "downloadResistanceScripts": [],
  "getScheduleEntries": [
    "network_block_id"
  ],
  "addScheduleEntries": [
    "network_block_id",
    "payload"
  ],
  "editScheduleEntry": [
    "schedule_entry_id",
    "entry"
  ],
  "deleteScheduleEntry": [
    "schedule_entry_id"
  ],
  "getAllSitePacks": [],
  "deleteSitePack": [
    "site_pack_id"
  ],
  "getSitePackSources": [
    "site_pack_id"
  ],
  "addSitePackSource": [
    "site_pack_id",
    "payload"
  ],
  "deleteSitePackSource": [
    "source_id"
  ],
  "reEnableRemoteSitePacks": [],
  "ping": [],
  "getVersion": [],
  "openProPage": [],
  "checkUpdates": [],
  "triggerContentUpdate": [],
  "initiateUnlock": [
    "network_block_id"
  ],
  "confirmUnlock": [
    "network_block_id",
    "payload"
  ],
  "getUserRules": [
    "network_block_id"
  ],
  "addUserRules": [
    "network_block_id",
    "payload"
  ],
  "deleteUserRule": [
    "user_rule_id"
  ],
  "getResistanceModeType": [],
  "setResistanceModeType": [
    "mode"
  ],
  "createSitePack": [
    "payload"
  ],
  "resyncSitePack": [
    "site_pack_id"
  ]
} as const satisfies Record<string, readonly string[]>;

export type RpcOperation = keyof typeof rpcParameters;
