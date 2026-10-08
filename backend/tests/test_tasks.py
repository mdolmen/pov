"""Tests for task parsing, toggle, and selection."""

from pathlib import Path

import pytest

from pov.tasks import parse_tasks, task_hash, toggle_line


# --- Parser tests ---

MIXED_CONTENT = """\
# Section one

Some plain text here.

- [ ] Task one
  - [ ] Subtask A
  - [x] Subtask B
- [x] Task two
- [ ] Task three

## Section two

- [ ] Task four
"""


def test_parser_finds_all_tasks():
    from pathlib import Path
    f = Path("/tmp/test_parse.md")
    f.write_text(MIXED_CONTENT)
    tasks = parse_tasks(f)
    assert len(tasks) == 4
    assert tasks[0].text == "Task one"
    assert tasks[1].text == "Task two"
    assert tasks[2].text == "Task three"
    assert tasks[3].text == "Task four"


def test_parser_finds_subtasks():
    f = Path("/tmp/test_subtasks.md")
    f.write_text(MIXED_CONTENT)
    tasks = parse_tasks(f)
    assert len(tasks[0].subtasks) == 2
    assert tasks[0].subtasks[0].text == "Subtask A"
    assert tasks[0].subtasks[0].checked is False
    assert tasks[0].subtasks[1].text == "Subtask B"
    assert tasks[0].subtasks[1].checked is True


def test_parser_checked_state():
    f = Path("/tmp/test_checked.md")
    f.write_text(MIXED_CONTENT)
    tasks = parse_tasks(f)
    assert tasks[0].checked is False
    assert tasks[1].checked is True


def test_parser_line_numbers():
    f = Path("/tmp/test_linenum.md")
    f.write_text(MIXED_CONTENT)
    tasks = parse_tasks(f)
    assert tasks[0].line_number == 4
    assert tasks[1].line_number == 7


def test_parser_handles_empty_file(tmp_path: Path):
    f = tmp_path / "empty.md"
    f.write_text("")
    assert parse_tasks(f) == []


def test_parser_handles_missing_file(tmp_path: Path):
    assert parse_tasks(tmp_path / "nonexistent.md") == []


# --- is_done logic ---

def test_is_done_no_subtasks_unchecked(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [ ] Task\n")
    assert parse_tasks(f)[0].is_done is False


def test_is_done_no_subtasks_checked(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [x] Task\n")
    assert parse_tasks(f)[0].is_done is True


def test_is_done_all_subtasks_checked(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [ ] Task\n  - [x] Sub A\n  - [x] Sub B\n")
    assert parse_tasks(f)[0].is_done is True


def test_is_done_partial_subtasks(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [ ] Task\n  - [x] Sub A\n  - [ ] Sub B\n")
    assert parse_tasks(f)[0].is_done is False


# --- Hash stability ---

def test_hash_stable_across_check_state():
    unchecked = "- [ ] My task"
    checked = "- [x] My task"
    assert task_hash(unchecked) == task_hash(checked)


def test_hash_unique_for_duplicate_text(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [ ] Same task\n- [ ] Other\n- [ ] Same task\n")
    tasks = parse_tasks(f)
    assert tasks[0].text == "Same task"
    assert tasks[2].text == "Same task"
    assert tasks[0].hash != tasks[2].hash


def test_toggle_disambiguates_duplicate_text(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [ ] Same task\n- [ ] Same task\n")
    tasks = parse_tasks(f)
    toggle_line(f, tasks[1].hash)
    assert f.read_text() == "- [ ] Same task\n- [x] Same task\n"


def test_hash_stable_across_line_shift(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [ ] Task one\n- [ ] Task two\n")
    tasks_before = parse_tasks(f)
    hash_two_before = tasks_before[1].hash

    # Insert a line above Task two.
    f.write_text("- [ ] Task one\n- [ ] Inserted\n- [ ] Task two\n")
    tasks_after = parse_tasks(f)
    hash_two_after = tasks_after[2].hash

    assert hash_two_before == hash_two_after


# --- Toggle ---

def test_toggle_unchecked_to_checked(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [ ] Task one\n- [ ] Task two\n")
    h = task_hash("- [ ] Task one")
    assert toggle_line(f, h) is True
    assert f.read_text() == "- [x] Task one\n- [ ] Task two\n"


def test_toggle_checked_to_unchecked(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [x] Task one\n")
    h = task_hash("- [x] Task one")
    assert toggle_line(f, h) is True
    assert f.read_text() == "- [ ] Task one\n"


def test_toggle_not_found(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [ ] Task one\n")
    assert toggle_line(f, "nonexistenthash") is False


def test_toggle_only_affects_matching_line(tmp_path: Path):
    f = tmp_path / "t.md"
    f.write_text("- [ ] Task one\n- [ ] Task two\n- [ ] Task three\n")
    h = task_hash("- [ ] Task two")
    toggle_line(f, h)
    lines = f.read_text().splitlines()
    assert lines[0] == "- [ ] Task one"
    assert lines[1] == "- [x] Task two"
    assert lines[2] == "- [ ] Task three"
