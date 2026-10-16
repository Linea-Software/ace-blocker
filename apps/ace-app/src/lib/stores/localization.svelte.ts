export type TimeFormat = "system" | "12h" | "24h";
export type DateFormat = "system" | "DD/MM/YYYY" | "MM/DD/YYYY" | "YYYY-MM-DD";

class LocalizationStore {
    timeFormat = $state<TimeFormat>("system");
    dateFormat = $state<DateFormat>("system");

    /** Whether the OS uses 12-hour (AM/PM) time. Detected client-side. */
    systemUses12h = $state(false);

    setPreferences(prefs: { timeFormat: TimeFormat; dateFormat: DateFormat }) {
        this.timeFormat = prefs.timeFormat;
        this.dateFormat = prefs.dateFormat;
    }

    /** Detects the OS time format (client-side only, after mount). */
    detectSystemTimeFormat() {
        this.systemUses12h =
            new Intl.DateTimeFormat(undefined, { hour: "numeric" })
                .resolvedOptions().hour12 ?? false;
    }

    /** Whether times should be displayed in 12-hour (AM/PM) format. */
    get uses12h(): boolean {
        if (this.timeFormat === "12h") return true;
        if (this.timeFormat === "24h") return false;
        return this.systemUses12h;
    }

    /**
     * Formats an "HH:MM" (24-hour) string according to the user's time format.
     * E.g. "14:00" → "02:00 PM" in 12h mode, "14:00" in 24h mode.
     */
    formatTime(timeStr: string): string {
        const [hours, minutes] = timeStr.split(":").map(Number);
        if (
            hours === undefined ||
            minutes === undefined ||
            Number.isNaN(hours) ||
            Number.isNaN(minutes)
        ) {
            return timeStr;
        }
        if (this.uses12h) {
            let hour12 = hours % 12;
            if (hour12 === 0) hour12 = 12;
            const period = hours >= 12 ? "PM" : "AM";
            return `${hour12.toString().padStart(2, "0")}:${minutes
                .toString()
                .padStart(2, "0")} ${period}`;
        }
        return `${hours.toString().padStart(2, "0")}:${minutes
            .toString()
            .padStart(2, "0")}`;
    }
}

export const localizationStore = new LocalizationStore();
