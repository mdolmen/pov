"""Manages ~/.local/share/pov/ — folder structure, hardlinks, git repo."""

import os
import subprocess
from pathlib import Path

_dir_name = "pov-dev" if os.environ.get("POV_ENV") == "dev" else "pov"
POV_DIR = Path.home() / ".local" / "share" / _dir_name
PROJECTS_DIR = POV_DIR / "projects"
LEARNING_DIR = POV_DIR / "learning"
CONFIG_FILE = POV_DIR / "config.json"


async def init_storage() -> None:
    """Create the app data directory and git repo on first launch."""
    PROJECTS_DIR.mkdir(parents=True, exist_ok=True)
    LEARNING_DIR.mkdir(parents=True, exist_ok=True)

    if not CONFIG_FILE.exists():
        CONFIG_FILE.write_text('{"projects": [], "learning": {}}')

    if not (POV_DIR / ".git").exists():
        subprocess.run(["git", "init", str(POV_DIR)], check=True, capture_output=True)
        subprocess.run(
            ["git", "-C", str(POV_DIR), "commit", "--allow-empty", "-m", "init: pov data repo"],
            check=True,
            capture_output=True,
        )
