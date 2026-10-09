use std::time::Duration;

use rusqlite::Connection;

use crate::{Paths, Result};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS projects (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    file_path    TEXT NOT NULL UNIQUE,
    status       TEXT NOT NULL DEFAULT 'open',
    sub_status   TEXT,
    type         TEXT NOT NULL DEFAULT 'project',
    has_hardlink INTEGER NOT NULL DEFAULT 1,
    paused_until TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS selected_tasks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    task_hash   TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    stage       TEXT NOT NULL DEFAULT 'next',
    UNIQUE(project_id, task_hash)
);

CREATE TABLE IF NOT EXISTS time_entries (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    date        TEXT NOT NULL,
    minutes     INTEGER NOT NULL,
    topic       TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_time_entries_project_date
ON time_entries(project_id, date);

DROP TABLE IF EXISTS activity;
";

// The app and the CLI share the file; wait for the other's locks instead of failing with
// SQLITE_BUSY.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Opens the pov database with foreign keys enforced.
pub fn open(paths: &Paths) -> Result<Connection> {
    let conn = Connection::open(&paths.db)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(conn)
}

/// Creates the tables if they don't exist and applies pending column migrations.
pub fn init(paths: &Paths) -> Result<()> {
    let conn = open(paths)?;
    conn.execute_batch(SCHEMA)?;
    if !has_column(&conn, "projects", "paused_until")? {
        // ISO 8601 date string (YYYY-MM-DD); SQLite has no native DATE type.
        conn.execute("ALTER TABLE projects ADD COLUMN paused_until TEXT", ())?;
    }
    if !has_column(&conn, "selected_tasks", "stage")? {
        // 'next' or 'queue'; rows from before the queue stage are next tasks.
        conn.execute(
            "ALTER TABLE selected_tasks ADD COLUMN stage TEXT NOT NULL DEFAULT 'next'",
            (),
        )?;
    }
    Ok(())
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = stmt.query_map((), |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());

        init(&paths).unwrap();
        init(&paths).unwrap();

        let conn = open(&paths).unwrap();
        assert!(has_column(&conn, "projects", "paused_until").unwrap());
        assert!(has_column(&conn, "time_entries", "topic").unwrap());
        assert!(has_column(&conn, "selected_tasks", "stage").unwrap());
    }

    #[test]
    fn init_migrates_an_old_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());
        let conn = open(&paths).unwrap();
        conn.execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL,
                 file_path TEXT NOT NULL UNIQUE, status TEXT NOT NULL DEFAULT 'open',
                 sub_status TEXT, type TEXT NOT NULL DEFAULT 'project',
                 has_hardlink INTEGER NOT NULL DEFAULT 1,
                 created_at TEXT NOT NULL DEFAULT (datetime('now')));
             INSERT INTO projects (id, name, file_path) VALUES ('p1', 'Old', '/tmp/TODO.md');
             CREATE TABLE selected_tasks (id INTEGER PRIMARY KEY AUTOINCREMENT,
                 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                 task_hash TEXT NOT NULL,
                 created_at TEXT NOT NULL DEFAULT (datetime('now')),
                 UNIQUE(project_id, task_hash));
             INSERT INTO selected_tasks (project_id, task_hash) VALUES ('p1', 'h');
             CREATE TABLE activity (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        init(&paths).unwrap();

        assert!(has_column(&conn, "projects", "paused_until").unwrap());
        let name: String = conn
            .query_row("SELECT name FROM projects WHERE id = 'p1'", (), |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "Old");
        let stage: String = conn
            .query_row("SELECT stage FROM selected_tasks", (), |r| r.get(0))
            .unwrap();
        assert_eq!(stage, "next");
        let activity: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'activity'",
                (),
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(activity, 0);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());
        init(&paths).unwrap();

        let conn = open(&paths).unwrap();
        let orphan = conn.execute(
            "INSERT INTO selected_tasks (project_id, task_hash) VALUES ('nope', 'h')",
            (),
        );
        assert!(orphan.is_err());
    }
}
