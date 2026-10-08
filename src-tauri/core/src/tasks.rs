//! TODO.md parsing, task hashing, checkbox toggling and task selection.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::pyline::{read_text, rstrip, split_lines, split_lines_keep_ends, strip};
use crate::{Error, Paths, Result, git};

// `[\s\x1c-\x1f]` is Python's `\s`: Rust's `\s` lacks the `\x1c`-`\x1f` separators.
static CHECKBOX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\s\x1c-\x1f]*)- \[(.)\] (.+)$").unwrap());
static HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(#{1,6})[\s\x1c-\x1f]+(.+)$").unwrap());
static ANY_BOX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[.\]").unwrap());
static CHECKED_BOX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[xX]\]").unwrap());
static OPEN_TASK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^[\s\x1c-\x1f]*- \[ \]").unwrap());

/// A checkbox line indented under a task.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Subtask {
    pub hash: String,
    pub text: String,
    pub checked: bool,
    pub line_number: usize,
}

/// A top-level checkbox line and its subtasks.
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub hash: String,
    pub text: String,
    pub checked: bool,
    pub line_number: usize,
    pub subtasks: Vec<Subtask>,
}

impl Task {
    /// A task with subtasks is done when all of them are checked.
    pub fn is_done(&self) -> bool {
        if self.subtasks.is_empty() {
            self.checked
        } else {
            self.subtasks.iter().all(|s| s.checked)
        }
    }
}

/// A parsed TODO.md entry, in document order.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Heading { text: String, level: usize },
    Task(Task),
}

/// A task as the frontend sees it (`Task` in `src/types.ts`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaskView {
    pub hash: String,
    pub text: String,
    pub checked: bool,
    pub line_number: usize,
    pub subtasks: Vec<Subtask>,
    pub is_done: bool,
    pub is_selected: bool,
}

/// A task list entry as the frontend sees it (`ListItem` in `src/types.ts`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ListItem {
    Heading { text: String, level: usize },
    Task(TaskView),
}

/// Matches a checkbox line, ignoring a trailing `\n` like Python's `$`.
fn checkbox(line: &str) -> Option<regex::Captures<'_>> {
    CHECKBOX_RE.captures(line.strip_suffix('\n').unwrap_or(line))
}

/// Replaces every `[.]` with `[ ]` so the hash doesn't depend on the checkbox state.
fn normalize(line: &str) -> String {
    ANY_BOX_RE.replace_all(rstrip(line), "[ ]").into_owned()
}

/// Hashes a checkbox line, disambiguated by occurrence among identical lines.
///
/// `occurrence` is 1-based: the first appearance of a given normalized line in a file gets 1,
/// the second gets 2, etc. This keeps hashes stable across line shifts but unique for tasks
/// that share text.
pub fn task_hash(line: &str, occurrence: usize) -> String {
    let normalized = normalize(line);
    let key = if occurrence == 1 {
        normalized
    } else {
        format!("{normalized}\x00#{occurrence}")
    };
    let digest = Sha256::digest(key.as_bytes());
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// Counts the occurrence of each normalized line, to hash checkbox lines in file order.
#[derive(Default)]
struct Occurrences(HashMap<String, usize>);

impl Occurrences {
    fn hash(&mut self, line: &str) -> String {
        let count = self.0.entry(normalize(line)).or_insert(0);
        *count += 1;
        task_hash(line, *count)
    }
}

fn set_checked(line: &str, checked: bool) -> String {
    if checked {
        line.replacen("[ ]", "[x]", 1)
    } else {
        CHECKED_BOX_RE.replace(line, "[ ]").into_owned()
    }
}

/// Parses headings and tasks of a TODO.md. Unreadable or non-UTF-8 files have none.
pub fn parse_items(path: &Path) -> Vec<Item> {
    read_text(path).map_or_else(Vec::new, |text| parse_text(&text))
}

/// Parses the tasks of a TODO.md, without headings.
pub fn parse_tasks(path: &Path) -> Vec<Task> {
    read_text(path).map_or_else(Vec::new, |text| tasks_of(&text))
}

fn tasks_of(text: &str) -> Vec<Task> {
    parse_text(text)
        .into_iter()
        .filter_map(|item| match item {
            Item::Task(task) => Some(task),
            Item::Heading { .. } => None,
        })
        .collect()
}

fn parse_text(text: &str) -> Vec<Item> {
    let mut items = Vec::new();
    let mut current: Option<usize> = None;
    let mut seen = Occurrences::default();

    for (i, line) in split_lines(text).into_iter().enumerate() {
        if let Some(h) = HEADING_RE.captures(line) {
            current = None;
            items.push(Item::Heading {
                text: strip(&h[2]).to_string(),
                level: h[1].len(),
            });
            continue;
        }
        let Some(m) = checkbox(line) else { continue };
        let checked = &m[2] == "x" || &m[2] == "X";
        let hash = seen.hash(line);
        let text = m[3].to_string();

        if m[1].is_empty() {
            items.push(Item::Task(Task {
                hash,
                text,
                checked,
                line_number: i,
                subtasks: vec![],
            }));
            current = Some(items.len() - 1);
        } else if let Some(Item::Task(task)) = current.and_then(|c| items.get_mut(c)) {
            task.subtasks.push(Subtask {
                hash,
                text,
                checked,
                line_number: i,
            });
        }
    }
    items
}

/// Counts unchecked checkbox lines, subtasks included.
pub fn count_tasks(path: &Path) -> usize {
    read_text(path).map_or(0, |text| OPEN_TASK_RE.find_iter(&text).count())
}

/// Toggles a task or subtask, keeping parents and subtasks consistent.
///
/// - Task with subtasks: sets it and all its subtasks to `!is_done`.
/// - Task without subtasks: plain toggle.
/// - Subtask: toggles it, then sets its parent to whether all subtasks are now checked.
///
/// Returns `false` when no task has that hash.
pub fn toggle_cascade(path: &Path, target: &str) -> Result<bool> {
    let Some(text) = read_text(path) else {
        return Ok(false);
    };
    let tasks = tasks_of(&text);
    let mut lines: Vec<String> = split_lines_keep_ends(&text)
        .into_iter()
        .map(String::from)
        .collect();
    let mut seen = Occurrences::default();

    if let Some(task) = tasks.iter().find(|t| t.hash == target) {
        if task.subtasks.is_empty() {
            for line in lines.iter_mut().filter(|l| checkbox(l).is_some()) {
                if seen.hash(line) == target {
                    *line = set_checked(line, !task.checked);
                    break;
                }
            }
        } else {
            let new_state = !task.is_done();
            let targets: HashSet<&str> = std::iter::once(task.hash.as_str())
                .chain(task.subtasks.iter().map(|s| s.hash.as_str()))
                .collect();
            for line in lines.iter_mut().filter(|l| checkbox(l).is_some()) {
                if targets.contains(seen.hash(line).as_str()) {
                    *line = set_checked(line, new_state);
                }
            }
        }
    } else if let Some(parent) = tasks
        .iter()
        .find(|t| t.subtasks.iter().any(|s| s.hash == target))
    {
        let new_sub = parent
            .subtasks
            .iter()
            .find(|s| s.hash == target)
            .is_some_and(|s| !s.checked);
        let new_parent = parent
            .subtasks
            .iter()
            .all(|s| if s.hash == target { new_sub } else { s.checked });
        let (mut found_sub, mut found_parent) = (false, false);
        for line in lines.iter_mut().filter(|l| checkbox(l).is_some()) {
            let h = seen.hash(line);
            if h == target && !found_sub {
                *line = set_checked(line, new_sub);
                found_sub = true;
            } else if h == parent.hash && !found_parent {
                *line = set_checked(line, new_parent);
                found_parent = true;
            }
            if found_sub && found_parent {
                break;
            }
        }
    } else {
        return Ok(false);
    }

    fs::write(path, lines.concat())?;
    Ok(true)
}

fn project_file(conn: &Connection, project_id: &str) -> Result<(PathBuf, String)> {
    conn.query_row(
        "SELECT file_path, type FROM projects WHERE id = ?",
        [project_id],
        |row| Ok((PathBuf::from(row.get::<_, String>(0)?), row.get(1)?)),
    )
    .optional()?
    .ok_or(Error::NotFound("project"))
}

fn selected_hashes(conn: &Connection, project_id: &str) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT task_hash FROM selected_tasks WHERE project_id = ?")?;
    let hashes = stmt.query_map([project_id], |row| row.get(0))?;
    Ok(hashes.collect::<rusqlite::Result<_>>()?)
}

fn view(task: Task, selected: &HashSet<String>) -> TaskView {
    TaskView {
        is_done: task.is_done(),
        is_selected: selected.contains(&task.hash),
        hash: task.hash,
        text: task.text,
        checked: task.checked,
        line_number: task.line_number,
        subtasks: task.subtasks,
    }
}

/// Lists a project's headings and tasks, with their selection state.
pub fn list(conn: &Connection, project_id: &str) -> Result<Vec<ListItem>> {
    let (file_path, _) = project_file(conn, project_id)?;
    let selected = selected_hashes(conn, project_id)?;
    Ok(parse_items(&file_path)
        .into_iter()
        .map(|item| match item {
            Item::Heading { text, level } => ListItem::Heading { text, level },
            Item::Task(task) => ListItem::Task(view(task, &selected)),
        })
        .collect())
}

/// Toggles a task or subtask in the project's file, commits the change to the data repo, and
/// unselects the task if it is now done. Returns the task (the parent, for a subtask).
pub fn toggle(
    conn: &Connection,
    paths: &Paths,
    project_id: &str,
    task_hash: &str,
) -> Result<TaskView> {
    let (file_path, project_type) = project_file(conn, project_id)?;
    if !toggle_cascade(&file_path, task_hash)? {
        return Err(Error::NotFound("task"));
    }

    let hardlink = paths.hardlink(project_id, &project_type);
    if hardlink.exists() {
        let name = hardlink.file_name().unwrap_or_default().to_string_lossy();
        git::add_commit_quiet(&paths.root, &hardlink, &format!("activity: {name}"));
    }

    let tasks = parse_tasks(&file_path);
    let task = tasks
        .iter()
        .position(|t| t.hash == task_hash)
        .or_else(|| {
            tasks
                .iter()
                .position(|t| t.subtasks.iter().any(|s| s.hash == task_hash))
        })
        .map(|i| tasks[i].clone())
        .ok_or(Error::NotFound("task"))?;

    if task.is_done() {
        conn.execute(
            "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ?",
            [project_id, &task.hash],
        )?;
    }
    let selected = selected_hashes(conn, project_id)?;
    Ok(view(task, &selected))
}

/// Marks a task as selected ("next"). Selecting twice is a no-op.
pub fn select(conn: &Connection, project_id: &str, task_hash: &str) -> Result<()> {
    project_file(conn, project_id)?;
    conn.execute(
        "INSERT OR IGNORE INTO selected_tasks (project_id, task_hash) VALUES (?, ?)",
        [project_id, task_hash],
    )?;
    Ok(())
}

/// Removes a task from the selection.
pub fn unselect(conn: &Connection, project_id: &str, task_hash: &str) -> Result<()> {
    project_file(conn, project_id)?;
    conn.execute(
        "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ?",
        [project_id, task_hash],
    )?;
    Ok(())
}

/// Removes selections whose task is done in the source file.
pub fn cleanup_selected(conn: &Connection) -> Result<()> {
    let projects: Vec<(String, PathBuf)> = conn
        .prepare("SELECT id, file_path FROM projects")?
        .query_map((), |row| {
            Ok((row.get(0)?, PathBuf::from(row.get::<_, String>(1)?)))
        })?
        .collect::<rusqlite::Result<_>>()?;

    let tx = conn.unchecked_transaction()?;
    for (project_id, file_path) in projects {
        if !file_path.exists() {
            continue;
        }
        for task in parse_tasks(&file_path).into_iter().filter(Task::is_done) {
            tx.execute(
                "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ?",
                [&project_id, &task.hash],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, storage};

    const MIXED_CONTENT: &str = "\
# Section one

Some plain text here.

- [ ] Task one
  - [ ] Subtask A
  - [x] Subtask B
- [x] Task two
- [ ] Task three

## Section two

- [ ] Task four
";

    const TODO: &str = "# My Project\n\n- [ ] Task one\n- [x] Task two (done)\n- [ ] Task three\n";

    fn file(dir: &Path, content: &str) -> PathBuf {
        let path = dir.join("t.md");
        fs::write(&path, content).unwrap();
        path
    }

    /// A data dir, a DB and a project tracking `TODO` through a hardlink.
    fn project() -> (tempfile::TempDir, Paths, Connection, PathBuf, String) {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("pov"));
        storage::init(&paths).unwrap();
        db::init(&paths).unwrap();
        let conn = db::open(&paths).unwrap();
        let todo = file(tmp.path(), TODO);
        let id = "p1".to_string();
        fs::hard_link(&todo, paths.hardlink(&id, "project")).unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, file_path) VALUES (?, 'P', ?)",
            [&id, todo.to_str().unwrap()],
        )
        .unwrap();
        (tmp, paths, conn, todo, id)
    }

    fn tasks_of_list(items: &[ListItem]) -> Vec<&TaskView> {
        items
            .iter()
            .filter_map(|i| match i {
                ListItem::Task(t) => Some(t),
                ListItem::Heading { .. } => None,
            })
            .collect()
    }

    #[test]
    fn parser_finds_tasks_subtasks_states_and_lines() {
        let tasks = tasks_of(MIXED_CONTENT);
        let texts: Vec<_> = tasks.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["Task one", "Task two", "Task three", "Task four"]);

        let subs = &tasks[0].subtasks;
        assert_eq!(
            (subs[0].text.as_str(), subs[0].checked),
            ("Subtask A", false)
        );
        assert_eq!(
            (subs[1].text.as_str(), subs[1].checked),
            ("Subtask B", true)
        );

        assert!(!tasks[0].checked);
        assert!(tasks[1].checked);
        assert_eq!(tasks[0].line_number, 4);
        assert_eq!(tasks[1].line_number, 7);
    }

    #[test]
    fn parser_handles_empty_and_missing_files() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(parse_tasks(&file(tmp.path(), "")).is_empty());
        assert!(parse_tasks(&tmp.path().join("nonexistent.md")).is_empty());
    }

    #[test]
    fn is_done_follows_subtasks() {
        assert!(!tasks_of("- [ ] Task\n")[0].is_done());
        assert!(tasks_of("- [x] Task\n")[0].is_done());
        assert!(tasks_of("- [ ] Task\n  - [x] Sub A\n  - [x] Sub B\n")[0].is_done());
        assert!(!tasks_of("- [ ] Task\n  - [x] Sub A\n  - [ ] Sub B\n")[0].is_done());
    }

    #[test]
    fn hash_ignores_check_state_and_line_shifts() {
        assert_eq!(task_hash("- [ ] My task", 1), task_hash("- [x] My task", 1));

        let before = tasks_of("- [ ] Task one\n- [ ] Task two\n");
        let after = tasks_of("- [ ] Task one\n- [ ] Inserted\n- [ ] Task two\n");
        assert_eq!(before[1].hash, after[2].hash);
    }

    #[test]
    fn hash_is_unique_for_duplicate_text() {
        let tasks = tasks_of("- [ ] Same task\n- [ ] Other\n- [ ] Same task\n");
        assert_eq!(tasks[0].text, tasks[2].text);
        assert_ne!(tasks[0].hash, tasks[2].hash);
    }

    #[test]
    fn toggle_disambiguates_duplicate_text() {
        let tmp = tempfile::tempdir().unwrap();
        let path = file(tmp.path(), "- [ ] Same task\n- [ ] Same task\n");
        let hash = parse_tasks(&path)[1].hash.clone();
        assert!(toggle_cascade(&path, &hash).unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [ ] Same task\n- [x] Same task\n"
        );
    }

    #[test]
    fn toggle_checks_and_unchecks_only_the_matching_line() {
        let tmp = tempfile::tempdir().unwrap();
        let path = file(
            tmp.path(),
            "- [ ] Task one\n- [ ] Task two\n- [ ] Task three\n",
        );
        assert!(toggle_cascade(&path, &task_hash("- [ ] Task two", 1)).unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [ ] Task one\n- [x] Task two\n- [ ] Task three\n"
        );

        let path = file(tmp.path(), "- [x] Task one\n");
        assert!(toggle_cascade(&path, &task_hash("- [x] Task one", 1)).unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "- [ ] Task one\n");
    }

    #[test]
    fn toggle_unknown_hash_is_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let path = file(tmp.path(), "- [ ] Task one\n");
        assert!(!toggle_cascade(&path, "nonexistenthash").unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "- [ ] Task one\n");
    }

    #[test]
    fn list_returns_headings_and_tasks() {
        let (_tmp, _paths, conn, _todo, id) = project();
        let items = list(&conn, &id).unwrap();
        assert_eq!(
            items[0],
            ListItem::Heading {
                text: "My Project".into(),
                level: 1
            }
        );
        let texts: Vec<_> = tasks_of_list(&items)
            .iter()
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(texts, ["Task one", "Task two (done)", "Task three"]);
    }

    #[test]
    fn unknown_project_is_not_found() {
        let (_tmp, paths, conn, _todo, _id) = project();
        assert!(matches!(
            list(&conn, "nope"),
            Err(Error::NotFound("project"))
        ));
        assert!(matches!(
            toggle(&conn, &paths, "nope", "h"),
            Err(Error::NotFound("project"))
        ));
        assert!(matches!(
            select(&conn, "nope", "h"),
            Err(Error::NotFound("project"))
        ));
    }

    #[test]
    fn list_of_a_binary_file_is_empty() {
        let (tmp, _paths, conn, _todo, _id) = project();
        let pdf = tmp.path().join("book.pdf");
        fs::write(&pdf, b"%PDF-1.4\n\xe2\x28\xa1 binary\n").unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, file_path, type) VALUES ('b', 'Book', ?, 'learning')",
            [pdf.to_str().unwrap()],
        )
        .unwrap();
        assert!(list(&conn, "b").unwrap().is_empty());
    }

    #[test]
    fn toggle_writes_the_file_and_commits_the_hardlink() {
        let (_tmp, paths, conn, todo, id) = project();
        let hash = task_hash("- [ ] Task one", 1);

        let task = toggle(&conn, &paths, &id, &hash).unwrap();

        assert!(task.checked);
        assert!(
            fs::read_to_string(&todo)
                .unwrap()
                .contains("- [x] Task one")
        );
        let log = git::run(&paths.root, &["log", "-1", "--format=%s"]).unwrap();
        assert_eq!(log, "activity: p1.md\n");
    }

    #[test]
    fn toggle_unknown_task_is_not_found() {
        let (_tmp, paths, conn, _todo, id) = project();
        assert!(matches!(
            toggle(&conn, &paths, &id, "badhash"),
            Err(Error::NotFound("task"))
        ));
    }

    #[test]
    fn select_unselect_and_idempotence() {
        let (_tmp, _paths, conn, _todo, id) = project();
        let hash = task_hash("- [ ] Task one", 1);
        let is_selected = |conn: &Connection| {
            tasks_of_list(&list(conn, &id).unwrap())
                .iter()
                .any(|t| t.hash == hash && t.is_selected)
        };

        select(&conn, &id, &hash).unwrap();
        select(&conn, &id, &hash).unwrap();
        assert!(is_selected(&conn));

        unselect(&conn, &id, &hash).unwrap();
        assert!(!is_selected(&conn));
    }

    #[test]
    fn toggling_to_done_unselects() {
        let (_tmp, paths, conn, _todo, id) = project();
        let hash = task_hash("- [ ] Task one", 1);
        select(&conn, &id, &hash).unwrap();

        let task = toggle(&conn, &paths, &id, &hash).unwrap();

        assert!(task.is_done);
        assert!(!task.is_selected);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM selected_tasks", (), |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn cleanup_removes_selected_tasks_that_are_done() {
        let (_tmp, _paths, conn, _todo, id) = project();
        select(&conn, &id, &task_hash("- [ ] Task one", 1)).unwrap();
        select(&conn, &id, &task_hash("- [x] Task two (done)", 1)).unwrap();

        cleanup_selected(&conn).unwrap();

        let left: String = conn
            .query_row("SELECT task_hash FROM selected_tasks", (), |r| r.get(0))
            .unwrap();
        assert_eq!(left, task_hash("- [ ] Task one", 1));
    }
}
