"""Tests for /projects endpoints."""

import json
import subprocess
from pathlib import Path
from unittest.mock import patch

import pytest
from fastapi.testclient import TestClient


def test_list_projects_empty(client: TestClient):
    r = client.get("/projects")
    assert r.status_code == 200
    assert r.json() == []


def test_create_project_success(client: TestClient, todo_file: Path, pov_dir: Path):
    r = client.post("/projects", json={"name": "My Project", "file_path": str(todo_file)})
    assert r.status_code == 201
    data = r.json()
    assert data["name"] == "My Project"
    assert data["status"] == "open"
    assert data["task_count"] == 3
    assert data["selected_count"] == 0
    assert data["has_hardlink"] is True

    # Hardlink exists in projects dir.
    project_id = data["id"]
    assert (pov_dir / "projects" / f"{project_id}.md").exists()

    # config.json updated.
    config = json.loads((pov_dir / "config.json").read_text())
    assert any(p["id"] == project_id for p in config["projects"])

    # Git commit made.
    log = subprocess.run(
        ["git", "-C", str(pov_dir), "log", "--oneline"],
        capture_output=True, text=True,
    )
    assert "add: My Project" in log.stdout


def test_create_project_file_not_found(client: TestClient):
    r = client.post("/projects", json={"name": "X", "file_path": "/nonexistent/TODO.md"})
    assert r.status_code == 422


def test_create_project_cross_device_fallback(client: TestClient, todo_file: Path):
    import errno
    with patch("pov.routers.projects._create_hardlink", side_effect=OSError(18, "cross-device")):
        # Should fail because _create_hardlink raising means no hardlink was made.
        # Actually the router catches EXDEV inside _create_hardlink — let's test
        # _create_hardlink directly returns False.
        pass

    # Patch os.link to raise EXDEV.
    import os
    original_link = os.link

    def cross_device_link(src, dst):
        raise OSError(18, "Invalid cross-device link")

    with patch("pov.projects.os.link", cross_device_link):
        r = client.post("/projects", json={"name": "X", "file_path": str(todo_file)})
    assert r.status_code == 201
    assert r.json()["has_hardlink"] is False


def test_list_projects_returns_metadata(client: TestClient, todo_file: Path):
    client.post("/projects", json={"name": "P", "file_path": str(todo_file)})
    r = client.get("/projects")
    assert r.status_code == 200
    projects = r.json()
    assert len(projects) == 1
    assert projects[0]["task_count"] == 3
    assert projects[0]["selected_count"] == 0
    assert projects[0]["activity"] in ("this_week", "this_month", "older", "none")


def test_delete_project(client: TestClient, todo_file: Path, pov_dir: Path):
    create = client.post("/projects", json={"name": "P", "file_path": str(todo_file)})
    project_id = create.json()["id"]
    hardlink = pov_dir / "projects" / f"{project_id}.md"
    assert hardlink.exists()

    r = client.delete(f"/projects/{project_id}")
    assert r.status_code == 204

    # Hardlink gone, original intact.
    assert not hardlink.exists()
    assert todo_file.exists()

    # Removed from config.
    config = json.loads((pov_dir / "config.json").read_text())
    assert not any(p["id"] == project_id for p in config["projects"])

    # Gone from list.
    assert client.get("/projects").json() == []


def test_delete_project_not_found(client: TestClient):
    r = client.delete("/projects/nonexistent-id")
    assert r.status_code == 404


def test_update_project_name(client: TestClient, todo_file: Path):
    create = client.post("/projects", json={"name": "Old", "file_path": str(todo_file)})
    project_id = create.json()["id"]

    r = client.patch(f"/projects/{project_id}", json={"name": "New"})
    assert r.status_code == 200
    assert r.json()["name"] == "New"


def test_update_project_status(client: TestClient, todo_file: Path):
    create = client.post("/projects", json={"name": "P", "file_path": str(todo_file)})
    project_id = create.json()["id"]

    r = client.patch(f"/projects/{project_id}", json={"status": "archived", "sub_status": "done"})
    assert r.status_code == 200
    data = r.json()
    assert data["status"] == "archived"
    assert data["sub_status"] == "done"


def test_update_project_not_found(client: TestClient):
    r = client.patch("/projects/nonexistent", json={"name": "X"})
    assert r.status_code == 404


def test_watchdog_commits_on_file_change(pov_dir: Path, tmp_path: Path):
    """Watchdog handler calls git add + commit when a .md file changes."""
    from unittest.mock import MagicMock, call, patch

    md = pov_dir / "projects" / "test.md"
    md.write_text("- [ ] task\n")

    from pov.watcher import _Handler
    from watchdog.events import FileModifiedEvent

    handler = _Handler()
    event = FileModifiedEvent(str(md))

    with patch("pov.watcher.subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0)
        handler.on_modified(event)

    calls = mock_run.call_args_list
    assert any("add" in str(c) for c in calls)
    assert any("commit" in str(c) for c in calls)


def test_watchdog_ignores_non_md_files(pov_dir: Path):
    from unittest.mock import patch

    from pov.watcher import _Handler
    from watchdog.events import FileModifiedEvent

    handler = _Handler()
    event = FileModifiedEvent(str(pov_dir / "projects" / "something.txt"))

    with patch("pov.watcher.subprocess.run") as mock_run:
        handler.on_modified(event)

    mock_run.assert_not_called()
