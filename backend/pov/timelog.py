"""Parsing of TIME.md — a markdown table of time spent per day.

Expected shape (the header separator row is skipped):

    | DATE       | TIME | TOPIC      |
    | ---------- | ---- | ---------- |
    | 2026-01-31 | 2,00 | Proba      |

TIME is a number of hours with either a comma or a dot as decimal separator,
rounded to the nearest 15 minutes.
"""

import sqlite3
from dataclasses import dataclass
from datetime import date
from pathlib import Path

STEP_MINUTES = 15


@dataclass(frozen=True)
class TimeRow:
    date: str  # YYYY-MM-DD
    minutes: int
    topic: str | None


class TimeLogParseError(ValueError):
    """Raised when a TIME.md row cannot be read."""


def _cells(line: str) -> list[str]:
    stripped = line.strip()
    if not stripped.startswith("|"):
        return []
    return [c.strip() for c in stripped.strip("|").split("|")]


def parse_time_log(path: Path) -> list[TimeRow]:
    """Parse a TIME.md file into time rows.

    Args:
        path: Path to the markdown file.

    Returns:
        One TimeRow per data row, in file order.

    Raises:
        TimeLogParseError: If a data row has an unreadable date or duration.
    """
    rows: list[TimeRow] = []
    for lineno, line in enumerate(path.read_text().splitlines(), start=1):
        cells = _cells(line)
        if len(cells) < 2:
            continue
        raw_date, raw_hours = cells[0], cells[1]
        if set(raw_date) <= set("- ") or raw_date.upper() == "DATE":
            continue
        try:
            day = date.fromisoformat(raw_date)
        except ValueError:
            raise TimeLogParseError(f"{path}:{lineno}: bad date {raw_date!r}") from None
        try:
            hours = float(raw_hours.replace(",", "."))
        except ValueError:
            raise TimeLogParseError(f"{path}:{lineno}: bad duration {raw_hours!r}") from None
        minutes = round(hours * 60 / STEP_MINUTES) * STEP_MINUTES
        if minutes <= 0:
            raise TimeLogParseError(f"{path}:{lineno}: non-positive duration {raw_hours!r}")
        topic = cells[2].strip() if len(cells) > 2 and cells[2].strip() else None
        rows.append(TimeRow(date=day.isoformat(), minutes=minutes, topic=topic))
    return rows


def insert_time_rows(db: sqlite3.Connection, project_id: str, rows: list[TimeRow]) -> int:
    """Insert time rows for a project. Returns the number of rows inserted."""
    db.executemany(
        "INSERT INTO time_entries (project_id, date, minutes, topic) VALUES (?, ?, ?, ?)",
        [(project_id, r.date, r.minutes, r.topic) for r in rows],
    )
    db.commit()
    return len(rows)
