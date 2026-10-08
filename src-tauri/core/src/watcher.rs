//! Commits edits of the tracked files in the data repo, so activity has a history.

use std::path::Path;

use notify::event::ModifyKind;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{Paths, Result, git};

/// Watches `projects/` and `learning/`; each modified `.md` file is committed as
/// `activity: <file>`. Watching stops when the returned watcher is dropped.
pub fn start(paths: &Paths) -> Result<RecommendedWatcher> {
    let handler_paths = paths.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        if let Ok(event) = event {
            handle(&handler_paths, &event);
        }
    })?;
    watcher.watch(&paths.projects, RecursiveMode::NonRecursive)?;
    watcher.watch(&paths.learning, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

fn handle(paths: &Paths, event: &Event) {
    if !matches!(event.kind, EventKind::Modify(kind) if !matches!(kind, ModifyKind::Name(_))) {
        return;
    }
    for path in &event.paths {
        if path.extension().is_some_and(|ext| ext == "md") && !path.is_dir() {
            commit(paths, path);
        }
    }
}

fn commit(paths: &Paths, path: &Path) {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    git::add_commit_quiet(&paths.root, path, &format!("activity: {name}"));
}

#[cfg(test)]
mod tests {
    use std::fs;

    use notify::event::DataChange;

    use super::*;
    use crate::storage;

    fn modified(path: &Path) -> Event {
        Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Content))).add_path(path.into())
    }

    fn wait_for_subject(paths: &Paths, subject: &str) -> bool {
        (0..50).any(|_| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            last_subject(paths) == subject
        })
    }

    fn last_subject(paths: &Paths) -> String {
        git::run(&paths.root, &["log", "-1", "--format=%s"])
            .unwrap()
            .trim()
            .to_string()
    }

    #[test]
    fn modified_md_file_is_committed() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());
        storage::init(&paths).unwrap();
        let md = paths.projects.join("test.md");
        fs::write(&md, "- [ ] task\n").unwrap();

        handle(&paths, &modified(&md));

        assert_eq!(last_subject(&paths), "activity: test.md");
    }

    #[test]
    fn other_files_and_events_are_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());
        storage::init(&paths).unwrap();
        let txt = paths.projects.join("something.txt");
        fs::write(&txt, "x").unwrap();
        let md = paths.projects.join("test.md");
        fs::write(&md, "- [ ] task\n").unwrap();

        handle(&paths, &modified(&txt));
        handle(
            &paths,
            &Event::new(EventKind::Create(notify::event::CreateKind::File)).add_path(md),
        );

        assert_eq!(last_subject(&paths), "init: pov data repo");
    }

    #[test]
    fn watcher_commits_edits_through_the_data_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("pov"));
        storage::init(&paths).unwrap();
        let md = paths.learning.join("live.md");
        fs::write(&md, "- [ ] task\n").unwrap();

        let _watcher = start(&paths).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        fs::write(&md, "- [x] task\n").unwrap();

        assert!(wait_for_subject(&paths, "activity: live.md"));
    }

    #[test]
    // FSEvents reports an edit of the original file on its hardlink in the data dir.
    fn watcher_sees_edits_through_the_original_path() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("pov"));
        storage::init(&paths).unwrap();
        let original = tmp.path().join("TODO.md");
        fs::write(&original, "- [ ] task\n").unwrap();
        fs::hard_link(&original, paths.projects.join("p.md")).unwrap();

        let _watcher = start(&paths).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        fs::write(&original, "- [x] task\n").unwrap();

        assert!(wait_for_subject(&paths, "activity: p.md"));
    }
}
