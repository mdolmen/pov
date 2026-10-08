//! Tracked projects.

use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension};

use crate::{Error, Result};

/// A project's tracked file and type, or `NotFound` when the project doesn't exist.
pub(crate) fn file_and_type(conn: &Connection, project_id: &str) -> Result<(PathBuf, String)> {
    conn.query_row(
        "SELECT file_path, type FROM projects WHERE id = ?",
        [project_id],
        |row| Ok((PathBuf::from(row.get::<_, String>(0)?), row.get(1)?)),
    )
    .optional()?
    .ok_or(Error::NotFound("project"))
}
