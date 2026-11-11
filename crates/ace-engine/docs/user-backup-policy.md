# Ace Engine User Backup Policy

This document defines backup rules, limits, and schemas for Ace Engine user configurations.

## 1. Cloud Storage Restrictions

- **Storage Limit**: Limit cloud backups to **1MB** per user. This limit supports extensive rule, configuration, and script lists without excessive storage.
- **Pro Feature**: Cloud saving requires **Pro**. Free users cannot upload or download cloud backups. Local import and export might be available.

## 2. Security and Import Validation

- **Minimal Validation**: During import, validate basic fields, such as UUIDs and time formats.
- **Pro Feature Safety**: Permit imports that contain Pro features, such as premium Site Packs URLs, regardless of license status. **The engine validates Pro status at runtime**. If a Free user imports a Pro site pack, the engine ignores that pack during URL evaluation.

## 3. Backup Contents

The backup JSON file contains these settings:

### I. Custom Site Packs

- Include only user-created site packs.
- Include each pack's UUID, name, description, and configured sources (remote URLs and local file paths).

### II. Network Blocks

- Include all custom network blocks.
- Save connected site packs (custom and default) by UUID.
- Save all user-defined RegEx rules.
- Save schedules (active time windows and days).
- **Excluded**: Do **not** save locks. A restored backup cannot accidentally lock a network block.

### III. Resistance Scripts State

- Save the state of deactivated resistance scripts.
- Save only each deactivated script's `ScriptId` to reduce JSON size.

## 4. JSON Schema Structure

The exported `EngineBackup` JSON approximately follows this schema:

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
