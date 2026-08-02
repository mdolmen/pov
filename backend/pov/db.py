"""SQLite database — schema init and connection."""

import aiosqlite

from pov.storage import POV_DIR

DB_PATH = POV_DIR / "pov.db"

CREATE_PROJECTS = """
CREATE TABLE IF NOT EXISTS projects (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    file_path    TEXT NOT NULL UNIQUE,
    status       TEXT NOT NULL DEFAULT 'open',
    sub_status   TEXT,
    type         TEXT NOT NULL DEFAULT 'project',
    has_hardlink INTEGER NOT NULL DEFAULT 1,
    paused_until TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
)
"""

CREATE_SELECTED_TASKS = """
CREATE TABLE IF NOT EXISTS selected_tasks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    task_hash   TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, task_hash)
)
"""

CREATE_TIME_ENTRIES = """
CREATE TABLE IF NOT EXISTS time_entries (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    date        TEXT NOT NULL,
    minutes     INTEGER NOT NULL,
    topic       TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
)
"""

CREATE_TIME_ENTRIES_INDEX = """
CREATE INDEX IF NOT EXISTS idx_time_entries_project_date
ON time_entries(project_id, date)
"""


async def init_db() -> None:
    """Create tables if they don't exist; apply any pending column migrations."""
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute("PRAGMA foreign_keys = ON")
        await db.execute(CREATE_PROJECTS)
        await db.execute(CREATE_SELECTED_TASKS)
        await db.execute(CREATE_TIME_ENTRIES)
        await db.execute(CREATE_TIME_ENTRIES_INDEX)
        await db.execute("DROP TABLE IF EXISTS activity")
        cursor = await db.execute("PRAGMA table_info(projects)")
        columns = {row[1] for row in await cursor.fetchall()}
        if "paused_until" not in columns:
            # ISO 8601 date string (YYYY-MM-DD); SQLite has no native DATE type.
            await db.execute("ALTER TABLE projects ADD COLUMN paused_until TEXT")
        await db.commit()


async def get_db() -> aiosqlite.Connection:
    """FastAPI dependency — yields an open DB connection."""
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute("PRAGMA foreign_keys = ON")
        db.row_factory = aiosqlite.Row
        yield db
