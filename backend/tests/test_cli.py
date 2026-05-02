"""Tests for the `pov` CLI."""

import sqlite3
from pathlib import Path

import pytest

from pov.cli import main


@pytest.fixture()
def cli_db(pov_dir: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """Point the CLI at a fresh on-disk DB inside the test pov_dir."""
    db_path = pov_dir / "pov.db"
    monkeypatch.setattr("pov.db.DB_PATH", db_path)
    monkeypatch.setattr("pov.cli.DB_PATH", db_path)
    return db_path


def test_cli_add_then_list(cli_db: Path, tmp_path: Path, capsys: pytest.CaptureFixture):
    src = tmp_path / "TODO.md"
    src.write_text("- [ ] task\n")

    assert main(["add", str(src), "--name", "Demo"]) == 0
    assert "added: Demo" in capsys.readouterr().out

    rows = sqlite3.connect(cli_db).execute("SELECT name, type, status FROM projects").fetchall()
    assert rows == [("Demo", "project", "open")]

    assert main(["list"]) == 0
    out = capsys.readouterr().out
    assert "Demo" in out
    assert "project" in out


def test_cli_add_default_name_is_file_stem(cli_db: Path, tmp_path: Path, capsys: pytest.CaptureFixture):
    src = tmp_path / "MyProject.md"
    src.write_text("- [ ] x\n")
    assert main(["add", str(src)]) == 0
    assert "added: MyProject" in capsys.readouterr().out


def test_cli_add_learning_type(cli_db: Path, tmp_path: Path):
    src = tmp_path / "L.md"
    src.write_text("- [ ] x\n")
    assert main(["add", str(src), "--type", "learning"]) == 0
    row = sqlite3.connect(cli_db).execute("SELECT type FROM projects").fetchone()
    assert row[0] == "learning"


def test_cli_add_missing_file_returns_2(cli_db: Path, tmp_path: Path, capsys: pytest.CaptureFixture):
    assert main(["add", str(tmp_path / "nope.md")]) == 2
    err = capsys.readouterr().err
    assert "not found" in err


def test_cli_add_duplicate_returns_1(cli_db: Path, tmp_path: Path, capsys: pytest.CaptureFixture):
    src = tmp_path / "dup.md"
    src.write_text("- [ ] x\n")
    assert main(["add", str(src), "--name", "A"]) == 0
    assert main(["add", str(src), "--name", "A2"]) == 1
    err = capsys.readouterr().err
    assert "already tracked" in err


def test_cli_remove(cli_db: Path, tmp_path: Path, capsys: pytest.CaptureFixture):
    src = tmp_path / "TODO.md"
    src.write_text("- [ ] x\n")
    main(["add", str(src), "--name", "Demo"])
    capsys.readouterr()

    assert main(["remove", "Demo"]) == 0
    assert "removed: Demo" in capsys.readouterr().out

    rows = sqlite3.connect(cli_db).execute("SELECT 1 FROM projects").fetchall()
    assert rows == []
    # Original file untouched
    assert src.exists()


def test_cli_remove_unknown_returns_1(cli_db: Path, capsys: pytest.CaptureFixture):
    assert main(["remove", "no-such-project"]) == 1
    assert "no project named" in capsys.readouterr().err
