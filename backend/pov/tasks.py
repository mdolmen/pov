"""TODO.md parser and task hash logic."""

import hashlib
import re
from pathlib import Path

CHECKBOX_RE = re.compile(r"^(\s*)- \[(.)\] (.+)$")


def _normalize(line: str) -> str:
    """Replace checkbox state with [ ] for stable hashing."""
    return re.sub(r"\[.\]", "[ ]", line.rstrip())


def task_hash(line: str) -> str:
    normalized = _normalize(line)
    return hashlib.sha256(normalized.encode()).hexdigest()[:16]


class Subtask:
    def __init__(self, text: str, checked: bool, line_number: int, raw_line: str) -> None:
        self.text = text
        self.checked = checked
        self.line_number = line_number
        self.hash = task_hash(raw_line)


class Task:
    def __init__(self, text: str, checked: bool, line_number: int, raw_line: str) -> None:
        self.text = text
        self.checked = checked
        self.line_number = line_number
        self.hash = task_hash(raw_line)
        self.subtasks: list[Subtask] = []

    @property
    def is_done(self) -> bool:
        if self.subtasks:
            return all(s.checked for s in self.subtasks)
        return self.checked


def parse_tasks(file_path: Path) -> list[Task]:
    try:
        lines = file_path.read_text().splitlines()
    except OSError:
        return []

    tasks: list[Task] = []
    current: Task | None = None

    for i, line in enumerate(lines):
        m = CHECKBOX_RE.match(line)
        if not m:
            continue
        indent, state, text = m.group(1), m.group(2), m.group(3)
        checked = state.lower() == "x"

        if not indent:
            current = Task(text=text, checked=checked, line_number=i, raw_line=line)
            tasks.append(current)
        elif current is not None:
            current.subtasks.append(
                Subtask(text=text, checked=checked, line_number=i, raw_line=line)
            )

    return tasks


def toggle_line(file_path: Path, target_hash: str) -> bool:
    """Toggle the checkbox of the line matching target_hash. Returns True if found."""
    try:
        lines = file_path.read_text().splitlines(keepends=True)
    except OSError:
        return False

    for i, line in enumerate(lines):
        if not CHECKBOX_RE.match(line):
            continue
        if task_hash(line) != target_hash:
            continue
        if "[ ]" in line:
            lines[i] = line.replace("[ ]", "[x]", 1)
        else:
            lines[i] = re.sub(r"\[[xX]\]", "[ ]", line, count=1)
        file_path.write_text("".join(lines))
        return True

    return False
