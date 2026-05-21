"""Task endpoints: list, toggle, select."""

import subprocess
from typing import Annotated, Literal, Union

import aiosqlite
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, Field

from pov.db import get_db
from pov.storage import LEARNING_DIR, POV_DIR, PROJECTS_DIR
from pov.tasks import Task, parse_items, toggle_cascade

router = APIRouter(tags=["tasks"])


class HeadingResponse(BaseModel):
    kind: Literal["heading"] = "heading"
    text: str
    level: int


class SubtaskResponse(BaseModel):
    hash: str
    text: str
    checked: bool
    line_number: int


class TaskResponse(BaseModel):
    kind: Literal["task"] = "task"
    hash: str
    text: str
    checked: bool
    line_number: int
    subtasks: list[SubtaskResponse]
    is_done: bool
    is_selected: bool


ListItemResponse = Annotated[
    Union[HeadingResponse, TaskResponse], Field(discriminator="kind")
]


async def _get_project_or_404(project_id: str, db: aiosqlite.Connection) -> aiosqlite.Row:
    cursor = await db.execute("SELECT * FROM projects WHERE id = ?", (project_id,))
    row = await cursor.fetchone()
    if not row:
        raise HTTPException(status_code=404, detail="project not found")
    return row


async def _selected_hashes(project_id: str, db: aiosqlite.Connection) -> set[str]:
    cursor = await db.execute(
        "SELECT task_hash FROM selected_tasks WHERE project_id = ?", (project_id,)
    )
    return {row[0] for row in await cursor.fetchall()}


def _task_to_response(task: Task, selected: set[str]) -> TaskResponse:
    return TaskResponse(
        hash=task.hash,
        text=task.text,
        checked=task.checked,
        line_number=task.line_number,
        subtasks=[
            SubtaskResponse(
                hash=s.hash, text=s.text, checked=s.checked, line_number=s.line_number
            )
            for s in task.subtasks
        ],
        is_done=task.is_done,
        is_selected=task.hash in selected,
    )


@router.get("/projects/{project_id}/tasks", response_model=list[ListItemResponse])
async def list_tasks(project_id: str, db: aiosqlite.Connection = Depends(get_db)):
    row = await _get_project_or_404(project_id, db)
    from pathlib import Path
    file_path = Path(row["file_path"])
    selected = await _selected_hashes(project_id, db)
    items = parse_items(file_path)
    result: list[HeadingResponse | TaskResponse] = []
    for item in items:
        if isinstance(item, Task):
            result.append(_task_to_response(item, selected))
        else:
            result.append(HeadingResponse(text=item.text, level=item.level))
    return result


@router.patch("/projects/{project_id}/tasks/{task_hash}", response_model=TaskResponse)
async def toggle_task(
    project_id: str, task_hash: str, db: aiosqlite.Connection = Depends(get_db)
):
    row = await _get_project_or_404(project_id, db)
    from pathlib import Path
    file_path = Path(row["file_path"])

    if not toggle_cascade(file_path, task_hash):
        raise HTTPException(status_code=404, detail="task not found")

    base = LEARNING_DIR if row["type"] == "learning" else PROJECTS_DIR
    hardlink = base / f"{project_id}.md"
    if hardlink.exists():
        subprocess.run(
            ["git", "-C", str(POV_DIR), "add", str(hardlink)],
            capture_output=True,
        )
        subprocess.run(
            ["git", "-C", str(POV_DIR), "commit", "-m", f"activity: {hardlink.name}"],
            capture_output=True,
        )

    from pov.tasks import parse_tasks
    tasks = parse_tasks(file_path)
    task = next((t for t in tasks if t.hash == task_hash), None)
    if task is None:
        task = next(
            (t for t in tasks if any(s.hash == task_hash for s in t.subtasks)),
            None,
        )
    if not task:
        raise HTTPException(status_code=404, detail="task not found after toggle")

    if task.is_done:
        await db.execute(
            "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ?",
            (project_id, task.hash),
        )
        await db.commit()

    selected = await _selected_hashes(project_id, db)
    return _task_to_response(task, selected)


@router.post("/projects/{project_id}/tasks/{task_hash}/select", status_code=204)
async def select_task(
    project_id: str, task_hash: str, db: aiosqlite.Connection = Depends(get_db)
):
    await _get_project_or_404(project_id, db)
    await db.execute(
        "INSERT OR IGNORE INTO selected_tasks (project_id, task_hash) VALUES (?, ?)",
        (project_id, task_hash),
    )
    await db.commit()


@router.delete("/projects/{project_id}/tasks/{task_hash}/select", status_code=204)
async def unselect_task(
    project_id: str, task_hash: str, db: aiosqlite.Connection = Depends(get_db)
):
    await _get_project_or_404(project_id, db)
    await db.execute(
        "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ?",
        (project_id, task_hash),
    )
    await db.commit()


@router.post("/tasks/cleanup", status_code=204)
async def cleanup_selected_tasks(db: aiosqlite.Connection = Depends(get_db)):
    """Remove selected_tasks rows whose task is marked done in the source file."""
    from pathlib import Path
    from pov.tasks import parse_tasks

    cursor = await db.execute("SELECT id, file_path FROM projects")
    projects = await cursor.fetchall()

    for project in projects:
        file_path = Path(project["file_path"])
        if not file_path.exists():
            continue
        done_hashes = {t.hash for t in parse_tasks(file_path) if t.is_done}
        if not done_hashes:
            continue
        placeholders = ",".join("?" * len(done_hashes))
        await db.execute(
            f"DELETE FROM selected_tasks WHERE project_id = ? AND task_hash IN ({placeholders})",
            (project["id"], *done_hashes),
        )

    await db.commit()
