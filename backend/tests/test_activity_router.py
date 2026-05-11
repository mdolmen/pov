"""Tests for GET /activity — heatmap data sourced from project git repos."""

import os
import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path

from fastapi.testclient import TestClient


def _make_project_repo(base: Path, name: str) -> Path:
    """Create a bare git repo at base/name (no commits) and return its path."""
    repo = base / name
    repo.mkdir()
    subprocess.run(["git", "init", str(repo)], check=True, capture_output=True)
    return repo


def _commit(repo: Path, when: datetime) -> None:
    """Add an empty commit at a fixed date."""
    iso = when.isoformat()
    env = {
        **os.environ,
        "GIT_AUTHOR_DATE": iso, "GIT_COMMITTER_DATE": iso,
        "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t",
        "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t",
    }
    subprocess.run(
        ["git", "-C", str(repo), "commit", "--allow-empty", "-m", "work"],
        check=True, capture_output=True, env=env,
    )


def _add_project(client: TestClient, repo: Path, name: str, type: str) -> str:
    todo = repo / "TODO.md"
    todo.write_text("- [ ] x\n")
    r = client.post("/projects", json={"name": name, "file_path": str(todo), "type": type})
    assert r.status_code == 201, r.text
    return r.json()["id"]


def test_activity_filters_by_type(client: TestClient, tmp_path: Path):
    proj_repo = _make_project_repo(tmp_path, "proj")
    learn_repo = _make_project_repo(tmp_path, "learn")
    _add_project(client, proj_repo, "P", "project")
    _add_project(client, learn_repo, "L", "learning")

    today = datetime.now(tz=timezone.utc)
    _commit(proj_repo, today)
    _commit(learn_repo, today)
    _commit(learn_repo, today - timedelta(days=2))

    r = client.get("/activity?type=project")
    assert r.status_code == 200
    assert sum(d["count"] for d in r.json()) == 1

    assert sum(d["count"] for d in client.get("/activity?type=learning").json()) == 2


def test_activity_buckets_by_day(client: TestClient, tmp_path: Path):
    repo = _make_project_repo(tmp_path, "proj")
    _add_project(client, repo, "P", "project")

    base = datetime.now(tz=timezone.utc).replace(hour=10, minute=0, second=0, microsecond=0)
    _commit(repo, base)
    _commit(repo, base + timedelta(hours=2))
    _commit(repo, base - timedelta(days=1))

    by_date = {d["date"]: d["count"] for d in client.get("/activity?type=project").json()}
    assert by_date[base.date().isoformat()] == 2
    assert by_date[(base - timedelta(days=1)).date().isoformat()] == 1


def test_activity_respects_days_window(client: TestClient, tmp_path: Path):
    repo = _make_project_repo(tmp_path, "proj")
    _add_project(client, repo, "P", "project")

    today = datetime.now(tz=timezone.utc)
    _commit(repo, today - timedelta(days=200))
    _commit(repo, today)

    assert sum(d["count"] for d in client.get("/activity?type=project&days=120").json()) == 1
    assert sum(d["count"] for d in client.get("/activity?type=project&days=365").json()) == 2


def test_activity_deduplicates_shared_repo(client: TestClient, tmp_path: Path):
    """Two projects in the same repo should count commits once, not twice."""
    repo = _make_project_repo(tmp_path, "mono")
    sub = repo / "sub"
    sub.mkdir()

    todo1 = repo / "TODO.md"
    todo1.write_text("- [ ] x\n")
    todo2 = sub / "TODO.md"
    todo2.write_text("- [ ] y\n")

    client.post("/projects", json={"name": "A", "file_path": str(todo1), "type": "project"})
    client.post("/projects", json={"name": "B", "file_path": str(todo2), "type": "project"})

    _commit(repo, datetime.now(tz=timezone.utc))

    assert sum(d["count"] for d in client.get("/activity?type=project").json()) == 1


def test_activity_handles_no_projects(client: TestClient):
    r = client.get("/activity?type=project")
    assert r.status_code == 200
    assert r.json() == []


def test_activity_handles_project_outside_git(client: TestClient, tmp_path: Path):
    """A project file not in any git repo contributes nothing."""
    todo = tmp_path / "TODO.md"
    todo.write_text("- [ ] x\n")
    client.post("/projects", json={"name": "P", "file_path": str(todo), "type": "project"})

    r = client.get("/activity?type=project")
    assert r.status_code == 200
    assert r.json() == []
