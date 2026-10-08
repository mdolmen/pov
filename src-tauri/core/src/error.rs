use std::io;

/// Errors raised by pov-core.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("git {args}: {stderr}")]
    Git { args: String, stderr: String },
    #[error("HOME is not set")]
    NoHome,
}

pub type Result<T> = std::result::Result<T, Error>;
