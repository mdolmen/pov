"""Shared fixtures for all tests."""

import subprocess
from pathlib import Path

import pytest


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
    monkeypatch.setattr("pov.projects.POV_DIR", data_dir)
    monkeypatch.setattr("pov.projects.PROJECTS_DIR", projects_dir)
    monkeypatch.setattr("pov.projects.LEARNING_DIR", learning_dir)
    monkeypatch.setattr("pov.projects.CONFIG_FILE", data_dir / "config.json")

    return data_dir
