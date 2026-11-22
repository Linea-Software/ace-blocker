import { createMemo, For, Show } from "solid-js";
import type { ScheduleEntrySchema } from "$engine-client";
import * as m from "$lib/paraglide/messages.js";
import { computeWeeklyCoverage, type WeekdayKey } from "$lib/utils/weekly-coverage";

const dayKeys: WeekdayKey[] = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
const days = [
  { key: "monday", label: m.monday_short }, { key: "tuesday", label: m.tuesday_short },
  { key: "wednesday", label: m.wednesday_short }, { key: "thursday", label: m.thursday_short },
  { key: "friday", label: m.friday_short }, { key: "saturday", label: m.saturday_short },
  { key: "sunday", label: m.sunday_short },
] as const satisfies ReadonlyArray<{ key: WeekdayKey; label: () => string }>;
const left = (start: number) => (start / 1440 * 100) + "%";
const width = (start: number, end: number) => ((end - start) / 1440 * 100) + "%";

export default function ScheduleTimeline(props: {entries?: ScheduleEntrySchema[]; active?: boolean; always?: boolean}) {
  const coverage = createMemo(() => computeWeeklyCoverage(props.entries ?? []));
  const now = new Date();
  const currentDay = dayKeys[now.getDay()] ?? "sunday";
  const currentMinute = now.getHours() * 60 + now.getMinutes();
  const summary = createMemo(() => {
    if (props.always) return m.block_mode_always();
    const activeDays = coverage().filter(row => row.ranges.length > 0);
    if (!activeDays.length) return m.schedule_coverage_none_summary();
    return activeDays.map(row => (days.find(day => day.key === row.day)?.label() ?? row.day) + ": " + row.ranges.length).join(", ");
  });
  return (
    <div class={["schedule-timeline", { inactive: props.active === false }]} role="img" aria-label={m.schedule_weekly_preview_aria({summary: summary()})}>
      <div class="scale" aria-hidden="true"><span>00</span><span>06</span><span>12</span><span>18</span><span>24</span></div>
      <div class="rows" aria-hidden="true">
        <For each={days}>{day => (
          <div class="day-row">
            <span class="day">{day.label()}</span>
            <span class="track">
              <Show when={props.always} fallback={
                <For each={coverage().find(row => row.day === day.key)?.ranges ?? []}>{range =>
                  <span class="window" style={{left: left(range.start), width: width(range.start, range.end)}} />
                }</For>
              }><span class="window" style={{left: "0%", width: "100%"}} /></Show>
              <Show when={day.key === currentDay}><span class="now-marker" style={{left: left(currentMinute)}} /></Show>
            </span>
          </div>
        )}</For>
      </div>
      <style>{"\n    .schedule-timeline {\n        display: grid;\n        gap: 5px;\n        min-width: 0;\n    }\n\n    .scale {\n        margin-left: 31px;\n        display: flex;\n        justify-content: space-between;\n        color: var(--muted-foreground);\n        font-family: \"IBM Plex Mono\", monospace;\n        font-size: 12px;\n        line-height: 1;\n    }\n\n    .rows {\n        display: grid;\n        gap: 5px;\n    }\n\n    .day-row {\n        min-width: 0;\n        display: grid;\n        grid-template-columns: 26px minmax(0, 1fr);\n        align-items: center;\n        gap: 5px;\n    }\n\n    .day {\n        color: var(--muted-foreground);\n        font-family: \"IBM Plex Mono\", monospace;\n        font-size: 12px;\n        font-weight: 500;\n        line-height: 1;\n    }\n\n    .track {\n        position: relative;\n        height: 8px;\n        overflow: hidden;\n        border-radius: 2px;\n        background: var(--muted);\n    }\n\n    .track::before,\n    .track::after {\n        content: \"\";\n        position: absolute;\n        inset-block: 0;\n        width: 1px;\n        background: color-mix(in srgb, var(--foreground) 12%, transparent);\n    }\n\n    .track::before {\n        left: 50%;\n    }\n\n    .track::after {\n        left: 75%;\n    }\n\n    .window {\n        position: absolute;\n        inset-block: 0;\n        min-width: 2px;\n        background: var(--ace-signal);\n    }\n\n    .inactive .window {\n        background: var(--ace-ash);\n    }\n\n    .now-marker {\n        position: absolute;\n        z-index: 2;\n        inset-block: -2px;\n        width: 2px;\n        background: var(--foreground);\n        opacity: 0.72;\n    }\n"}</style>
    </div>
  );
}
