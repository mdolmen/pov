"""Tests for activity classification."""

import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest.mock import patch

import pytest

from pov.activity import _classify, get_activity


def test_classify_this_week():
    dt = datetime.now(tz=timezone.utc) - timedelta(days=3)
    assert _classify(dt) == "this_week"


def test_classify_this_month():
    dt = datetime.now(tz=timezone.utc) - timedelta(days=15)
    assert _classify(dt) == "this_month"


def test_classify_older():
    dt = datetime.now(tz=timezone.utc) - timedelta(days=60)
    assert _classify(dt) == "older"


def test_classify_none():
    assert _classify(None) == "none"


def test_get_activity_uses_git_when_hardlink(pov_dir: Path, tmp_path: Path):
    md = tmp_path / "test.md"
    md.write_text("- [ ] task\n")
    # Hardlink into pov_dir/projects/ and commit it.
    dst = pov_dir / "projects" / "test.md"
    dst.hardlink_to(md)
    subprocess.run(["git", "-C", str(pov_dir), "add", str(dst)], check=True, capture_output=True)
    subprocess.run(
        ["git", "-C", str(pov_dir), "commit", "-m", "add test"],
        check=True, capture_output=True,
    )
    result = get_activity(dst, has_hardlink=True)
    assert result == "this_week"


def test_get_activity_falls_back_to_mtime_when_no_hardlink(tmp_path: Path):
    md = tmp_path / "test.md"
    md.write_text("- [ ] task\n")
    result = get_activity(md, has_hardlink=False)
    # File was just created, so mtime is recent.
    assert result == "this_week"


def test_get_activity_falls_back_to_mtime_when_git_returns_nothing(
    pov_dir: Path, tmp_path: Path
):
    md = tmp_path / "untracked.md"
    md.write_text("- [ ] task\n")
    # has_hardlink=True but file is not in git → git log returns nothing → mtime fallback.
    result = get_activity(md, has_hardlink=True)
    assert result == "this_week"
