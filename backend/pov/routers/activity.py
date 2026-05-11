"""Activity heatmap endpoint.

Projects with a git repo: counts commits in that repo.
Projects without a git repo: falls back to pov-data-repo activity commits.
"""

import re
import subprocess
from collections import Counter
from datetime import datetime
from pathlib import Path
from typing import Literal

import aiosqlite
from fastapi import APIRouter, Depends
from pydantic import BaseModel

from pov.db import get_db
from pov.storage import POV_DIR

router = APIRouter(prefix="/activity", tags=["activity"])

ProjectType = Literal["project", "learning"]

# Matches watcher commit messages: "activity: <project_id>.md"
_ACTIVITY_RE = re.compile(r"^activity:\s+([0-9a-f-]+)\.md$")


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


def _pov_commits_since(days: int) -> list[tuple[str, str]]:
    """Return [(iso_timestamp, subject)] from the pov data repo."""
    result = subprocess.run(
        ["git", "-C", str(POV_DIR), "log", f"--since={days} days ago",
         "--pretty=format:%aI%x09%s"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0 or not result.stdout:
        return []
    pairs = []
    for line in result.stdout.splitlines():
        if "\t" in line:
            ts, subject = line.split("\t", 1)
            pairs.append((ts, subject))
    return pairs


def _parse_date(ts: str) -> str | None:
    try:
        return datetime.fromisoformat(ts).date().isoformat()
    except ValueError:
        return None


@router.get("", response_model=list[ActivityDay])
async def get_activity(
    type: ProjectType,
    days: int = 120,
    db: aiosqlite.Connection = Depends(get_db),
):
    """Return per-day activity counts for the last `days` days.

    Projects with a git repo: aggregates commits from that repo (shared repos
    counted once). Projects without a git repo: falls back to pov-data-repo
    activity commits for that project. Days with zero activity are omitted.
    """
    cursor = await db.execute(
        "SELECT id, file_path FROM projects WHERE type = ?", (type,)
    )
    rows = await cursor.fetchall()

    git_roots: set[Path] = set()
    fallback_ids: set[str] = set()
    for row in rows:
        root = _find_git_root(Path(row["file_path"]))
        if root is not None:
            git_roots.add(root)
        else:
            fallback_ids.add(row["id"])

    counts: Counter[str] = Counter()

    for root in git_roots:
        for ts in _commits_since(root, days):
            date = _parse_date(ts)
            if date:
                counts[date] += 1

    if fallback_ids:
        for ts, subject in _pov_commits_since(days):
            m = _ACTIVITY_RE.match(subject)
            if m and m.group(1) in fallback_ids:
                date = _parse_date(ts)
                if date:
                    counts[date] += 1

    return [ActivityDay(date=d, count=c) for d, c in sorted(counts.items())]
