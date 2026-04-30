"""TODO.md parser and task hash logic."""

import hashlib
import re
from pathlib import Path

CHECKBOX_RE = re.compile(r"^(\s*)- \[(.)\] (.+)$")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+)$")


def _normalize(line: str) -> str:
    """Replace checkbox state with [ ] for stable hashing."""
    return re.sub(r"\[.\]", "[ ]", line.rstrip())


def task_hash(line: str) -> str:
    normalized = _normalize(line)
    return hashlib.sha256(normalized.encode()).hexdigest()[:16]


def _set_checked(line: str, checked: bool) -> str:
    """Set a checkbox line to a specific checked state."""
    if checked:
        return re.sub(r"\[ \]", "[x]", line, count=1)
    return re.sub(r"\[[xX]\]", "[ ]", line, count=1)


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


class Heading:
    def __init__(self, text: str, level: int) -> None:
        self.text = text
        self.level = level


def parse_items(file_path: Path) -> list[Heading | Task]:
    """Return headings and tasks in document order."""
    try:
        lines = file_path.read_text().splitlines()
    except OSError:
        return []

    items: list[Heading | Task] = []
    current: Task | None = None

    for i, line in enumerate(lines):
        h = HEADING_RE.match(line)
        if h:
            current = None
            items.append(Heading(text=h.group(2).strip(), level=len(h.group(1))))
            continue

        m = CHECKBOX_RE.match(line)
        if not m:
            continue
        indent, state, text = m.group(1), m.group(2), m.group(3)
        checked = state.lower() == "x"

        if not indent:
            current = Task(text=text, checked=checked, line_number=i, raw_line=line)
            items.append(current)
        elif current is not None:
            current.subtasks.append(
                Subtask(text=text, checked=checked, line_number=i, raw_line=line)
            )

    return items


def parse_tasks(file_path: Path) -> list[Task]:
    return [i for i in parse_items(file_path) if isinstance(i, Task)]


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


def toggle_cascade(file_path: Path, target_hash: str) -> bool:
    """Toggle a task with cascade and parent-sync behavior.

    - Parent with subtasks: sets all subtasks (and parent) to !is_done.
    - Parent without subtasks: simple toggle.
    - Subtask: toggles subtask, then syncs parent checkbox to match is_done.
    Returns True if the target was found.
    """
    tasks = parse_tasks(file_path)
    task = next((t for t in tasks if t.hash == target_hash), None)
    parent: Task | None = None
    if task is None:
        parent = next(
            (t for t in tasks if any(s.hash == target_hash for s in t.subtasks)),
            None,
        )
    if task is None and parent is None:
        return False

    try:
        lines = file_path.read_text().splitlines(keepends=True)
    except OSError:
        return False

    if task is not None and task.subtasks:
        new_state = not task.is_done
        targets = {task.hash: new_state} | {s.hash: new_state for s in task.subtasks}
        for i, line in enumerate(lines):
            if CHECKBOX_RE.match(line):
                h = task_hash(line)
                if h in targets:
                    lines[i] = _set_checked(line, targets[h])

    elif task is not None:
        for i, line in enumerate(lines):
            if CHECKBOX_RE.match(line) and task_hash(line) == target_hash:
                lines[i] = _set_checked(line, not task.checked)
                break

    else:
        assert parent is not None
        subtask = next(s for s in parent.subtasks if s.hash == target_hash)
        new_sub = not subtask.checked
        new_parent = all(
            (new_sub if s.hash == target_hash else s.checked) for s in parent.subtasks
        )
        found_sub = found_parent = False
        for i, line in enumerate(lines):
            if not CHECKBOX_RE.match(line):
                continue
            h = task_hash(line)
            if h == target_hash and not found_sub:
                lines[i] = _set_checked(line, new_sub)
                found_sub = True
            elif h == parent.hash and not found_parent:
                lines[i] = _set_checked(line, new_parent)
                found_parent = True
            if found_sub and found_parent:
                break

    file_path.write_text("".join(lines))
    return True
