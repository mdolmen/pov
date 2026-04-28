"""SQLite database — schema init and connection."""

import aiosqlite

from pov.storage import POV_DIR

DB_PATH = POV_DIR / "pov.db"

CREATE_PROJECTS = """
CREATE TABLE IF NOT EXISTS projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    file_path   TEXT NOT NULL UNIQUE,
    status      TEXT NOT NULL DEFAULT 'open',
    sub_status  TEXT,
    type        TEXT NOT NULL DEFAULT 'project',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
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

CREATE_ACTIVITY = """
CREATE TABLE IF NOT EXISTS activity (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    occurred_at TEXT NOT NULL DEFAULT (datetime('now'))
)
"""


async def init_db() -> None:
    """Create tables if they don't exist."""
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute("PRAGMA foreign_keys = ON")
        await db.execute(CREATE_PROJECTS)
        await db.execute(CREATE_SELECTED_TASKS)
        await db.execute(CREATE_ACTIVITY)
        await db.commit()


async def get_db() -> aiosqlite.Connection:
    """FastAPI dependency — yields an open DB connection."""
    async with aiosqlite.connect(DB_PATH) as db:
        await db.execute("PRAGMA foreign_keys = ON")
        db.row_factory = aiosqlite.Row
        yield db
