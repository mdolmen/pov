//! pov domain logic: storage, database, TODO.md parsing, activity and time tracking.
//!
//! No Tauri types here, so the app, the CLI and tests share it.

pub mod activity;
pub mod db;
pub mod error;
pub mod git;
pub mod paths;
pub mod projects;
pub mod pyline;
pub mod storage;
pub mod tasks;
pub mod timelog;
pub mod watcher;

pub use error::{Error, Result};
pub use paths::Paths;
