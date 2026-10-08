use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, PoisonError};

use crate::{Error, Result};

// The watcher thread and commands both commit to the data repo; serializing them avoids
// `index.lock` failures.
static GIT_LOCK: Mutex<()> = Mutex::new(());

/// Runs `git -C <repo> <args>` and returns its stdout.
pub fn run<S: AsRef<OsStr>>(repo: &Path, args: &[S]) -> Result<String> {
    let _guard = GIT_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if !out.status.success() {
        let args = args
            .iter()
            .map(|a| a.as_ref().to_string_lossy())
            .collect::<Vec<_>>();
        return Err(Error::Git {
            args: args.join(" "),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `git add <file>` then `git commit -m <message>`.
pub fn add_commit(repo: &Path, file: &Path, message: &str) -> Result<()> {
    run(repo, &[OsStr::new("add"), file.as_os_str()])?;
    run(repo, &["commit", "-m", message])?;
    Ok(())
}

/// Like [`add_commit`], ignoring failures (nothing to commit, git missing).
pub fn add_commit_quiet(repo: &Path, file: &Path, message: &str) {
    let _ = run(repo, &[OsStr::new("add"), file.as_os_str()]);
    let _ = run(repo, &["commit", "-m", message]);
}
