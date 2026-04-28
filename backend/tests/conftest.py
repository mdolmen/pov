"""Shared fixtures for all tests."""

import asyncio
import subprocess
from pathlib import Path

import aiosqlite
import pytest
import pytest_asyncio
from fastapi.testclient import TestClient

from pov.db import CREATE_ACTIVITY, CREATE_PROJECTS, CREATE_SELECTED_TASKS


@pytest.fixture()
def pov_dir(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """A temp ~/.local/share/pov/ with git repo, projects/ and learning/ dirs."""
    data_dir = tmp_path / "pov"
    projects_dir = data_dir / "projects"
    learning_dir = data_dir / "learning"
    projects_dir.mkdir(parents=True)
    learning_dir.mkdir(parents=True)
    (data_dir / "config.json").write_text('{"projects": [], "learning": {}}')

    subprocess.run(["git", "init", str(data_dir)], check=True, capture_output=True)
    subprocess.run(
        ["git", "-C", str(data_dir), "commit", "--allow-empty", "-m", "init"],
        check=True, capture_output=True,
    )

    # Patch storage constants so all modules use the temp dir.
    monkeypatch.setattr("pov.storage.POV_DIR", data_dir)
    monkeypatch.setattr("pov.storage.PROJECTS_DIR", projects_dir)
    monkeypatch.setattr("pov.storage.LEARNING_DIR", learning_dir)
    monkeypatch.setattr("pov.storage.CONFIG_FILE", data_dir / "config.json")
    monkeypatch.setattr("pov.routers.projects.POV_DIR", data_dir)
    monkeypatch.setattr("pov.routers.projects.PROJECTS_DIR", projects_dir)
    monkeypatch.setattr("pov.routers.projects.CONFIG_FILE", data_dir / "config.json")
    monkeypatch.setattr("pov.activity.POV_DIR", data_dir)

    return data_dir


@pytest_asyncio.fixture()
async def db(pov_dir: Path) -> aiosqlite.Connection:
    """In-memory SQLite connection with the full schema applied."""
    async with aiosqlite.connect(":memory:") as conn:
        await conn.execute("PRAGMA foreign_keys = ON")
        await conn.execute(CREATE_PROJECTS)
        await conn.execute(CREATE_SELECTED_TASKS)
        await conn.execute(CREATE_ACTIVITY)
        await conn.commit()
        conn.row_factory = aiosqlite.Row
        yield conn


@pytest.fixture()
def client(pov_dir: Path, db: aiosqlite.Connection) -> TestClient:
    """FastAPI test client with the DB dependency overridden."""
    from main import app
    from pov.db import get_db

    async def override_get_db():
        yield db

    app.dependency_overrides[get_db] = override_get_db
    with TestClient(app) as c:
        yield c
    app.dependency_overrides.clear()


@pytest.fixture()
def todo_file(tmp_path: Path) -> Path:
    """A sample TODO.md file."""
    f = tmp_path / "TODO.md"
    f.write_text(
        "# My Project\n"
        "\n"
        "- [ ] Task one\n"
        "- [x] Task two (done)\n"
        "- [ ] Task three\n"
    )
    return f
