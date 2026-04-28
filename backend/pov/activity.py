"""Activity classification from git log or mtime fallback."""

import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path

from pov.storage import POV_DIR

ActivityLevel = str  # "this_week" | "this_month" | "older" | "none"


def _classify(dt: datetime | None) -> ActivityLevel:
    if dt is None:
        return "none"
    now = datetime.now(tz=timezone.utc)
    if dt >= now - timedelta(days=7):
        return "this_week"
    if dt >= now - timedelta(days=30):
        return "this_month"
    return "older"


def _last_commit_date(file_path: Path) -> datetime | None:
    """Return the UTC datetime of the last git commit touching file_path, or None."""
    result = subprocess.run(
        ["git", "-C", str(POV_DIR), "log", "-1", "--format=%aI", "--", str(file_path)],
        capture_output=True,
        text=True,
    )
    line = result.stdout.strip()
    if not line:
        return None
    try:
        return datetime.fromisoformat(line)
    except ValueError:
        return None


def _mtime(file_path: Path) -> datetime | None:
    try:
        ts = file_path.stat().st_mtime
        return datetime.fromtimestamp(ts, tz=timezone.utc)
    except OSError:
        return None


def get_activity(file_path: Path, has_hardlink: bool) -> ActivityLevel:
    """Return activity level for a project file.

    Uses git log when a hardlink exists in the pov repo; falls back to mtime.
    """
    if has_hardlink:
        dt = _last_commit_date(file_path)
        if dt is not None:
            return _classify(dt)
    return _classify(_mtime(file_path))
