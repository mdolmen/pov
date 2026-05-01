"""CRUD endpoints for projects."""

import json
import os
import re
import subprocess
import uuid
from pathlib import Path
from typing import Literal

import aiosqlite
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from pov.activity import ActivityLevel, get_activity
from pov.db import get_db
from pov.storage import CONFIG_FILE, LEARNING_DIR, POV_DIR, PROJECTS_DIR

router = APIRouter(prefix="/projects", tags=["projects"])

Status = Literal["open", "archived"]
SubStatus = Literal["paused", "done", "canceled"] | None
ProjectType = Literal["project", "learning"]


class ProjectResponse(BaseModel):
    id: str
    name: str
    file_path: str
    status: Status
    sub_status: SubStatus
    type: str
    has_hardlink: bool
    task_count: int
    selected_count: int
    activity: ActivityLevel
    paused_until: str | None = None


class CreateProjectRequest(BaseModel):
    name: str
    file_path: str  # absolute path to the original TODO.md
    status: Status = "open"
    sub_status: SubStatus = None
    type: ProjectType = "project"


class UpdateProjectRequest(BaseModel):
    name: str | None = None
    status: Status | None = None
    sub_status: SubStatus | None = None
    # ISO 8601 date (YYYY-MM-DD). Empty string clears the value;
    # None means "don't change".
    paused_until: str | None = None


def _count_tasks(file_path: Path) -> int:
    try:
        text = file_path.read_text()
        return len(re.findall(r"^\s*- \[.\]", text, re.MULTILINE))
    except OSError:
        return 0


def _hardlink_path(project_id: str, type: str = "project") -> Path:
    base = LEARNING_DIR if type == "learning" else PROJECTS_DIR
    return base / f"{project_id}.md"


def _create_hardlink(src: Path, dst: Path) -> bool:
    """Try to create a hardlink. Returns True on success, False on cross-device."""
    try:
        os.link(src, dst)
        return True
    except OSError as e:
        if e.errno == 18:  # EXDEV: cross-device link
            return False
        raise


def _git_add_commit(file_path: Path, message: str) -> None:
    subprocess.run(
        ["git", "-C", str(POV_DIR), "add", str(file_path)],
        check=True, capture_output=True,
    )
    subprocess.run(
        ["git", "-C", str(POV_DIR), "commit", "-m", message],
        check=True, capture_output=True,
    )


def _read_config() -> dict:
    return json.loads(CONFIG_FILE.read_text())


def _write_config(config: dict) -> None:
    CONFIG_FILE.write_text(json.dumps(config, indent=2))


async def _row_to_response(row: aiosqlite.Row, db: aiosqlite.Connection) -> ProjectResponse:
    project_id = row["id"]
    file_path = Path(row["file_path"])
    has_hardlink = bool(row["has_hardlink"])

    cursor = await db.execute(
        "SELECT COUNT(*) FROM selected_tasks WHERE project_id = ?", (project_id,)
    )
    selected_count = (await cursor.fetchone())[0]

    return ProjectResponse(
        id=project_id,
        name=row["name"],
        file_path=str(file_path),
        status=row["status"],
        sub_status=row["sub_status"],
        type=row["type"],
        has_hardlink=has_hardlink,
        task_count=_count_tasks(file_path),
        selected_count=selected_count,
        activity=get_activity(file_path, has_hardlink),
        paused_until=row["paused_until"] if "paused_until" in row.keys() else None,
    )


@router.get("", response_model=list[ProjectResponse])
async def list_projects(db: aiosqlite.Connection = Depends(get_db)):
    cursor = await db.execute("SELECT * FROM projects ORDER BY created_at DESC")
    rows = await cursor.fetchall()
    return [await _row_to_response(row, db) for row in rows]


@router.post("", response_model=ProjectResponse, status_code=201)
async def create_project(body: CreateProjectRequest, db: aiosqlite.Connection = Depends(get_db)):
    src = Path(body.file_path).expanduser().resolve()
    if not src.exists():
        raise HTTPException(status_code=422, detail="file not found")

    project_id = str(uuid.uuid4())
    dst = _hardlink_path(project_id, body.type)
    has_hardlink = _create_hardlink(src, dst)

    if has_hardlink:
        _git_add_commit(dst, f"add: {body.name}")

    await db.execute(
        """INSERT INTO projects (id, name, file_path, status, sub_status, has_hardlink, type)
           VALUES (?, ?, ?, ?, ?, ?, ?)""",
        (project_id, body.name, str(src), body.status, body.sub_status, int(has_hardlink), body.type),
    )
    await db.commit()

    config = _read_config()
    config["projects"].append({"id": project_id, "name": body.name, "path": str(src)})
    _write_config(config)

    cursor = await db.execute("SELECT * FROM projects WHERE id = ?", (project_id,))
    row = await cursor.fetchone()
    return await _row_to_response(row, db)


@router.delete("/{project_id}", status_code=204)
async def delete_project(project_id: str, db: aiosqlite.Connection = Depends(get_db)):
    cursor = await db.execute("SELECT * FROM projects WHERE id = ?", (project_id,))
    row = await cursor.fetchone()
    if not row:
        raise HTTPException(status_code=404, detail="project not found")

    hardlink = _hardlink_path(project_id, row["type"])
    if hardlink.exists():
        hardlink.unlink()

    await db.execute("DELETE FROM projects WHERE id = ?", (project_id,))
    await db.commit()

    config = _read_config()
    config["projects"] = [p for p in config["projects"] if p["id"] != project_id]
    _write_config(config)


@router.patch("/{project_id}", response_model=ProjectResponse)
async def update_project(
    project_id: str,
    body: UpdateProjectRequest,
    db: aiosqlite.Connection = Depends(get_db),
):
    cursor = await db.execute("SELECT * FROM projects WHERE id = ?", (project_id,))
    if not await cursor.fetchone():
        raise HTTPException(status_code=404, detail="project not found")

    # Use exclude_unset so callers can explicitly null a field (e.g. clear paused_until).
    updates = body.model_dump(exclude_unset=True)
    # Treat empty string on paused_until as "clear".
    if updates.get("paused_until") == "":
        updates["paused_until"] = None
    if updates:
        set_clause = ", ".join(f"{k} = ?" for k in updates)
        await db.execute(
            f"UPDATE projects SET {set_clause} WHERE id = ?",
            (*updates.values(), project_id),
        )
        await db.commit()

    cursor = await db.execute("SELECT * FROM projects WHERE id = ?", (project_id,))
    row = await cursor.fetchone()
    return await _row_to_response(row, db)
