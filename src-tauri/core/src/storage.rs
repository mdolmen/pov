use std::fs;

use crate::{Paths, Result, git};

/// Creates the data dir, its config and its git repo on first launch. Idempotent.
pub fn init(paths: &Paths) -> Result<()> {
    fs::create_dir_all(&paths.projects)?;
    fs::create_dir_all(&paths.learning)?;

    if !paths.config.exists() {
        fs::write(&paths.config, r#"{"projects": [], "learning": {}}"#)?;
    }

    if !paths.root.join(".git").exists() {
        git::run(&paths.root, &["init"])?;
        git::run(
            &paths.root,
            &["commit", "--allow-empty", "-m", "init: pov data repo"],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_dirs_config_and_repo_once() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("pov"));

        init(&paths).unwrap();
        init(&paths).unwrap();

        assert!(paths.projects.is_dir());
        assert!(paths.learning.is_dir());
        assert_eq!(
            fs::read_to_string(&paths.config).unwrap(),
            r#"{"projects": [], "learning": {}}"#
        );
        let log = git::run(&paths.root, &["log", "--format=%s"]).unwrap();
        assert_eq!(log, "init: pov data repo\n");
    }
}
