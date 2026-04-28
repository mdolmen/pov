"""File watcher — auto-commits changes in the pov data dir to git."""

import subprocess
from pathlib import Path

from watchdog.events import FileModifiedEvent, FileSystemEventHandler
from watchdog.observers import Observer

from pov.storage import LEARNING_DIR, POV_DIR, PROJECTS_DIR


class _Handler(FileSystemEventHandler):
    def on_modified(self, event: FileModifiedEvent) -> None:
        if event.is_directory:
            return
        path = Path(event.src_path)
        if path.suffix != ".md":
            return
        subprocess.run(
            ["git", "-C", str(POV_DIR), "add", str(path)],
            capture_output=True,
        )
        subprocess.run(
            ["git", "-C", str(POV_DIR), "commit", "-m", f"activity: {path.name}"],
            capture_output=True,
        )


def start_watcher() -> Observer:
    """Start watching projects/ and learning/ directories. Returns the observer."""
    handler = _Handler()
    observer = Observer()
    observer.schedule(handler, str(PROJECTS_DIR), recursive=False)
    observer.schedule(handler, str(LEARNING_DIR), recursive=False)
    observer.start()
    return observer
