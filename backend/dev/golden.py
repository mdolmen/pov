"""Record golden outputs of the Python backend for the Rust port to match.

Usage (from backend/):
    uv run python dev/golden.py

Writes one JSON file per input to src-tauri/core/tests/golden/{tasks,time}/. Each file embeds
its input, so the Rust tests don't depend on the current state of fixtures/. Inputs: the
tracked fixtures/*.md, plus edge cases defined below.
"""

import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

from pov.tasks import Task, parse_items, toggle_cascade
from pov.timelog import TimeLogParseError, parse_time_log

REPO_ROOT = Path(__file__).parent.parent.parent
OUT_DIR = REPO_ROOT / "src-tauri" / "core" / "tests" / "golden"

# The Python API's task count: unchecked checkbox lines, subtasks included.
OPEN_TASK_RE = re.compile(r"^\s*- \[ \]", re.MULTILINE)

EDGE_TASKS: dict[str, bytes] = {
    "crlf": b"# Title\r\n- [ ] one\r\n  - [x] sub\r\n- [x] two\r\n",
    "separators": (
        "- [ ] form\x0cfeed\n"
        "- [ ] vertical\x0btab\n"
        "- [ ] file\x1csep\n"
        "- [ ] line sep\n"
        "- [ ] next\x85line\n"
        "- [x] trailing\x1c\n"
        "- [ ] after\n"
    ).encode(),
    "duplicates": (
        "- [ ] same\n"
        "- [x] same\n"
        "- [ ] parent\n"
        "  - [ ] same\n"
        "  - [ ] child\n"
        "- [ ] parent\n"
        "  - [ ] child\n"
    ).encode(),
    "orphans": (
        "  - [ ] orphan before any task\n"
        "- [ ] task\n"
        "  - [ ] sub\n"
        "## Heading\n"
        "  - [ ] orphan after heading\n"
        "- [ ] other\n"
    ).encode(),
    "states": (
        "- [X] upper\n"
        "- [-] dash\n"
        "- [~] tilde\n"
        "- [ ]\n"
        "- [ ] \n"
        "-  [ ] two spaces\n"
        "* [ ] star\n"
        "- [ ] parent\n"
        "  - [X] upper sub\n"
        "  - [-] dash sub\n"
    ).encode(),
    "brackets": (
        "- [ ] see [a] and [ ] here\n"
        "- [ ] see [b] and [ ] here\n"
        "- [x] done [ ] inner\n"
        "- [ ] parent [ ] text\n"
        "  - [x] sub\n"
    ).encode(),
    "whitespace": (
        "- [ ] trailing spaces   \n"
        "- [ ] trailing tab\t\n"
        "- [ ] trailing nbsp\xa0\n"
        "- [ ] parent\n"
        "\t- [ ] tab sub\n"
        "    - [x] four spaces sub\n"
        "   \n"
        "#Not a heading\n"
        "####### seven hashes\n"
        "###   Spaced heading   \n"
    ).encode(),
    "cascade": (
        "- [ ] partial\n"
        "  - [x] a\n"
        "  - [ ] b\n"
        "- [ ] all done\n"
        "  - [x] a\n"
        "  - [x] b\n"
        "- [x] checked parent, open subs\n"
        "  - [ ] a\n"
        "- [x] single\n"
    ).encode(),
    "bom": "﻿# Title\n- [ ] first\n".encode(),
    "no-final-newline": b"# Title\n- [ ] last",
    "empty": b"",
    "latin1": "- [ ] caf\xe9\n".encode("latin-1"),
}

EDGE_TIME: dict[str, str] = {
    "basic": (
        "# Time\n"
        "\n"
        "| DATE       | TIME | TOPIC      |\n"
        "| ---------- | ---- | ---------- |\n"
        "| 2026-01-31 | 2,00 | Proba      |\n"
        "| 2026-02-01 | 1.5  | Algebra    |\n"
        "| 2026-02-02 | 0,25 |            |\n"
        "| 2026-02-03 | 1    |\n"
        "|| 2026-02-04 | 1e0 | Doubled pipes ||\n"
        "| 20260205 | +0,75 | Basic date format |\n"
        "not a table row\n"
    ),
    "ties": (
        "| 2026-01-01 | 0,625 | 2.5 steps |\n"
        "| 2026-01-02 | 1,125 | 4.5 steps |\n"
        "| 2026-01-03 | 0,375 | 1.5 steps |\n"
        "| 2026-01-04 | 0.13 | rounds up |\n"
    ),
    "rounds-to-zero": "| 2026-01-01 | 0,125 | 0.5 steps |\n",
    "bad-date": "| DATE | TIME |\n| 2026-13-01 | 1 |\n",
    "bad-duration": "| 2026-01-01 | 1 |\n| 2026-01-02 | 1h |\n",
    "alignment-row": "| DATE | TIME |\n| :--- | ---: |\n| 2026-01-01 | 1 |\n",
    "empty": "",
}


def _item_json(item: object) -> dict:
    if isinstance(item, Task):
        return {
            "kind": "task",
            "hash": item.hash,
            "text": item.text,
            "checked": item.checked,
            "line_number": item.line_number,
            "is_done": item.is_done,
            "subtasks": [
                {"hash": s.hash, "text": s.text, "checked": s.checked,
                 "line_number": s.line_number}
                for s in item.subtasks
            ],
        }
    return {"kind": "heading", "text": item.text, "level": item.level}


def _input_json(data: bytes) -> dict:
    try:
        return {"input": data.decode()}
    except UnicodeDecodeError:
        return {"input_bytes": list(data)}


def _count_tasks(path: Path) -> int:
    try:
        return len(OPEN_TASK_RE.findall(path.read_text()))
    except (OSError, UnicodeDecodeError):
        return 0


def _tasks_golden(data: bytes, work: Path) -> dict:
    path = work / "TODO.md"
    path.write_bytes(data)
    items = parse_items(path)
    count = _count_tasks(path)
    hashes = [
        h
        for item in items
        if isinstance(item, Task)
        for h in [item.hash, *(s.hash for s in item.subtasks)]
    ]
    toggles = {}
    for h in hashes:
        path.write_bytes(data)
        assert toggle_cascade(path, h)
        toggles[h] = path.read_bytes().decode()
    return {
        **_input_json(data),
        "items": [_item_json(i) for i in items],
        "count_tasks": count,
        "toggles": toggles,
    }


def _time_golden(text: str, work: Path) -> dict:
    path = work / "TIME.md"
    path.write_text(text)
    try:
        rows = parse_time_log(path)
    except TimeLogParseError as e:
        return {"input": text, "error_line": int(str(e).split(":")[1])}
    return {
        "input": text,
        "rows": [{"date": r.date, "minutes": r.minutes, "topic": r.topic} for r in rows],
    }


def _tracked_fixtures() -> list[Path]:
    out = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "ls-files", "fixtures/*.md"],
        check=True, capture_output=True, text=True,
    ).stdout
    return [REPO_ROOT / line for line in out.splitlines()]


def _write(kind: str, name: str, golden: dict) -> None:
    out = OUT_DIR / kind / f"{name}.json"
    out.write_text(json.dumps(golden, indent=1) + "\n")


def main() -> None:
    shutil.rmtree(OUT_DIR, ignore_errors=True)
    (OUT_DIR / "tasks").mkdir(parents=True)
    (OUT_DIR / "time").mkdir(parents=True)

    tasks_inputs = {f"fixture-{p.stem}": p.read_bytes() for p in _tracked_fixtures()}
    tasks_inputs |= EDGE_TASKS
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        for name, data in tasks_inputs.items():
            _write("tasks", name, _tasks_golden(data, work))
        for name, text in EDGE_TIME.items():
            _write("time", name, _time_golden(text, work))
    print(f"wrote {len(tasks_inputs)} tasks and {len(EDGE_TIME)} time goldens to {OUT_DIR}")


if __name__ == "__main__":
    main()
