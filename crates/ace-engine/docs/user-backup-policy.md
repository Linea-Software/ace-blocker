# Ace Engine User Backup Policy

This document outlines the backup policy, limitations, and schema definitions for the user configuration backups in Ace Engine.

## 1. Cloud Storage Restrictions

- **Storage Limit**: Cloud backups are strictly limited to **1MB** per user. This is more than enough to handle extensive lists of rules, configurations, and scripts without storing excessive data.
- **Pro Feature**: Cloud saving is a **Pro-only feature**. Free users cannot upload or download backups from the cloud, although local import/export might be available.

## 2. Security and Import Validation

- **Minimal Validation**: During the import process, only basic data validation is performed (e.g., ensuring valid UUIDs and time formats).
- **Pro Feature Safety**: We allow users to import Pro features (such as premium Site Packs URLs) regardless of their license status. This is because **the engine inherently validates Pro status at runtime**. If a free user imports a backup containing a Pro site pack, the engine will simply ignore the pack during URL evaluation as they lack the required license.

## 3. Backup Contents

The backup JSON artifact contains the following configurations:

### I. Custom Site Packs

- Only custom-created site packs are included.
- Contains the pack's UUID, name, description, and the configured sources (both remote URLs and local file paths).

### II. Network Blocks

- Includes all custom network blocks.
- Saves the connected site packs (both custom and default, via their UUIDs).
- Saves all user-defined RegEx rules.
- Saves the schedules (active timeframes and active days).
- **Excluded**: Locks are explicitly **not saved**. Restoring a backup will never result in an accidentally locked network block.

### III. Resistance Scripts State

- Saves the state of deactivated resistance scripts.
- Only the `ScriptId` of the deactivated scripts are stored, minimizing JSON size.

## 4. JSON Schema Structure

The structure of the exported `EngineBackup` JSON will roughly follow this schema:

```json
{
  "version": "1.0",
  "custom_site_packs": [
    {
      "id": "uuid-v7",
      "name": "My Custom List",
      "description": "Optional description",
      "sources": [
        {
          "source": "https://example.com/hosts",
          "source_type": "URL",
          "format": "Standard"
        }
      ]
    }
  ],
  "network_blocks": [
    {
      "id": "uuid-v7",
      "name": "Focus Mode",
      "schedules": [
        {
          "start_time": "09:00:00",
          "end_time": "17:00:00",
          "weekdays": 31
        }
      ],
      "user_rules": ["youtube.com", "twitter.com"],
      "connected_site_packs": ["uuid-of-default-or-custom-pack"]
    }
  ],
  "deactivated_resistance_scripts": ["script-id-1", "script-id-2"]
}
```
