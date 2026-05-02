"""Activity heatmap endpoint, sourced from git log on the pov data repo."""

import re
import subprocess
from collections import Counter
from datetime import datetime
from typing import Literal

import aiosqlite
from fastapi import APIRouter, Depends
from pydantic import BaseModel

from pov.db import get_db
from pov.storage import POV_DIR

router = APIRouter(prefix="/activity", tags=["activity"])

ProjectType = Literal["project", "learning"]

# Watcher and toggle commit messages: "activity: <project_id>.md"
ACTIVITY_RE = re.compile(r"^activity:\s+([0-9a-f-]+)\.md$")


class ActivityDay(BaseModel):
    date: str  # YYYY-MM-DD (local time)
    count: int


def _git_log_since(days: int) -> list[tuple[str, str]]:
    """Return [(iso_timestamp, subject)] for commits in the last N days.

    Empty list if the repo is missing or git fails.
    """
    result = subprocess.run(
        ["git", "-C", str(POV_DIR), "log", f"--since={days} days ago", "--pretty=format:%aI%x09%s"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0 or not result.stdout:
        return []
    pairs: list[tuple[str, str]] = []
    for line in result.stdout.splitlines():
        if "\t" in line:
            ts, subject = line.split("\t", 1)
            pairs.append((ts, subject))
    return pairs


async def _project_types(db: aiosqlite.Connection) -> dict[str, str]:
    cursor = await db.execute("SELECT id, type FROM projects")
    return {row["id"]: row["type"] for row in await cursor.fetchall()}


@router.get("", response_model=list[ActivityDay])
async def get_activity(
    type: ProjectType,
    days: int = 120,
    db: aiosqlite.Connection = Depends(get_db),
):
    """Return per-day activity counts for the last `days` days.

    Counts commits whose message is `activity: <project_id>.md` and
    whose project has the requested type. Days with zero activity are
    omitted; the frontend fills the calendar window itself.
    """
    types = await _project_types(db)
    counts: Counter[str] = Counter()
    for ts, subject in _git_log_since(days):
        m = ACTIVITY_RE.match(subject)
        if not m:
            continue
        project_id = m.group(1)
        if types.get(project_id) != type:
            continue
        try:
            dt = datetime.fromisoformat(ts)
        except ValueError:
            continue
        # Bucket by the commit's own local date (no tz conversion).
        counts[dt.date().isoformat()] += 1

    return [ActivityDay(date=d, count=c) for d, c in sorted(counts.items())]
