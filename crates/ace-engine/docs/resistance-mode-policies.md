# Resistance Mode Policies (v2.2 - "Strict Ladder" Update)

> ## ⚠️ Removal notice (2026-09-02)
>
> The escalation ladder documented below is **historical**. On 2026-09-02 the
> dynamic escalation/de-escalation engine, the probation fields, and the
> usage-report pipeline that fed them were **removed** (tracking removal).
> Resistance Mode now ships as a v1 beta with **static intensity only**:
>
> - `POST /resistance-mode/type` accepts only `{"type":"static", ...}`; the
>   `PunishmentMode::Dynamic` variant no longer exists (a raw `dynamic`
>   payload fails deserialization).
> - Persisted settings carry only `static_intensity` and `scripts_version`;
>   there are no probation or last-evaluated-date columns.
> - No usage data source exists anywhere: tracking scripts were removed from
>   the boredom-engine, the site, and the engine, and the `/report/usage`
>   receivers are gone.
> - The ladder sections below are kept as a dated design record only; they do
>   not describe shipped behavior.

This document outlines the operational logic and intensity levels for the Resistance Mode.

## Core Architecture: The Escalation Ladder

The system now enforces a strict sequential escalation path. **Users cannot skip tiers.** Surpassing any threshold simply promotes the user to the _next immediate tier_, at which point all usage counters and probation timers are reset to zero.

### Intensity Levels

```typescript
export enum Intensity {
  None = "none", // Zero Punishment: Full speed, no friction.
  Low = "low", // Standard UX: Subtle Latency/Friction (Entry warning).
  Medium = "medium", // Intermittent Latency (10-20% lag).
  High = "high", // Heavy Friction (40-60% lag, Grayscale).
  Extreme = "extreme", // Maximum Stress (80% lag, layout shifts, font obfuscation).
}
```

```

---

## State Transition Logic

Transitions are evaluated daily based on time spent on restricted domains. When a tier is escalated, the user begins Day 1 of the new tier with 0 accumulated minutes.

### 0. None Intensity (The "Focus" State)

The baseline state for disciplined usage. No multi-day tracking is required here, only a daily tripwire.

- **Escalation**: If **Daily Usage > 45m**, transition immediately to **Low**.
- **Maintenance**: Remains **None** indefinitely as long as daily usage is **≤ 45m**.

### 1. Low Intensity (The Warning)

The first layer of friction. The system now tracks both a daily spike and a multi-day creep.

- **Escalation (To Medium)**:
- **Daily Tripwire**: Usage **> 45m** in a single day.
- **Multi-Day Tripwire**: Cumulative usage **> 90m** across 3 days.

- **De-escalation (To None)**: Transition back to **None** after **1 day** with usage **≤ 15m**.
- **Maintenance**: Remains **Low** if usage stays below the escalation thresholds.

### 2. Medium Intensity (The Squeeze)

The friction becomes actively annoying. Thresholds tighten.

- **Escalation (To High)**:
- **Daily Tripwire**: Usage **> 30m** in a single day.
- **Multi-Day Tripwire**: Cumulative usage **> 75m** across 3 days.

- **Maintenance (Reset)**: If usage exceeds **20m** on any single day (but stays under the 30m escalation tripwire), the current probation timer resets to Day 1.
- **De-escalation (To Low)**: Transition to **Low** only after **2 consecutive days** with usage **≤ 15m**.

### 3. High Intensity (The Chokehold)

The web becomes thoroughly unpleasant to navigate. The system affords very little leniency.

- **Escalation (To Extreme)**:
- **Daily Tripwire**: Usage **> 20m** in a single day.
- **Multi-Day Tripwire**: Cumulative usage **> 45m** across 3 days.

- **Maintenance (Reset)**: If usage exceeds **15m** on any single day, the current probation timer resets to Day 1.
- **De-escalation (To Medium)**: Transition to **Medium** only after **4 consecutive days** with usage **≤ 10m**.

### 4. Extreme Intensity (Maximum Punishment)

The engine makes targeted apps practically unsuitable for use. There is no further escalation, only a grueling path out.

- **Probation Lock**: Minimum **8 Days**. The intensity cannot be lowered during this period.
- **Maintenance (Reset)**: If usage is **> 15m** on _any single day_, the 8-day lock completely **resets to Day 1**.
- **De-escalation (To High)**: Transition to **High** only after completing **8 consecutive days** with usage **≤ 15m** per day.

```
