"""Seed the dev DB with fixture projects for visual testing.

Usage (from backend/):
    uv run python dev/seed.py

Resets ALL projects in ~/.local/share/pov/pov.db to the fixture set,
and seeds synthetic git commits so the activity heatmap has data.
"""

import hashlib
import json
import os
import random
import re
import sqlite3
import subprocess
import uuid
from datetime import datetime, timedelta, timezone
from pathlib import Path

FIXTURES = Path(__file__).parent.parent.parent / "fixtures"
POV_DIR = Path.home() / ".local" / "share" / "pov-dev"
DB_PATH = POV_DIR / "pov.db"
CONFIG_FILE = POV_DIR / "config.json"

CHECKBOX_RE = re.compile(r"^(\s*- \[).\]")


def days_ago(n: int) -> float:
    return (datetime.now(tz=timezone.utc) - timedelta(days=n)).timestamp()


def task_hash(line: str) -> str:
    # Must match pov/tasks.py: sha256 of normalized line, truncated to 16 hex chars.
    normalized = CHECKBOX_RE.sub(r"\1 ]", line.rstrip())
    return hashlib.sha256(normalized.encode()).hexdigest()[:16]


def unchecked_hashes(path: Path) -> list[str]:
    """Return task hashes for all unchecked top-level tasks in a file."""
    hashes = []
    for line in path.read_text().splitlines():
        if re.match(r"^\s*- \[ \]", line):
            hashes.append(task_hash(line))
    return hashes


SEED = [
    {
        "name": "Active this week",
        "file": "active-this-week.md",
        "status": "open",
        "sub_status": None,
        "mtime_days_ago": 2,
        "select_tasks": 2,  # pre-select first N unchecked tasks
    },
    {
        "name": "Active this month",
        "file": "active-this-month.md",
        "status": "open",
        "sub_status": None,
        "mtime_days_ago": 15,
        "select_tasks": 0,
    },
    {
        "name": "Older activity",
        "file": "older-activity.md",
        "status": "open",
        "sub_status": None,
        "mtime_days_ago": 60,
        "select_tasks": 0,
    },
    {
        "name": "No activity",
        "file": None,  # intentionally missing file → activity = "none"
        "status": "open",
        "sub_status": None,
        "mtime_days_ago": None,
        "select_tasks": 0,
    },
    {
        "name": "Archived – Paused",
        "file": "archived-paused.md",
        "status": "archived",
        "sub_status": "paused",
        "mtime_days_ago": 20,
        "select_tasks": 0,
    },
    {
        "name": "Archived – Done",
        "file": "archived-done.md",
        "status": "archived",
        "sub_status": "done",
        "mtime_days_ago": 10,
        "select_tasks": 0,
    },
    {
        "name": "Archived – Canceled",
        "file": "archived-canceled.md",
        "status": "archived",
        "sub_status": "canceled",
        "mtime_days_ago": 5,
        "select_tasks": 0,
    },
    # Learning entries
    {
        "name": "Maths",
        "file": "maths.md",
        "type": "learning",
        "status": "open",
        "sub_status": None,
        "mtime_days_ago": 3,
        "select_tasks": 1,
    },
    {
        "name": "Papers",
        "file": "papers.md",
        "type": "learning",
        "status": "open",
        "sub_status": None,
        "mtime_days_ago": 12,
        "select_tasks": 0,
    },
    {
        "name": "Books",
        "file": "books.md",
        "type": "learning",
        "status": "open",
        "sub_status": None,
        "mtime_days_ago": 4,
        "select_tasks": 0,
    },
    {
        "name": "Videos",
        "file": "videos.md",
        "type": "learning",
        "status": "open",
        "sub_status": None,
        "mtime_days_ago": 45,
        "select_tasks": 0,
    },
]


def main() -> None:
    (POV_DIR / "projects").mkdir(parents=True, exist_ok=True)
    if not CONFIG_FILE.exists():
        CONFIG_FILE.write_text('{"projects": [], "learning": {}}')
    db = sqlite3.connect(DB_PATH)
    db.execute("PRAGMA foreign_keys = ON")
    db.executescript("""
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, file_path TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL DEFAULT 'open', sub_status TEXT,
            type TEXT NOT NULL DEFAULT 'project', has_hardlink INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS selected_tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            task_hash TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(project_id, task_hash)
        );
        DROP TABLE IF EXISTS activity;
    """)
    db.execute("DELETE FROM projects")
    db.commit()

    config_projects = []

    for spec in SEED:
        project_id = str(uuid.uuid4())

        if spec["file"] is None:
            file_path = str(POV_DIR / "_missing_no_activity.md")
        else:
            src = FIXTURES / spec["file"]
            file_path = str(src)
            if spec["mtime_days_ago"] is not None:
                t = days_ago(spec["mtime_days_ago"])
                os.utime(src, (t, t))

        project_type = spec.get("type", "project")
        db.execute(
            "INSERT INTO projects (id, name, file_path, status, sub_status, type, has_hardlink)"
            " VALUES (?,?,?,?,?,?,?)",
            (project_id, spec["name"], file_path, spec["status"], spec["sub_status"], project_type, 0),
        )

        if spec["select_tasks"] > 0 and spec["file"] is not None:
            hashes = unchecked_hashes(FIXTURES / spec["file"])
            for h in hashes[: spec["select_tasks"]]:
                db.execute(
                    "INSERT OR IGNORE INTO selected_tasks (project_id, task_hash) VALUES (?,?)",
                    (project_id, h),
                )

        config_projects.append({"id": project_id, "name": spec["name"], "path": file_path})

    db.commit()
    db.close()

    CONFIG_FILE.write_text(json.dumps({"projects": config_projects, "learning": {}}, indent=2))
    print(f"✓ seeded {len(SEED)} projects → {DB_PATH}")

    _seed_activity_commits(config_projects)


def _seed_activity_commits(seeded_projects: list[dict]) -> None:
    """Backfill the pov-dev git repo with synthetic 'activity:' commits over
    the last 120 days so the heatmap has visible data after a fresh seed.

    Distribution: ~60% of recent days have at least one commit; weekday days
    are more likely than weekends; counts skew small (1–4) with occasional
    bursts (5–8). All commits go to the project type's directory if a
    hardlink can be made; otherwise we just commit the change message
    against the repo with --allow-empty.
    """
    if not (POV_DIR / ".git").exists():
        # init step usually creates this; skip if absent.
        subprocess.run(["git", "init", str(POV_DIR)], check=True, capture_output=True)
        subprocess.run(
            ["git", "-C", str(POV_DIR), "commit", "--allow-empty", "-m", "init"],
            check=True, capture_output=True,
        )

    rng = random.Random(42)
    open_projects = [
        p for p in seeded_projects
        if any(s["name"] == p["name"] and s.get("status") == "open" for s in SEED)
    ]
    if not open_projects:
        return

    now = datetime.now(tz=timezone.utc).replace(microsecond=0)
    base_env = {
        **os.environ,
        "GIT_AUTHOR_NAME": "pov-seed",
        "GIT_AUTHOR_EMAIL": "seed@pov.local",
        "GIT_COMMITTER_NAME": "pov-seed",
        "GIT_COMMITTER_EMAIL": "seed@pov.local",
    }

    for offset in range(119, -1, -1):
        day = now - timedelta(days=offset)
        weekday = day.weekday()  # Mon=0..Sun=6
        # Activity probability: ~70% weekdays, ~30% weekends.
        p_active = 0.7 if weekday < 5 else 0.3
        if rng.random() > p_active:
            continue
        # Most days have 1–4 commits; ~10% have a burst of 5–8.
        n = rng.randint(5, 8) if rng.random() < 0.1 else rng.randint(1, 4)
        for _ in range(n):
            proj = rng.choice(open_projects)
            # Pick a plausible time within the day (work hours weighted).
            hour = rng.choices(range(24), weights=[1]*8 + [3]*9 + [2]*4 + [1]*3)[0]
            minute = rng.randint(0, 59)
            ts = day.replace(hour=hour, minute=minute, second=rng.randint(0, 59))
            iso = ts.isoformat()
            env = {**base_env, "GIT_AUTHOR_DATE": iso, "GIT_COMMITTER_DATE": iso}
            # The activity router only attributes commits with this exact
            # subject format and a known project_id from the DB.
            msg = f"activity: {proj['id']}.md"
            subprocess.run(
                ["git", "-C", str(POV_DIR), "commit", "--allow-empty", "-m", msg],
                check=True, capture_output=True, env=env,
            )


if __name__ == "__main__":
    main()
