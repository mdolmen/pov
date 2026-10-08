use std::env;
use std::path::PathBuf;

use crate::{Error, Result};

/// Locations inside a pov data dir (`~/.local/share/pov/`).
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
    pub projects: PathBuf,
    pub learning: PathBuf,
    pub config: PathBuf,
    pub db: PathBuf,
}

impl Paths {
    /// Paths for a data dir rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            projects: root.join("projects"),
            learning: root.join("learning"),
            config: root.join("config.json"),
            db: root.join("pov.db"),
            root,
        }
    }

    /// The user's data dir. Dev uses `pov-dev` so it never touches real data.
    pub fn data_dir(dev: bool) -> Result<Self> {
        let home = env::var_os("HOME").ok_or(Error::NoHome)?;
        let name = if dev { "pov-dev" } else { "pov" };
        Ok(Self::new(
            PathBuf::from(home).join(".local/share").join(name),
        ))
    }

    /// Where a project's hardlink lives, by project type (`project` or `learning`).
    pub fn hardlink(&self, project_id: &str, project_type: &str) -> PathBuf {
        let base = if project_type == "learning" {
            &self.learning
        } else {
            &self.projects
        };
        base.join(format!("{project_id}.md"))
    }
}
