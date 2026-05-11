"""Activity heatmap endpoint, sourced from each project's own git repo."""

import subprocess
from collections import Counter
from datetime import datetime
from pathlib import Path
from typing import Literal

import aiosqlite
from fastapi import APIRouter, Depends
from pydantic import BaseModel

from pov.db import get_db

router = APIRouter(prefix="/activity", tags=["activity"])

ProjectType = Literal["project", "learning"]


class ActivityDay(BaseModel):
    date: str  # YYYY-MM-DD (local time)
    count: int


def _find_git_root(file_path: Path) -> Path | None:
    """Walk up from file_path to find the enclosing git repo root."""
    current = file_path.parent
    while True:
        if (current / ".git").exists():
            return current
        parent = current.parent
        if parent == current:
            return None
        current = parent


def _commits_since(git_root: Path, days: int) -> list[str]:
    """Return ISO author-date timestamps for commits in the last N days."""
    result = subprocess.run(
        ["git", "-C", str(git_root), "log", f"--since={days} days ago", "--format=%aI"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0 or not result.stdout:
        return []
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


@router.get("", response_model=list[ActivityDay])
async def get_activity(
    type: ProjectType,
    days: int = 120,
    db: aiosqlite.Connection = Depends(get_db),
):
    """Return per-day commit counts across all project repos for the last `days` days.

    Each project's file_path is used to locate its git repo root. Repos shared
    by multiple projects are counted once. Days with zero commits are omitted.
    """
    cursor = await db.execute(
        "SELECT file_path FROM projects WHERE type = ?", (type,)
    )
    rows = await cursor.fetchall()

    git_roots: set[Path] = set()
    for row in rows:
        root = _find_git_root(Path(row["file_path"]))
        if root is not None:
            git_roots.add(root)

    counts: Counter[str] = Counter()
    for root in git_roots:
        for ts in _commits_since(root, days):
            try:
                dt = datetime.fromisoformat(ts)
            except ValueError:
                continue
            counts[dt.date().isoformat()] += 1

    return [ActivityDay(date=d, count=c) for d, c in sorted(counts.items())]
