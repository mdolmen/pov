"""Project domain logic shared between the FastAPI router and the CLI.

Pure-sync helpers that operate on the data dir (`POV_DIR`, `PROJECTS_DIR`,
`LEARNING_DIR`, `CONFIG_FILE`) and the SQLite DB. The router wraps these
in async DB calls; the CLI calls them directly with a sync `sqlite3`
connection.
"""

import json
import os
import sqlite3
import subprocess
import uuid
from pathlib import Path
from typing import Iterable

from pov.storage import CONFIG_FILE, LEARNING_DIR, POV_DIR, PROJECTS_DIR


def hardlink_path(project_id: str, type: str = "project") -> Path:
    base = LEARNING_DIR if type == "learning" else PROJECTS_DIR
    return base / f"{project_id}.md"


def create_hardlink(src: Path, dst: Path) -> bool:
    """Try to create a hardlink. Returns True on success, False on cross-device."""
    try:
        os.link(src, dst)
        return True
    except OSError as e:
        if e.errno == 18:  # EXDEV: cross-device link
            return False
        raise


def git_add_commit(file_path: Path, message: str) -> None:
    subprocess.run(
        ["git", "-C", str(POV_DIR), "add", str(file_path)],
        check=True, capture_output=True,
    )
    subprocess.run(
        ["git", "-C", str(POV_DIR), "commit", "-m", message],
        check=True, capture_output=True,
    )


def read_config() -> dict:
    return json.loads(CONFIG_FILE.read_text())


def write_config(config: dict) -> None:
    CONFIG_FILE.write_text(json.dumps(config, indent=2))


# --- Sync DB operations (used by the CLI) -----------------------------------


class ProjectExistsError(Exception):
    """Raised when a file_path is already tracked."""


class ProjectNotFoundError(Exception):
    """Raised when a project name (or id) doesn't match any row."""


class AmbiguousProjectError(Exception):
    """Raised when a project name matches more than one row."""


def add_project_sync(
    db: sqlite3.Connection,
    *,
    name: str,
    file_path: Path,
    type: str = "project",
    status: str = "open",
    sub_status: str | None = None,
) -> str:
    """Create a project row + hardlink + config entry. Returns project_id."""
    src = file_path.expanduser().resolve()
    if not src.exists():
        raise FileNotFoundError(src)

    existing = db.execute(
        "SELECT id FROM projects WHERE file_path = ?", (str(src),)
    ).fetchone()
    if existing is not None:
        raise ProjectExistsError(str(src))

    project_id = str(uuid.uuid4())
    dst = hardlink_path(project_id, type)
    has_hardlink = create_hardlink(src, dst)
    if has_hardlink:
        git_add_commit(dst, f"add: {name}")

    db.execute(
        """INSERT INTO projects (id, name, file_path, status, sub_status, has_hardlink, type)
           VALUES (?, ?, ?, ?, ?, ?, ?)""",
        (project_id, name, str(src), status, sub_status, int(has_hardlink), type),
    )
    db.commit()

    config = read_config()
    config.setdefault("projects", []).append(
        {"id": project_id, "name": name, "path": str(src)}
    )
    write_config(config)
    return project_id


def list_projects_sync(db: sqlite3.Connection) -> list[sqlite3.Row]:
    cursor = db.execute("SELECT * FROM projects ORDER BY name COLLATE NOCASE")
    return cursor.fetchall()


def find_project_by_name_sync(db: sqlite3.Connection, name: str) -> sqlite3.Row:
    rows = db.execute(
        "SELECT * FROM projects WHERE name = ?", (name,)
    ).fetchall()
    if not rows:
        raise ProjectNotFoundError(name)
    if len(rows) > 1:
        raise AmbiguousProjectError(name)
    return rows[0]


def remove_project_sync(db: sqlite3.Connection, project: sqlite3.Row) -> None:
    """Delete the project row, hardlink and config entry. Original file is untouched."""
    project_id = project["id"]
    type_ = project["type"] if "type" in project.keys() else "project"

    dst = hardlink_path(project_id, type_)
    if dst.exists():
        dst.unlink()

    db.execute("DELETE FROM projects WHERE id = ?", (project_id,))
    db.commit()

    config = read_config()
    config["projects"] = [
        p for p in config.get("projects", []) if p["id"] != project_id
    ]
    write_config(config)


def project_summary_lines(rows: Iterable[sqlite3.Row]) -> list[str]:
    """Format a list of project rows as aligned plaintext."""
    rows = list(rows)
    if not rows:
        return ["(no projects)"]
    name_w = max(len(r["name"]) for r in rows)
    return [
        f"{r['name'].ljust(name_w)}  {r['type']:<8}  {r['status']:<8}  {r['file_path']}"
        for r in rows
    ]
