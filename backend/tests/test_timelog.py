"""Tests for TIME.md parsing and the `pov import-time` command."""

import sqlite3
from pathlib import Path

import pytest

from pov.cli import main
from pov.timelog import TimeLogParseError, parse_time_log

TABLE = """\
| DATE       | TIME | TOPIC          |
| ---------- | ---- | -------------- |
| 2026-01-31 | 2,00 | Proba          |
| 2026-01-31 | 1,50 | Statistics     |
| 2026-02-01 | 0,25 |                |
"""


@pytest.fixture()
def cli_db(pov_dir: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    db_path = pov_dir / "pov.db"
    monkeypatch.setattr("pov.db.DB_PATH", db_path)
    monkeypatch.setattr("pov.cli.DB_PATH", db_path)
    return db_path


def test_parse_converts_hours_to_minutes(tmp_path: Path):
    f = tmp_path / "TIME.md"
    f.write_text(TABLE)
    rows = parse_time_log(f)
    assert [(r.date, r.minutes, r.topic) for r in rows] == [
        ("2026-01-31", 120, "Proba"),
        ("2026-01-31", 90, "Statistics"),
        ("2026-02-01", 15, None),
    ]


def test_parse_accepts_dot_separator(tmp_path: Path):
    f = tmp_path / "TIME.md"
    f.write_text("| 2026-01-31 | 1.75 | Proba |\n")
    assert parse_time_log(f)[0].minutes == 105


def test_parse_rounds_to_15_minutes(tmp_path: Path):
    f = tmp_path / "TIME.md"
    f.write_text("| 2026-01-31 | 1,10 | Proba |\n")
    assert parse_time_log(f)[0].minutes == 60


def test_parse_rejects_bad_date(tmp_path: Path):
    f = tmp_path / "TIME.md"
    f.write_text("| 31/01/2026 | 1,00 | Proba |\n")
    with pytest.raises(TimeLogParseError, match="bad date"):
        parse_time_log(f)


def test_parse_rejects_bad_duration(tmp_path: Path):
    f = tmp_path / "TIME.md"
    f.write_text("| 2026-01-31 | two | Proba |\n")
    with pytest.raises(TimeLogParseError, match="bad duration"):
        parse_time_log(f)


def _seed_project(tmp_path: Path, name: str = "Maths") -> None:
    src = tmp_path / f"{name}.md"
    src.write_text("- [ ] x\n")
    assert main(["add", str(src), "--name", name, "--type", "learning"]) == 0


def test_import_time(cli_db: Path, tmp_path: Path, capsys: pytest.CaptureFixture):
    _seed_project(tmp_path)
    capsys.readouterr()
    time_file = tmp_path / "TIME.md"
    time_file.write_text(TABLE)

    assert main(["import-time", "Maths", str(time_file)]) == 0
    assert "imported: 3 entries" in capsys.readouterr().out

    rows = sqlite3.connect(cli_db).execute(
        "SELECT date, minutes, topic FROM time_entries ORDER BY id"
    ).fetchall()
    assert rows == [
        ("2026-01-31", 120, "Proba"),
        ("2026-01-31", 90, "Statistics"),
        ("2026-02-01", 15, None),
    ]


def test_import_time_refuses_to_duplicate(cli_db: Path, tmp_path: Path, capsys: pytest.CaptureFixture):
    _seed_project(tmp_path)
    time_file = tmp_path / "TIME.md"
    time_file.write_text(TABLE)
    assert main(["import-time", "Maths", str(time_file)]) == 0
    capsys.readouterr()

    assert main(["import-time", "Maths", str(time_file)]) == 1
    assert "--replace" in capsys.readouterr().err

    assert main(["import-time", "Maths", str(time_file), "--replace"]) == 0
    count = sqlite3.connect(cli_db).execute("SELECT COUNT(*) FROM time_entries").fetchone()[0]
    assert count == 3


def test_import_time_append_keeps_existing(cli_db: Path, tmp_path: Path):
    _seed_project(tmp_path)
    time_file = tmp_path / "TIME.md"
    time_file.write_text(TABLE)
    assert main(["import-time", "Maths", str(time_file)]) == 0

    assert main(["import-time", "Maths", str(time_file), "--append"]) == 0
    count = sqlite3.connect(cli_db).execute("SELECT COUNT(*) FROM time_entries").fetchone()[0]
    assert count == 6


def test_import_time_unknown_project(cli_db: Path, tmp_path: Path, capsys: pytest.CaptureFixture):
    time_file = tmp_path / "TIME.md"
    time_file.write_text(TABLE)
    assert main(["import-time", "Nope", str(time_file)]) == 1
    assert "no project named" in capsys.readouterr().err


def test_import_time_missing_file(cli_db: Path, tmp_path: Path, capsys: pytest.CaptureFixture):
    _seed_project(tmp_path)
    assert main(["import-time", "Maths", str(tmp_path / "nope.md")]) == 2
    assert "not found" in capsys.readouterr().err
