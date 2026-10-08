use std::io;
use std::path::PathBuf;

/// Errors raised by pov-core.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("git {args}: {stderr}")]
    Git { args: String, stderr: String },
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    Invalid(String),
    #[error("file not found: {}", .0.display())]
    FileNotFound(PathBuf),
    #[error("already tracked: {}", .0.display())]
    ProjectExists(PathBuf),
    #[error("multiple projects named '{0}'")]
    AmbiguousProject(String),
    #[error("{}:{line}: {msg}", path.display())]
    TimeLogParse {
        path: PathBuf,
        line: usize,
        msg: String,
    },
    #[error("HOME is not set")]
    NoHome,
}

pub type Result<T> = std::result::Result<T, Error>;
