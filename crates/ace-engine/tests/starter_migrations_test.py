"""Regression tests for the starter-block SQL migrations (stdlib SQLite only)."""

from pathlib import Path
import sqlite3
import unittest


MIGRATIONS = Path(__file__).resolve().parents[1] / "migrations"
BLOCK_PREFIX = "01950000-0000-7000-8000-00000000000"
BLOCK_IDS = [f"{BLOCK_PREFIX}{index}" for index in range(1, 5)]
SCHEDULE_IDS = [
    f"01950000-0000-7000-8000-00000000100{index}"
    for index in range(1, 5)
]


def run_migrations(db: sqlite3.Connection, through: int) -> None:
    for migration in sorted(MIGRATIONS.iterdir()):
        if not migration.is_dir():
            continue
        number = int(migration.name.split("_", 1)[0])
        if number > through:
            break
        db.executescript((migration / "up.sql").read_text(encoding="utf-8"))


def block_states(db: sqlite3.Connection) -> list[tuple[str, int]]:
    return db.execute(
        "SELECT id, enabled FROM network_blocks WHERE id IN (" +
        ",".join("?" for _ in BLOCK_IDS) + ") ORDER BY id", BLOCK_IDS
    ).fetchall()


def schedules(db: sqlite3.Connection, block_id: str) -> list[tuple]:
    return db.execute(
        "SELECT id, start_time, end_time, schedule_days_bitmask "
        "FROM network_block_schedule_entries WHERE network_block_id = ? ORDER BY id",
        (block_id,),
    ).fetchall()


class StarterMigrationsTest(unittest.TestCase):
    def test_fresh_seed_is_disabled_with_24_7_coverage(self):
        with sqlite3.connect(":memory:") as db:
            run_migrations(db, through=23)
            self.assertEqual(block_states(db), [(item, 0) for item in BLOCK_IDS])
            for block_id, schedule_id in zip(BLOCK_IDS, SCHEDULE_IDS):
                self.assertEqual(
                    schedules(db, block_id),
                    [(schedule_id, "00:00:00", "00:00:00", 127)],
                )

            # The enabled switch changes only the flag; the saved schedule
            # already matches at every hour of every weekday.
            db.execute("UPDATE network_blocks SET enabled = 1 WHERE id = ?", (BLOCK_IDS[0],))
            self.assertEqual(block_states(db)[0], (BLOCK_IDS[0], 1))
            self.assertEqual(schedules(db, BLOCK_IDS[0])[0][1:], ("00:00:00", "00:00:00", 127))

            # If seeding is accidentally reattempted, no second schedule is added.
            seed_sql = (MIGRATIONS / "0021_seed_default_network_blocks" / "up.sql").read_text(encoding="utf-8")
            db.executescript(seed_sql.split("-- Rule IDs must be UUIDs")[0])
            self.assertEqual(sum(len(schedules(db, block)) for block in BLOCK_IDS), 4)

    def test_upgrade_preserves_legacy_schedules_and_flags(self):
        with sqlite3.connect(":memory:") as db:
            run_migrations(db, through=22)
            # Reconstruct an older installation that ran 0021 before it added
            # seeded schedules. Its user-authored entries have different IDs.
            db.executemany(
                "DELETE FROM network_block_schedule_entries WHERE id = ?",
                [(item,) for item in SCHEDULE_IDS],
            )
            db.execute(
                "INSERT INTO network_block_schedule_entries "
                "(id, network_block_id, start_time, end_time, schedule_days_bitmask) "
                "VALUES (?, ?, '09:00:00', '11:00:00', 31)",
                ("01950000-0000-7000-8000-000000009001", BLOCK_IDS[0]),
            )
            before_schedules = {block: schedules(db, block) for block in BLOCK_IDS}
            # Apply the new 0023 to a pre-existing DB. Neither schedule nor
            # enabled flag may change due to a seeded-ID mismatch.
            db.executescript((MIGRATIONS / "0023_add_enabled_to_network_blocks" / "up.sql").read_text(encoding="utf-8"))
            self.assertEqual(block_states(db), [(item, 1) for item in BLOCK_IDS])
            self.assertEqual(
                {block: schedules(db, block) for block in BLOCK_IDS},
                before_schedules,
            )

            # Existing users who already applied 0023 and changed flags retain
            # those choices: no subsequent migration resets them.
            db.execute("UPDATE network_blocks SET enabled = 0 WHERE id = ?", (BLOCK_IDS[1],))
            self.assertEqual([enabled for _, enabled in block_states(db)], [1, 0, 1, 1])


if __name__ == "__main__":
    unittest.main()
