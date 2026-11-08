"""First-install baseline: Free, disabled, always-on starter policies."""
from pathlib import Path
import sqlite3
import hashlib
import json
import unittest

MIGRATIONS = Path(__file__).resolve().parents[1] / "migrations"
BLOCK_IDS = [f"01950000-0000-7000-8000-00000000000{index}" for index in range(1, 5)]


def run_migrations(db: sqlite3.Connection) -> None:
    db.execute("PRAGMA foreign_keys = ON")
    for migration in sorted(MIGRATIONS.iterdir()):
        if migration.is_dir():
            db.executescript((migration / "up.sql").read_text(encoding="utf-8"))


class StarterMigrationsTest(unittest.TestCase):
    def test_fresh_install_defaults_are_free_disabled_and_always(self):
        with sqlite3.connect(":memory:") as db:
            run_migrations(db)
            states = db.execute(
                "SELECT id, enabled, schedule_mode FROM network_blocks "
                "WHERE id IN (" + ",".join("?" for _ in BLOCK_IDS) + ") ORDER BY id",
                BLOCK_IDS,
            ).fetchall()
            self.assertEqual(states, [(id, 0, "always") for id in BLOCK_IDS])
            count = db.execute(
                "SELECT COUNT(*) FROM network_block_schedule_entries "
                "WHERE network_block_id IN (" + ",".join("?" for _ in BLOCK_IDS) + ")",
                BLOCK_IDS,
            ).fetchone()[0]
            self.assertEqual(count, 0)
            connected = db.execute(
                "SELECT b.id, p.remote_id, p.requires_pro "
                "FROM network_block_to_site_pack c "
                "JOIN network_blocks b ON c.network_block_id = b.id "
                "JOIN site_packs p ON c.site_pack_id = p.id "
                "WHERE b.id IN (" + ",".join("?" for _ in BLOCK_IDS) + ") "
                "ORDER BY b.id", BLOCK_IDS,
            ).fetchall()
            self.assertEqual(connected, [
                (BLOCK_IDS[0], "pack-adult-free", 0),
                (BLOCK_IDS[1], "pack-social-free", 0),
                (BLOCK_IDS[2], "pack-gambling-free", 0),
                (BLOCK_IDS[3], "pack-streaming-free", 0),
            ])
            self.assertEqual(db.execute(
                "SELECT COUNT(*) FROM user_defined_rules WHERE network_block_id IN (" +
                ",".join("?" for _ in BLOCK_IDS) + ")", BLOCK_IDS,
            ).fetchone()[0], 0)

    def test_installer_bundle_contains_usable_verified_free_packs(self):
        bundle = Path(__file__).resolve().parents[3] / "installer" / "bundled-packs"
        manifest = json.loads((bundle / "manifest.json").read_text(encoding="utf-8"))
        packs = {entry["id"]: entry for entry in manifest["packs"]}
        for id, count in [("pack-gambling-free", 14), ("pack-streaming-free", 9)]:
            entry = packs[id]
            file = bundle / entry["file"]
            data = file.read_bytes()
            self.assertFalse(entry["requires_pro"])
            self.assertEqual(hashlib.sha256(data).hexdigest(), entry["sha256"])
            self.assertEqual(len(data), entry["size_bytes"])
            self.assertEqual(data[:16], b"SQLite format 3\x00")
            with sqlite3.connect("file:" + str(file) + "?mode=ro", uri=True) as db:
                self.assertEqual(db.execute("SELECT COUNT(*) FROM entries").fetchone()[0], count)
                self.assertEqual(db.execute("SELECT id FROM pack_meta").fetchone()[0], id)

    def test_original_tables_have_final_columns(self):
        with sqlite3.connect(":memory:") as db:
            run_migrations(db)
            blocks = {col[1] for col in db.execute("PRAGMA table_info(network_blocks)")}
            accounts = {col[1] for col in db.execute("PRAGMA table_info(accounts)")}
            self.assertTrue({"icon", "enabled", "schedule_mode", "schedule_timezone"} <= blocks)
            self.assertTrue({"tos_accepted_at", "entitlement_token",
                             "entitlement_public_key", "clock_high_watermark"} <= accounts)


if __name__ == "__main__":
    unittest.main()
