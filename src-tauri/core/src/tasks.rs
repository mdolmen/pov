//! TODO.md parsing, task hashing, checkbox toggling and task selection.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::pyline::{
    read_text, rstrip, split_lines, split_lines_keep_ends, strip, universal_newlines,
};
use crate::{Error, Paths, Result, git, projects};

// `[\s\x1c-\x1f]` is Python's `\s`: Rust's `\s` lacks the `\x1c`-`\x1f` separators.
static CHECKBOX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\s\x1c-\x1f]*)- \[(.)\] (.+)$").unwrap());
static HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(#{1,6})[\s\x1c-\x1f]+(.+)$").unwrap());

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
    pub is_queued: bool,
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

/// Writes `state` in the checkbox of a checkbox line; other lines are returned unchanged.
fn with_state(line: &str, state: char) -> String {
    let Some(old) = checkbox(line).and_then(|m| m.get(2)) else {
        return line.to_string();
    };
    format!("{}{state}{}", &line[..old.start()], &line[old.end()..])
}

/// Blanks the checkbox so the hash doesn't depend on the checkbox state.
fn normalize(line: &str) -> String {
    with_state(rstrip(line), ' ')
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
    with_state(line, if checked { 'x' } else { ' ' })
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
        let checked = &m[2] != " ";
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

/// Counts the top-level tasks that aren't done.
pub fn count_tasks(path: &Path) -> usize {
    parse_tasks(path).iter().filter(|t| !t.is_done()).count()
}

/// Toggles a task or subtask, keeping parents and subtasks consistent.
///
/// - Task with subtasks: sets it and all its subtasks to `!is_done`.
/// - Task without subtasks: plain toggle.
/// - Subtask: toggles it, then sets its parent to whether all subtasks are now checked.
///
/// Keeps the file's CRLF line endings. Returns `false` when no task has that hash.
pub fn toggle_cascade(path: &Path, target: &str) -> Result<bool> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok(false);
    };
    let crlf = raw.contains("\r\n");
    let text = universal_newlines(&raw);
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

    let text = lines.concat();
    fs::write(
        path,
        if crlf {
            text.replace('\n', "\r\n")
        } else {
            text
        },
    )?;
    Ok(true)
}

/// The hashes of a project's tasks marked `stage` (`next` or `queue`).
fn hashes(conn: &Connection, project_id: &str, stage: &str) -> Result<HashSet<String>> {
    let mut stmt =
        conn.prepare("SELECT task_hash FROM selected_tasks WHERE project_id = ? AND stage = ?")?;
    let hashes = stmt.query_map([project_id, stage], |row| row.get(0))?;
    Ok(hashes.collect::<rusqlite::Result<_>>()?)
}

fn view(task: Task, next: &HashSet<String>, queued: &HashSet<String>) -> TaskView {
    TaskView {
        is_done: task.is_done(),
        is_selected: next.contains(&task.hash),
        is_queued: queued.contains(&task.hash),
        hash: task.hash,
        text: task.text,
        checked: task.checked,
        line_number: task.line_number,
        subtasks: task.subtasks,
    }
}

/// Lists a project's headings and tasks, with their next / queue marks.
pub fn list(conn: &Connection, project_id: &str) -> Result<Vec<ListItem>> {
    let (file_path, _) = projects::file_and_type(conn, project_id)?;
    let next = hashes(conn, project_id, "next")?;
    let queued = hashes(conn, project_id, "queue")?;
    Ok(parse_items(&file_path)
        .into_iter()
        .map(|item| match item {
            Item::Heading { text, level } => ListItem::Heading { text, level },
            Item::Task(task) => ListItem::Task(view(task, &next, &queued)),
        })
        .collect())
}

/// Toggles a task or subtask in the project's file, commits the change to the data repo, and
/// clears its next / queue mark if it is now done. Returns the task (the parent, for a subtask).
pub fn toggle(
    conn: &Connection,
    paths: &Paths,
    project_id: &str,
    task_hash: &str,
) -> Result<TaskView> {
    let (file_path, project_type) = projects::file_and_type(conn, project_id)?;
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
    let next = hashes(conn, project_id, "next")?;
    let queued = hashes(conn, project_id, "queue")?;
    Ok(view(task, &next, &queued))
}

/// Marks a task `stage` (`next` or `queue`), replacing its other mark.
fn set_stage(conn: &Connection, project_id: &str, task_hash: &str, stage: &str) -> Result<()> {
    projects::file_and_type(conn, project_id)?;
    conn.execute(
        "INSERT INTO selected_tasks (project_id, task_hash, stage) VALUES (?, ?, ?)
         ON CONFLICT(project_id, task_hash) DO UPDATE SET stage = excluded.stage",
        [project_id, task_hash, stage],
    )?;
    Ok(())
}

/// Marks a task as selected ("next"), moving it up if it was queued. Selecting twice is a
/// no-op.
pub fn select(conn: &Connection, project_id: &str, task_hash: &str) -> Result<()> {
    set_stage(conn, project_id, task_hash, "next")
}

/// Queues a task: committed to, but not next yet. Queuing a next task moves it down.
pub fn queue(conn: &Connection, project_id: &str, task_hash: &str) -> Result<()> {
    set_stage(conn, project_id, task_hash, "queue")
}

/// Removes a task from the queue; a next task keeps its mark.
pub fn unqueue(conn: &Connection, project_id: &str, task_hash: &str) -> Result<()> {
    projects::file_and_type(conn, project_id)?;
    conn.execute(
        "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ? AND stage = 'queue'",
        [project_id, task_hash],
    )?;
    Ok(())
}

/// Removes a task from the selection.
pub fn unselect(conn: &Connection, project_id: &str, task_hash: &str) -> Result<()> {
    projects::file_and_type(conn, project_id)?;
    conn.execute(
        "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ?",
        [project_id, task_hash],
    )?;
    Ok(())
}

/// What a project should handle next (`NextUpEntry` in `src/types.ts`).
///
/// A milestone entry has `milestone` set, with `done` / `total` over its tasks. A task entry
/// (next tasks before any `##`) has `task_text` and `task_hash` set, with `done` / `total` over
/// that task's subtasks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NextUpEntry {
    pub project_id: String,
    pub project_name: String,
    pub project_type: String,
    pub milestone: Option<String>,
    pub task_text: Option<String>,
    pub task_hash: Option<String>,
    pub next_count: usize,
    pub done: usize,
    pub total: usize,
}

/// Groups top-level tasks by their `##` milestone, in file order. Tasks before any `##` form a
/// group without a name; `###` and deeper stay in the `##` above them; `#` is ignored.
fn milestones(items: Vec<Item>) -> Vec<(Option<String>, Vec<Task>)> {
    let mut groups: Vec<(Option<String>, Vec<Task>)> = vec![(None, vec![])];
    for item in items {
        match item {
            Item::Heading { text, level: 2 } => groups.push((Some(text), vec![])),
            Item::Heading { .. } => {}
            Item::Task(task) => {
                if let Some((_, tasks)) = groups.last_mut() {
                    tasks.push(task);
                }
            }
        }
    }
    groups
}

/// One entry per open project: its first milestone holding a selected task that isn't done.
/// Most next tasks first, then by project name.
pub fn next_up(conn: &Connection) -> Result<Vec<NextUpEntry>> {
    let mut entries = Vec::new();
    for row in projects::list_by_name(conn)? {
        if row.status != "open" {
            continue;
        }
        let selected = hashes(conn, &row.id, "next")?;
        let is_next = |t: &Task| selected.contains(&t.hash) && !t.is_done();
        let Some((milestone, tasks)) = milestones(parse_items(&row.file_path))
            .into_iter()
            .find(|(_, tasks)| tasks.iter().any(is_next))
        else {
            continue;
        };
        let next_count = tasks.iter().filter(|t| is_next(t)).count();
        let (task_text, task_hash, done, total) = match milestone {
            Some(_) => (
                None,
                None,
                tasks.iter().filter(|t| t.is_done()).count(),
                tasks.len(),
            ),
            None => {
                let Some(task) = tasks.into_iter().find(is_next) else {
                    continue;
                };
                let done = task.subtasks.iter().filter(|s| s.checked).count();
                let total = task.subtasks.len();
                (Some(task.text), Some(task.hash), done, total)
            }
        };
        entries.push(NextUpEntry {
            project_id: row.id,
            project_name: row.name,
            project_type: row.project_type,
            milestone,
            task_text,
            task_hash,
            next_count,
            done,
            total,
        });
    }
    // Stable: ties keep the name order of `list_by_name`.
    entries.sort_by_key(|e| std::cmp::Reverse(e.next_count));
    Ok(entries)
}

/// Removes the next / queue marks of tasks that are done in the source file.
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
    fn count_tasks_counts_open_top_level_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        let path = file(tmp.path(), "- [ ] One\n  - [ ] Sub A\n  - [x] Sub B\n");
        assert_eq!(count_tasks(&path), 1);
        let path = file(
            tmp.path(),
            "- [ ] One\n  - [x] Sub A\n- [-] Two\n- [ ] Three\n",
        );
        assert_eq!(count_tasks(&path), 1);
    }

    #[test]
    fn other_states_are_closed() {
        assert!(tasks_of("- [-] Cancelled\n")[0].is_done());
        assert!(tasks_of("- [~] Deferred\n")[0].is_done());
        assert!(tasks_of("- [ ] Task\n  - [x] Sub A\n  - [-] Sub B\n")[0].is_done());

        let tmp = tempfile::tempdir().unwrap();
        let path = file(tmp.path(), "- [-] Cancelled\n");
        assert!(toggle_cascade(&path, &task_hash("- [-] Cancelled", 1)).unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "- [ ] Cancelled\n");
    }

    #[test]
    fn hash_ignores_check_state_and_line_shifts() {
        assert_eq!(task_hash("- [ ] My task", 1), task_hash("- [x] My task", 1));

        let before = tasks_of("- [ ] Task one\n- [ ] Task two\n");
        let after = tasks_of("- [ ] Task one\n- [ ] Inserted\n- [ ] Task two\n");
        assert_eq!(before[1].hash, after[2].hash);
    }

    #[test]
    fn hash_keeps_brackets_in_the_text() {
        assert_ne!(task_hash("- [ ] see [a]", 1), task_hash("- [ ] see [b]", 1));
        assert_eq!(task_hash("- [x] see [a]", 1), task_hash("- [ ] see [a]", 1));
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
    fn toggle_leaves_brackets_in_the_text() {
        let tmp = tempfile::tempdir().unwrap();
        let path = file(tmp.path(), "- [x] fix [ ] parser\n  - [ ] sub\n");
        let hash = parse_tasks(&path)[0].hash.clone();
        assert!(toggle_cascade(&path, &hash).unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [x] fix [ ] parser\n  - [x] sub\n"
        );
    }

    #[test]
    fn toggle_keeps_crlf_line_endings() {
        let tmp = tempfile::tempdir().unwrap();
        let path = file(tmp.path(), "# T\r\n- [ ] one\r\n- [ ] two\r\n");
        assert!(toggle_cascade(&path, &task_hash("- [ ] one", 1)).unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# T\r\n- [x] one\r\n- [ ] two\r\n"
        );
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

    /// The (next, queued) marks of a task in the project's list.
    fn marks(conn: &Connection, id: &str, hash: &str) -> (bool, bool) {
        let items = list(conn, id).unwrap();
        let task = tasks_of_list(&items)
            .into_iter()
            .find(|t| t.hash == hash)
            .unwrap();
        (task.is_selected, task.is_queued)
    }

    fn mark_rows(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM selected_tasks", (), |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn queue_and_select_move_a_task_between_stages() {
        let (_tmp, _paths, conn, _todo, id) = project();
        let hash = task_hash("- [ ] Task one", 1);

        queue(&conn, &id, &hash).unwrap();
        queue(&conn, &id, &hash).unwrap();
        assert_eq!(marks(&conn, &id, &hash), (false, true));

        select(&conn, &id, &hash).unwrap();
        assert_eq!(marks(&conn, &id, &hash), (true, false));

        queue(&conn, &id, &hash).unwrap();
        assert_eq!(marks(&conn, &id, &hash), (false, true));
        assert_eq!(mark_rows(&conn), 1);

        unqueue(&conn, &id, &hash).unwrap();
        assert_eq!(marks(&conn, &id, &hash), (false, false));
        assert_eq!(mark_rows(&conn), 0);
    }

    #[test]
    fn unqueue_leaves_a_next_task_alone() {
        let (_tmp, _paths, conn, _todo, id) = project();
        let hash = task_hash("- [ ] Task one", 1);
        select(&conn, &id, &hash).unwrap();

        unqueue(&conn, &id, &hash).unwrap();

        assert_eq!(marks(&conn, &id, &hash), (true, false));
    }

    #[test]
    fn queue_unknown_project_is_not_found() {
        let (_tmp, _paths, conn, _todo, _id) = project();
        assert!(matches!(
            queue(&conn, "nope", "h"),
            Err(Error::NotFound("project"))
        ));
        assert!(matches!(
            unqueue(&conn, "nope", "h"),
            Err(Error::NotFound("project"))
        ));
    }

    #[test]
    fn done_queued_tasks_lose_their_mark() {
        let (_tmp, paths, conn, _todo, id) = project();
        let one = task_hash("- [ ] Task one", 1);
        queue(&conn, &id, &one).unwrap();
        queue(&conn, &id, &task_hash("- [x] Task two (done)", 1)).unwrap();

        cleanup_selected(&conn).unwrap();
        assert_eq!(mark_rows(&conn), 1);

        let task = toggle(&conn, &paths, &id, &one).unwrap();
        assert!(task.is_done && !task.is_queued);
        assert_eq!(mark_rows(&conn), 0);
    }

    #[test]
    fn next_up_ignores_queued_tasks() {
        let (tmp, _paths, conn, _todo, _id) = project();
        add_project(&conn, tmp.path(), "Q", "open", "## M\n- [ ] a\n", &[]);
        queue(&conn, "Q", &task_hash("- [ ] a", 1)).unwrap();

        assert!(next_up(&conn).unwrap().is_empty());
    }

    /// Adds a project tracking `content`, with the tasks named in `next` selected.
    fn add_project(
        conn: &Connection,
        dir: &Path,
        name: &str,
        status: &str,
        content: &str,
        next: &[&str],
    ) {
        let path = dir.join(format!("{name}.md"));
        fs::write(&path, content).unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, file_path, status) VALUES (?, ?, ?, ?)",
            [name, name, path.to_str().unwrap(), status],
        )
        .unwrap();
        let tasks = parse_tasks(&path);
        for text in next {
            let task = tasks.iter().find(|t| t.text == *text).unwrap();
            select(conn, name, &task.hash).unwrap();
        }
    }

    #[test]
    fn next_up_takes_the_first_milestone_with_next_tasks() {
        let (tmp, _paths, conn, _todo, _id) = project();
        let content = "\
# Title
## One
- [x] a
## Two
- [ ] b
- [ ] c
### Later
- [ ] d
- [x] e
## Three
- [ ] f
";
        add_project(&conn, tmp.path(), "Q", "open", content, &["f", "b", "d"]);

        let entries = next_up(&conn).unwrap();

        assert_eq!(
            entries,
            [NextUpEntry {
                project_id: "Q".into(),
                project_name: "Q".into(),
                project_type: "project".into(),
                milestone: Some("Two".into()),
                task_text: None,
                task_hash: None,
                next_count: 2,
                done: 1,
                total: 4,
            }]
        );
    }

    #[test]
    fn next_up_gives_a_task_before_any_milestone() {
        let (tmp, _paths, conn, _todo, _id) = project();
        let content = "# T\n- [ ] a\n- [ ] b\n  - [x] b1\n  - [ ] b2\n## M\n- [ ] c\n";
        add_project(&conn, tmp.path(), "Q", "open", content, &["b", "c"]);

        let entries = next_up(&conn).unwrap();

        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.milestone, None);
        assert_eq!(e.task_text.as_deref(), Some("b"));
        assert_eq!(e.task_hash, Some(task_hash("- [ ] b", 1)));
        assert_eq!((e.next_count, e.done, e.total), (1, 1, 2));
    }

    #[test]
    fn next_up_skips_done_tasks_and_archived_projects() {
        let (tmp, paths, conn, _todo, id) = project();
        select(&conn, &id, &task_hash("- [ ] Task one", 1)).unwrap();
        toggle(&conn, &paths, &id, &task_hash("- [ ] Task one", 1)).unwrap();
        // Selected while done: written outside the app.
        add_project(&conn, tmp.path(), "Done", "open", "## M\n- [x] a\n", &["a"]);
        add_project(
            &conn,
            tmp.path(),
            "Old",
            "archived",
            "## M\n- [ ] a\n",
            &["a"],
        );

        assert!(next_up(&conn).unwrap().is_empty());
    }

    #[test]
    fn next_up_orders_by_next_count_then_name() {
        let (tmp, _paths, conn, _todo, _id) = project();
        let two = "## M\n- [ ] a\n- [ ] b\n";
        add_project(&conn, tmp.path(), "charlie", "open", two, &["a"]);
        add_project(&conn, tmp.path(), "Bravo", "open", two, &["a"]);
        add_project(&conn, tmp.path(), "zulu", "open", two, &["a", "b"]);
        add_project(&conn, tmp.path(), "alpha", "open", two, &[]);

        let names: Vec<_> = next_up(&conn)
            .unwrap()
            .into_iter()
            .map(|e| e.project_name)
            .collect();
        assert_eq!(names, ["zulu", "Bravo", "charlie"]);
    }
}
