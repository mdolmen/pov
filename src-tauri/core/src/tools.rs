//! Starting the tools tasks are delegated to, one instance per tool.

use std::collections::HashMap;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};

use crate::projects::Tool;
use crate::{Error, Result};

/// The tools pov started, by name. Clones share them.
#[derive(Debug, Clone, Default)]
pub struct Launcher(Arc<Mutex<HashMap<String, Child>>>);

impl Launcher {
    /// Starts `tool` unless it's running: pov's child for it hasn't exited, or its pidfile names
    /// a live process. The tool runs detached, without stdio, and is never waited on. Returns
    /// `true` when it was started.
    pub fn ensure_running(&self, name: &str, tool: &Tool) -> Result<bool> {
        // Held through the start: two delegations at once start the tool once.
        let mut children = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let child_running = children
            .get_mut(name)
            .is_some_and(|child| matches!(child.try_wait(), Ok(None)));
        if child_running || pidfile_alive(&tool.pidfile) {
            return Ok(false);
        }
        let child = Command::new(&tool.command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|e| Error::Invalid(format!("could not start {name}: {e}")))?;
        children.insert(name.to_string(), child);
        Ok(true)
    }
}

/// Whether `pidfile` holds the pid of a live process.
fn pidfile_alive(pidfile: &Path) -> bool {
    let Some(pid) = fs::read_to_string(pidfile)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
        .filter(|pid| *pid > 0)
    else {
        return false;
    };
    // Absolute: the app has no shell `PATH`.
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

    use super::*;

    /// An executable shell script running `body`.
    fn script(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("tool.sh");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Waits up to 2 s for `f` to hold.
    fn eventually(mut f: impl FnMut() -> bool) -> bool {
        (0..200).any(|_| {
            let done = f();
            if !done {
                thread::sleep(Duration::from_millis(10));
            }
            done
        })
    }

    #[test]
    fn starts_a_tool_once_while_it_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let count = tmp.path().join("count");
        let body = format!("echo x >> '{}'\nsleep 2", count.display());
        let tool = Tool {
            command: script(tmp.path(), &body),
            pidfile: tmp.path().join("tool.pid"),
        };
        let launcher = Launcher::default();

        assert!(launcher.ensure_running("t", &tool).unwrap());
        assert!(!launcher.clone().ensure_running("t", &tool).unwrap());

        assert!(eventually(|| count.exists()));
        assert_eq!(fs::read_to_string(&count).unwrap(), "x\n");
    }

    #[test]
    fn starts_a_tool_again_once_it_exited() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = Tool {
            command: script(tmp.path(), "exit 0"),
            pidfile: tmp.path().join("tool.pid"),
        };
        let launcher = Launcher::default();

        assert!(launcher.ensure_running("t", &tool).unwrap());

        assert!(eventually(|| launcher.ensure_running("t", &tool).unwrap()));
    }

    #[test]
    fn a_live_pidfile_means_running() {
        let tmp = tempfile::tempdir().unwrap();
        let pidfile = tmp.path().join("tool.pid");
        fs::write(&pidfile, format!("{}\n", std::process::id())).unwrap();
        let tool = Tool {
            command: tmp.path().join("missing"),
            pidfile,
        };

        assert!(!Launcher::default().ensure_running("t", &tool).unwrap());
    }

    #[test]
    fn a_dead_pidfile_means_stopped() {
        let tmp = tempfile::tempdir().unwrap();
        let mut gone = Command::new("/usr/bin/true").spawn().unwrap();
        gone.wait().unwrap();
        let pidfile = tmp.path().join("tool.pid");
        fs::write(&pidfile, gone.id().to_string()).unwrap();
        let tool = Tool {
            command: script(tmp.path(), "exit 0"),
            pidfile,
        };

        assert!(Launcher::default().ensure_running("t", &tool).unwrap());
    }

    #[test]
    fn a_missing_command_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = Tool {
            command: tmp.path().join("missing"),
            pidfile: tmp.path().join("tool.pid"),
        };

        let err = Launcher::default().ensure_running("t", &tool).unwrap_err();

        assert!(err.to_string().starts_with("could not start t: "), "{err}");
    }
}
