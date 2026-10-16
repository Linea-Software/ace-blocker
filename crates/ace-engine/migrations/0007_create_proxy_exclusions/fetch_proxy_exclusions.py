from pathlib import Path

# Broad compatibility exclusion lists are intentionally retired. Re-running this
# helper must never reintroduce a site-by-site bypass list.
script_dir = Path(__file__).resolve().parent
output_path = script_dir / "up.sql"
output_path.write_text(
    "CREATE TABLE IF NOT EXISTS proxy_exclusions (\n"
    "    host TEXT NOT NULL PRIMARY KEY\n"
    ");\n\n"
    "-- Intentionally empty. TLS is opaque by default; interception is policy-driven.\n",
    encoding="utf-8",
)
print(f"Successfully generated {output_path}")
