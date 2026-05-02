"""Tests for the /activity endpoint (heatmap data sourced from git log)."""

import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path

from fastapi.testclient import TestClient


def _commit(repo: Path, message: str, when: datetime) -> None:
    """Create an empty commit with a fixed author/committer date."""
    iso = when.isoformat()
    env = {
        "GIT_AUTHOR_DATE": iso,
        "GIT_COMMITTER_DATE": iso,
        "GIT_AUTHOR_NAME": "test",
        "GIT_AUTHOR_EMAIL": "test@example.com",
        "GIT_COMMITTER_NAME": "test",
        "GIT_COMMITTER_EMAIL": "test@example.com",
    }
    subprocess.run(
        ["git", "-C", str(repo), "commit", "--allow-empty", "-m", message],
        check=True, capture_output=True, env={**__import__("os").environ, **env},
    )


def _seed_project(client: TestClient, name: str, file_path: Path, type: str) -> str:
    file_path.write_text("- [ ] x\n")
    r = client.post(
        "/projects",
        json={"name": name, "file_path": str(file_path), "type": type},
    )
    return r.json()["id"]


def test_activity_filters_by_type(client: TestClient, pov_dir: Path, tmp_path: Path):
    proj_id = _seed_project(client, "P", tmp_path / "p.md", "project")
    learn_id = _seed_project(client, "L", tmp_path / "l.md", "learning")

    today = datetime.now(tz=timezone.utc)
    _commit(pov_dir, f"activity: {proj_id}.md", today)
    _commit(pov_dir, f"activity: {learn_id}.md", today)
    _commit(pov_dir, f"activity: {learn_id}.md", today - timedelta(days=2))

    r = client.get("/activity?type=project")
    assert r.status_code == 200
    data = r.json()
    assert sum(d["count"] for d in data) == 1

    r = client.get("/activity?type=learning")
    data = r.json()
    assert sum(d["count"] for d in data) == 2


def test_activity_buckets_by_day(client: TestClient, pov_dir: Path, tmp_path: Path):
    pid = _seed_project(client, "P", tmp_path / "p.md", "project")
    base = datetime.now(tz=timezone.utc).replace(hour=10, minute=0, second=0, microsecond=0)

    _commit(pov_dir, f"activity: {pid}.md", base)
    _commit(pov_dir, f"activity: {pid}.md", base + timedelta(hours=2))
    _commit(pov_dir, f"activity: {pid}.md", base - timedelta(days=1))

    data = client.get("/activity?type=project").json()
    by_date = {d["date"]: d["count"] for d in data}
    assert by_date[base.date().isoformat()] == 2
    assert by_date[(base - timedelta(days=1)).date().isoformat()] == 1


def test_activity_ignores_non_activity_commits(client: TestClient, pov_dir: Path, tmp_path: Path):
    pid = _seed_project(client, "P", tmp_path / "p.md", "project")
    today = datetime.now(tz=timezone.utc)
    _commit(pov_dir, f"add: {pid}", today)
    _commit(pov_dir, "init: pov data repo", today)
    _commit(pov_dir, f"activity: {pid}.md", today)

    data = client.get("/activity?type=project").json()
    assert sum(d["count"] for d in data) == 1


def test_activity_respects_days_window(client: TestClient, pov_dir: Path, tmp_path: Path):
    pid = _seed_project(client, "P", tmp_path / "p.md", "project")
    today = datetime.now(tz=timezone.utc)

    # Create older commit first so HEAD remains the recent one
    # (mirrors real usage where commits are always made with "now").
    _commit(pov_dir, f"activity: {pid}.md", today - timedelta(days=200))
    _commit(pov_dir, f"activity: {pid}.md", today)

    data = client.get("/activity?type=project&days=120").json()
    assert sum(d["count"] for d in data) == 1
    data_all = client.get("/activity?type=project&days=365").json()
    assert sum(d["count"] for d in data_all) == 2


def test_activity_handles_empty_repo(client: TestClient):
    r = client.get("/activity?type=project")
    assert r.status_code == 200
    assert r.json() == []
