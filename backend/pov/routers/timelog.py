"""Time tracking endpoints — manually recorded time spent per project.

Unlike activity, this data has no git source of truth: it lives in SQLite.
"""

from datetime import date as date_type, timedelta

import aiosqlite
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, Field, field_validator

from pov.db import get_db

router = APIRouter(tags=["time"])

STEP_MINUTES = 15


class TimeDay(BaseModel):
    date: str  # YYYY-MM-DD
    minutes: int


class TimeEntryCreate(BaseModel):
    minutes: int = Field(gt=0)
    date: date_type = Field(default_factory=date_type.today)
    topic: str | None = None

    @field_validator("minutes")
    @classmethod
    def _multiple_of_step(cls, v: int) -> int:
        if v % STEP_MINUTES != 0:
            raise ValueError(f"minutes must be a multiple of {STEP_MINUTES}")
        return v

    @field_validator("topic")
    @classmethod
    def _strip_topic(cls, v: str | None) -> str | None:
        if v is None:
            return None
        stripped = v.strip()
        return stripped or None


class TimeEntry(BaseModel):
    id: int
    date: str
    minutes: int
    topic: str | None


async def _assert_project_exists(project_id: str, db: aiosqlite.Connection) -> None:
    cursor = await db.execute("SELECT 1 FROM projects WHERE id = ?", (project_id,))
    if await cursor.fetchone() is None:
        raise HTTPException(status_code=404, detail="project not found")


@router.get("/projects/{project_id}/time", response_model=list[TimeDay])
async def list_time(
    project_id: str, days: int = 120, db: aiosqlite.Connection = Depends(get_db)
):
    """Return per-day minute totals for the last `days` days.

    Days with no recorded time are omitted.
    """
    await _assert_project_exists(project_id, db)
    since = (date_type.today() - timedelta(days=days)).isoformat()
    cursor = await db.execute(
        "SELECT date, SUM(minutes) AS minutes FROM time_entries "
        "WHERE project_id = ? AND date >= ? GROUP BY date ORDER BY date",
        (project_id, since),
    )
    return [TimeDay(date=row["date"], minutes=row["minutes"]) for row in await cursor.fetchall()]


@router.post("/projects/{project_id}/time", response_model=TimeEntry, status_code=201)
async def add_time(
    project_id: str, body: TimeEntryCreate, db: aiosqlite.Connection = Depends(get_db)
):
    """Record time spent on a project."""
    await _assert_project_exists(project_id, db)
    iso_date = body.date.isoformat()
    cursor = await db.execute(
        "INSERT INTO time_entries (project_id, date, minutes, topic) VALUES (?, ?, ?, ?)",
        (project_id, iso_date, body.minutes, body.topic),
    )
    await db.commit()
    return TimeEntry(
        id=cursor.lastrowid, date=iso_date, minutes=body.minutes, topic=body.topic
    )


@router.get("/projects/{project_id}/time/topics", response_model=list[str])
async def list_topics(project_id: str, db: aiosqlite.Connection = Depends(get_db)):
    """Return topics used on this project, most recently used first."""
    await _assert_project_exists(project_id, db)
    cursor = await db.execute(
        "SELECT topic FROM time_entries WHERE project_id = ? AND topic IS NOT NULL "
        "GROUP BY topic ORDER BY MAX(date) DESC, MAX(id) DESC",
        (project_id,),
    )
    return [row["topic"] for row in await cursor.fetchall()]
