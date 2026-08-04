"""`pov` command-line entry point.

Operates directly on the SQLite DB and config — no FastAPI server required.
"""

import argparse
import sqlite3
import sys
from pathlib import Path

from pov.db import DB_PATH, init_db
from pov.projects import (
    AmbiguousProjectError,
    ProjectExistsError,
    ProjectNotFoundError,
    add_project_sync,
    find_project_by_name_sync,
    list_projects_sync,
    project_summary_lines,
    remove_project_sync,
)
from pov.storage import POV_DIR
from pov.timelog import TimeLogParseError, insert_time_rows, parse_time_log


def _open_db() -> sqlite3.Connection:
    """Open the same DB the API uses, ensuring the schema exists."""
    import asyncio
    if not DB_PATH.exists():
        # init_db is async; just bootstrap directly.
        POV_DIR.mkdir(parents=True, exist_ok=True)
    asyncio.run(_ensure_schema())
    db = sqlite3.connect(DB_PATH)
    db.row_factory = sqlite3.Row
    db.execute("PRAGMA foreign_keys = ON")
    return db


async def _ensure_schema() -> None:
    from pov.storage import init_storage
    await init_storage()
    await init_db()


def cmd_add(args: argparse.Namespace) -> int:
    db = _open_db()
    name = args.name or Path(args.path).stem
    try:
        pid = add_project_sync(
            db,
            name=name,
            file_path=Path(args.path),
            type=args.type,
            status=args.status,
        )
    except FileNotFoundError as e:
        print(f"error: file not found: {e}", file=sys.stderr)
        return 2
    except ProjectExistsError as e:
        print(f"error: already tracked: {e}", file=sys.stderr)
        return 1
    finally:
        db.close()
    print(f"added: {name} ({pid})")
    return 0


def cmd_list(_: argparse.Namespace) -> int:
    db = _open_db()
    try:
        rows = list_projects_sync(db)
    finally:
        db.close()
    for line in project_summary_lines(rows):
        print(line)
    return 0


def cmd_remove(args: argparse.Namespace) -> int:
    db = _open_db()
    try:
        try:
            row = find_project_by_name_sync(db, args.name)
        except ProjectNotFoundError:
            print(f"error: no project named {args.name!r}", file=sys.stderr)
            return 1
        except AmbiguousProjectError:
            print(
                f"error: multiple projects named {args.name!r}; "
                "remove via the GUI or pass an --id (TODO)",
                file=sys.stderr,
            )
            return 1
        remove_project_sync(db, row)
    finally:
        db.close()
    print(f"removed: {args.name}")
    return 0


def cmd_import_time(args: argparse.Namespace) -> int:
    path = Path(args.path)
    if not path.exists():
        print(f"error: file not found: {path}", file=sys.stderr)
        return 2

    db = _open_db()
    try:
        try:
            project = find_project_by_name_sync(db, args.name)
        except ProjectNotFoundError:
            print(f"error: no project named {args.name!r}", file=sys.stderr)
            return 1
        except AmbiguousProjectError:
            print(f"error: multiple projects named {args.name!r}", file=sys.stderr)
            return 1

        try:
            rows = parse_time_log(path)
        except TimeLogParseError as e:
            print(f"error: {e}", file=sys.stderr)
            return 2

        existing = db.execute(
            "SELECT COUNT(*) FROM time_entries WHERE project_id = ?", (project["id"],)
        ).fetchone()[0]
        if existing and not (args.replace or args.append):
            print(
                f"error: {args.name!r} already has {existing} time entries; "
                "pass --append to keep them or --replace to overwrite",
                file=sys.stderr,
            )
            return 1
        if args.replace:
            db.execute("DELETE FROM time_entries WHERE project_id = ?", (project["id"],))

        count = insert_time_rows(db, project["id"], rows)
    finally:
        db.close()

    total = sum(r.minutes for r in rows)
    print(f"imported: {count} entries ({total / 60:.2f}h) into {args.name}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="pov", description="pov CLI")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_add = sub.add_parser("add", help="track a new project")
    p_add.add_argument("path", help="path to a TODO.md file")
    p_add.add_argument("--name", help="display name (default: file stem)")
    p_add.add_argument(
        "--type",
        choices=("project", "learning"),
        default="project",
        help="project kind (default: project)",
    )
    p_add.add_argument(
        "--status",
        choices=("open", "archived"),
        default="open",
        help='project status (default: open)',
    )
    p_add.set_defaults(func=cmd_add)

    p_list = sub.add_parser("list", help="list tracked projects")
    p_list.set_defaults(func=cmd_list)

    p_remove = sub.add_parser("remove", help="stop tracking a project")
    p_remove.add_argument("name", help="exact project name")
    p_remove.set_defaults(func=cmd_remove)

    p_import = sub.add_parser("import-time", help="import a TIME.md file into a project")
    p_import.add_argument("name", help="exact project name")
    p_import.add_argument("path", help="path to a TIME.md file")
    mode = p_import.add_mutually_exclusive_group()
    mode.add_argument(
        "--replace",
        action="store_true",
        help="delete the project's existing time entries first",
    )
    mode.add_argument(
        "--append",
        action="store_true",
        help="keep the project's existing time entries",
    )
    p_import.set_defaults(func=cmd_import_time)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
