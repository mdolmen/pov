//! pov domain logic: storage, database, TODO.md parsing, activity and time tracking.
//!
//! No Tauri types here, so the app, the CLI and tests share it.

pub mod db;
pub mod error;
pub mod git;
pub mod paths;
pub mod pyline;
pub mod storage;
pub mod tasks;

pub use error::{Error, Result};
pub use paths::Paths;
