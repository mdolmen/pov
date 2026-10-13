//! TODO.md parsing, task hashing, checkbox toggling and task selection.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use rusqlite::{Connection, Transaction, TransactionBehavior};
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
    pub in_inbox: bool,
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

/// The hashes of a project's tasks marked `stage` (`next` or `inbox`).
fn hashes(conn: &Connection, project_id: &str, stage: &str) -> Result<HashSet<String>> {
    let mut stmt =
        conn.prepare("SELECT task_hash FROM selected_tasks WHERE project_id = ? AND stage = ?")?;
    let hashes = stmt.query_map([project_id, stage], |row| row.get(0))?;
    Ok(hashes.collect::<rusqlite::Result<_>>()?)
}

fn view(task: Task, next: &HashSet<String>, inbox: &HashSet<String>) -> TaskView {
    TaskView {
        is_done: task.is_done(),
        is_selected: next.contains(&task.hash),
        in_inbox: inbox.contains(&task.hash),
        hash: task.hash,
        text: task.text,
        checked: task.checked,
        line_number: task.line_number,
        subtasks: task.subtasks,
    }
}

/// Lists a project's headings and tasks, with their next / inbox marks.
pub fn list(conn: &Connection, project_id: &str) -> Result<Vec<ListItem>> {
    let (file_path, _) = projects::file_and_type(conn, project_id)?;
    let next = hashes(conn, project_id, "next")?;
    let inbox = hashes(conn, project_id, "inbox")?;
    Ok(parse_items(&file_path)
        .into_iter()
        .map(|item| match item {
            Item::Heading { text, level } => ListItem::Heading { text, level },
            Item::Task(task) => ListItem::Task(view(task, &next, &inbox)),
        })
        .collect())
}

/// Toggles a task or subtask in the project's file, commits the change to the data repo, and
/// clears its next / inbox mark if it is now done. Returns the task (the parent, for a subtask).
pub fn toggle(
    conn: &Connection,
    paths: &Paths,
    project_id: &str,
    task_hash: &str,
) -> Result<TaskView> {
    let (file_path, _) = projects::file_and_type(conn, project_id)?;
    // A toggle keeps every line: no splice.
    if !edit(conn, paths, project_id, |path| {
        Ok((toggle_cascade(path, task_hash)?, None))
    })? {
        return Err(Error::NotFound("task"));
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
    let inbox = hashes(conn, project_id, "inbox")?;
    Ok(view(task, &next, &inbox))
}

/// Commits an edit of the project's file to the data repo, through its hardlink.
fn commit_activity(paths: &Paths, project_id: &str, project_type: &str) {
    let hardlink = paths.hardlink(project_id, project_type);
    if hardlink.exists() {
        let name = hardlink.file_name().unwrap_or_default().to_string_lossy();
        git::add_commit_quiet(&paths.root, &hardlink, &format!("activity: {name}"));
    }
}

/// Each line of `text` normalized, with its hash when it is a checkbox line.
fn keyed_lines(text: &str) -> (Vec<String>, Vec<Option<String>>) {
    let text = universal_newlines(text);
    let mut seen = Occurrences::default();
    split_lines(&text)
        .into_iter()
        .map(|line| {
            (
                normalize(line),
                checkbox(line).is_some().then(|| seen.hash(line)),
            )
        })
        .unzip()
}

/// Old hash → new hash of the checkbox lines paired by `pairs` (old index, new index).
fn pair_hashes(
    old: &[Option<String>],
    new: &[Option<String>],
    pairs: impl Iterator<Item = (usize, usize)>,
) -> HashMap<String, String> {
    pairs
        .filter_map(|(i, j)| Some((old.get(i)?.clone()?, new.get(j)?.clone()?)))
        .collect()
}

/// Follows the lines an outside edit kept: a diff pairs the unchanged lines of `old` and `new`.
/// Changed lines aren't paired; a toggled checkbox counts as unchanged.
fn remap(old: &str, new: &str) -> HashMap<String, String> {
    let (old_keys, old_hashes) = keyed_lines(old);
    let (new_keys, new_hashes) = keyed_lines(new);
    let ops = similar::capture_diff_slices(similar::Algorithm::Myers, &old_keys, &new_keys);
    let pairs = ops.into_iter().flat_map(|op| match op {
        similar::DiffOp::Equal {
            old_index,
            new_index,
            len,
        } => (0..len).map(|k| (old_index + k, new_index + k)).collect(),
        _ => vec![],
    });
    pair_hashes(&old_hashes, &new_hashes, pairs)
}

/// `removed` lines at line `at` replaced by `inserted` lines.
struct Splice {
    at: usize,
    removed: usize,
    inserted: usize,
}

/// Follows the lines around a known splice: exact where a diff can't tell identical lines apart.
fn remap_splice(old: &str, new: &str, s: &Splice) -> HashMap<String, String> {
    let (_, old_hashes) = keyed_lines(old);
    let (_, new_hashes) = keyed_lines(new);
    let pairs = (0..s.at)
        .map(|i| (i, i))
        .chain((s.at + s.removed..old_hashes.len()).map(|i| (i, i - s.removed + s.inserted)));
    pair_hashes(&old_hashes, &new_hashes, pairs)
}

/// Moves the project's marks to their tasks' new hashes. A moved mark replaces a mark already
/// on its new hash.
fn apply_remap(conn: &Connection, project_id: &str, map: &HashMap<String, String>) -> Result<()> {
    let rows: Vec<(String, String, String)> = conn
        .prepare("SELECT task_hash, stage, created_at FROM selected_tasks WHERE project_id = ?")?
        .query_map([project_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    // Deleting first: chains like `#1 → #2, #2 → #3` would clash on the unique index.
    let moved: Vec<_> = rows
        .into_iter()
        .filter_map(|(hash, stage, created_at)| {
            let new = map.get(&hash).filter(|new| **new != hash)?;
            Some((hash, new.clone(), stage, created_at))
        })
        .collect();
    for (hash, ..) in &moved {
        conn.execute(
            "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ?",
            [project_id, hash],
        )?;
    }
    for (_, new, stage, created_at) in &moved {
        conn.execute(
            "INSERT OR REPLACE INTO selected_tasks (project_id, task_hash, stage, created_at)
             VALUES (?, ?, ?, ?)",
            [project_id, new, stage, created_at],
        )?;
    }
    Ok(())
}

/// Moves marks over the edits made since the data repo's last commit of the file (an editor,
/// another process). Projects without a hardlink have no history: their marks stay.
fn sync_uncommitted(
    conn: &Connection,
    paths: &Paths,
    project_id: &str,
    project_type: &str,
) -> Result<()> {
    let hardlink = paths.hardlink(project_id, project_type);
    let Ok(rel) = hardlink.strip_prefix(&paths.root) else {
        return Ok(());
    };
    let spec = format!("HEAD:{}", rel.to_string_lossy());
    let (Ok(old), Some(new)) = (
        git::run(&paths.root, &["show", &spec]),
        read_text(&hardlink),
    ) else {
        return Ok(());
    };
    apply_remap(conn, project_id, &remap(&old, &new))
}

/// Brings marks up to date with the project's file and commits it. Holds the DB write lock
/// throughout, so the watcher and other processes don't remap the same edit twice.
pub fn sync_marks(conn: &Connection, paths: &Paths, project_id: &str) -> Result<()> {
    edit(conn, paths, project_id, |_| Ok(((), None)))
}

/// [`sync_marks`] for every project; a project that fails is skipped.
pub fn sync_all(conn: &Connection, paths: &Paths) -> Result<()> {
    for row in projects::list_by_name(conn)? {
        let _ = sync_marks(conn, paths, &row.id);
    }
    Ok(())
}

/// Edits the project's file with `f`, which returns what it spliced, then moves the marks of
/// the lines that shifted and commits. Outside edits pending since the last commit are synced
/// first. The DB write lock is held throughout; an error rolls the marks back.
fn edit<T>(
    conn: &Connection,
    paths: &Paths,
    project_id: &str,
    f: impl FnOnce(&Path) -> Result<(T, Option<Splice>)>,
) -> Result<T> {
    let (file_path, project_type) = projects::file_and_type(conn, project_id)?;
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    sync_uncommitted(conn, paths, project_id, &project_type)?;
    let old = read_text(&file_path);
    let (out, splice) = f(&file_path)?;
    if let (Some(old), Some(new), Some(splice)) = (old, read_text(&file_path), splice) {
        apply_remap(conn, project_id, &remap_splice(&old, &new, &splice))?;
    }
    commit_activity(paths, project_id, &project_type);
    tx.commit()?;
    Ok(out)
}

/// The lines a [`delete`] removed, to put them back with [`restore`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Deleted {
    /// Index of the task's line in the file, as in [`Task::line_number`].
    pub line_number: usize,
    /// The removed lines, with their `\n` endings (the file's last line may have none).
    pub text: String,
}

/// Whether a line belongs to the block of the task above it: indented and not blank.
fn continues_block(line: &str) -> bool {
    line.starts_with([' ', '\t']) && !strip(line).is_empty()
}

/// Removes the top-level task `target` and its indented block (subtasks, notes) from `text`.
fn remove_task(text: &str, target: &str) -> Option<(String, Deleted)> {
    let task = tasks_of(text).into_iter().find(|t| t.hash == target)?;
    let lines = split_lines_keep_ends(text);
    let start = task.line_number;
    let end = start
        + 1
        + lines[start + 1..]
            .iter()
            .take_while(|l| continues_block(l))
            .count();
    let deleted = Deleted {
        line_number: start,
        text: lines[start..end].concat(),
    };
    let kept = [&lines[..start], &lines[end..]].concat().concat();
    Some((kept, deleted))
}

/// Inserts `block` before line `line_number` of `text`, or at the end when the file is shorter.
fn insert_lines(text: &str, line_number: usize, block: &str) -> String {
    let lines = split_lines_keep_ends(text);
    let at = line_number.min(lines.len());
    let mut before = lines[..at].concat();
    if !before.is_empty() && !before.ends_with('\n') {
        before.push('\n');
    }
    let mut block = block.to_string();
    if at < lines.len() && !block.ends_with('\n') {
        block.push('\n');
    }
    before + &block + &lines[at..].concat()
}

/// Reads a file as text with `\n` line endings; `true` when it used CRLF.
fn read_lf(path: &Path) -> Result<(String, bool)> {
    let raw = fs::read_to_string(path)?;
    Ok((universal_newlines(&raw), raw.contains("\r\n")))
}

/// Writes `text` over the file in place, so a hardlink to it keeps the change.
fn write_lf(path: &Path, text: &str, crlf: bool) -> Result<()> {
    fs::write(
        path,
        if crlf {
            text.replace('\n', "\r\n")
        } else {
            text.to_string()
        },
    )?;
    Ok(())
}

/// Deletes a top-level task and its indented block from the project's file, commits the change
/// to the data repo and clears the task's mark. Returns what was removed, for [`restore`].
pub fn delete(
    conn: &Connection,
    paths: &Paths,
    project_id: &str,
    task_hash: &str,
) -> Result<Deleted> {
    edit(conn, paths, project_id, |path| {
        let (text, crlf) = read_lf(path).map_err(|_| Error::NotFound("task"))?;
        let (kept, deleted) = remove_task(&text, task_hash).ok_or(Error::NotFound("task"))?;
        // Before the remap: a shifted duplicate may take this hash.
        conn.execute(
            "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ?",
            [project_id, task_hash],
        )?;
        write_lf(path, &kept, crlf)?;
        let splice = Splice {
            at: deleted.line_number,
            removed: split_lines_keep_ends(&deleted.text).len(),
            inserted: 0,
        };
        Ok((deleted, Some(splice)))
    })
}

/// Puts deleted lines back at `line_number` (at the end, if the file got shorter) and commits.
pub fn restore(
    conn: &Connection,
    paths: &Paths,
    project_id: &str,
    line_number: usize,
    text: &str,
) -> Result<()> {
    edit(conn, paths, project_id, |path| {
        let (current, crlf) = read_lf(path)?;
        write_lf(path, &insert_lines(&current, line_number, text), crlf)?;
        let splice = Splice {
            at: line_number.min(split_lines_keep_ends(&current).len()),
            removed: 0,
            inserted: split_lines_keep_ends(text).len(),
        };
        Ok(((), Some(splice)))
    })
}

/// A heading of a TODO.md: where [`add`] can put a task.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Section {
    pub text: String,
    pub level: usize,
}

/// The file's headings, in order.
pub fn sections(path: &Path) -> Vec<Section> {
    parse_items(path)
        .into_iter()
        .filter_map(|item| match item {
            Item::Heading { text, level } => Some(Section { text, level }),
            Item::Task(_) => None,
        })
        .collect()
}

/// The line a task appended to `section` goes at: after the section's last non-blank line, or
/// right after its heading when it is empty. A section runs to the next heading of the same or
/// a higher level; `None` is the whole file.
fn section_end(text: &str, section: Option<&str>) -> Result<usize> {
    let lines = split_lines(text);
    let heading = |line: &str| {
        HEADING_RE
            .captures(line)
            .map(|h| (strip(&h[2]).to_string(), h[1].len()))
    };
    let (start, end) = match section {
        None => (0, lines.len()),
        Some(name) => {
            let (at, level) = lines
                .iter()
                .enumerate()
                .find_map(|(i, line)| {
                    heading(line)
                        .filter(|(text, _)| text == name)
                        .map(|(_, level)| (i, level))
                })
                .ok_or_else(|| Error::Invalid(format!("no heading '{name}'")))?;
            let end = lines[at + 1..]
                .iter()
                .position(|line| heading(line).is_some_and(|(_, l)| l <= level))
                .map_or(lines.len(), |p| at + 1 + p);
            (at + 1, end)
        }
    };
    Ok(lines[start..end]
        .iter()
        .rposition(|line| !strip(line).is_empty())
        .map_or(start, |p| start + p + 1))
}

/// [`append_task`], also returning the new task's line.
fn append(path: &Path, text: &str, section: Option<&str>) -> Result<(String, usize)> {
    let task = strip(text);
    if task.is_empty() || split_lines(task).len() != 1 {
        return Err(Error::Invalid(
            "a task is one line of non-blank text".to_string(),
        ));
    }
    let (current, crlf) = read_lf(path)?;
    let at = section_end(&current, section)?;
    let new = insert_lines(&current, at, &format!("- [ ] {task}\n"));
    write_lf(path, &new, crlf)?;
    let hash = tasks_of(&new)
        .into_iter()
        .find(|t| t.line_number == at)
        .map(|t| t.hash)
        .ok_or(Error::NotFound("task"))?;
    Ok((hash, at))
}

/// Appends `- [ ] <text>` at the end of the section under heading `section` (exact text, any
/// level; `None`: the end of the file) and returns the new task's hash. Writes in place, so
/// the hardlink keeps the change, and keeps CRLF.
pub fn append_task(path: &Path, text: &str, section: Option<&str>) -> Result<String> {
    Ok(append(path, text, section)?.0)
}

/// Adds a task to a project's file (see [`append_task`]), commits it and marks it `stage`:
/// `inbox`, `next`, or `backlog` (no mark). Returns its hash.
pub fn add(
    conn: &Connection,
    paths: &Paths,
    project_id: &str,
    text: &str,
    section: Option<&str>,
    stage: &str,
) -> Result<String> {
    let (_, project_type) = projects::file_and_type(conn, project_id)?;
    if !["inbox", "backlog", "next"].contains(&stage) {
        return Err(Error::Invalid(format!("unknown stage '{stage}'")));
    }
    // A learning project's file may be a PDF.
    if project_type != "project" {
        return Err(Error::Invalid(
            "tasks can only be added to a project, not a learning project".to_string(),
        ));
    }
    let hash = edit(conn, paths, project_id, |path| {
        let (hash, at) = append(path, text, section)?;
        let splice = Splice {
            at,
            removed: 0,
            inserted: 1,
        };
        Ok((hash, Some(splice)))
    })?;
    // After the remap, which moves marks off the hashes the shift reused.
    match stage {
        "inbox" => inbox(conn, project_id, &hash)?,
        "next" => select(conn, project_id, &hash)?,
        _ => {}
    }
    Ok(hash)
}

/// Marks a task `stage` (`next` or `inbox`), replacing its other mark.
fn set_stage(conn: &Connection, project_id: &str, task_hash: &str, stage: &str) -> Result<()> {
    projects::file_and_type(conn, project_id)?;
    conn.execute(
        "INSERT INTO selected_tasks (project_id, task_hash, stage) VALUES (?, ?, ?)
         ON CONFLICT(project_id, task_hash) DO UPDATE SET stage = excluded.stage",
        [project_id, task_hash, stage],
    )?;
    Ok(())
}

/// Marks a task as selected ("next"), moving it out of the inbox. Selecting twice is a
/// no-op.
pub fn select(conn: &Connection, project_id: &str, task_hash: &str) -> Result<()> {
    set_stage(conn, project_id, task_hash, "next")
}

/// Puts a task in the inbox: added by an external tool, waiting for triage. A next task moves
/// back to the inbox.
pub fn inbox(conn: &Connection, project_id: &str, task_hash: &str) -> Result<()> {
    set_stage(conn, project_id, task_hash, "inbox")
}

/// Keeps an inbox task: it leaves the inbox for the backlog. A next task keeps its mark.
pub fn keep(conn: &Connection, project_id: &str, task_hash: &str) -> Result<()> {
    projects::file_and_type(conn, project_id)?;
    conn.execute(
        "DELETE FROM selected_tasks WHERE project_id = ? AND task_hash = ? AND stage = 'inbox'",
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

/// Removes the next / inbox marks of tasks that are done in the source file.
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

    /// The (next, inbox) marks of a task in the project's list.
    fn marks(conn: &Connection, id: &str, hash: &str) -> (bool, bool) {
        let items = list(conn, id).unwrap();
        let task = tasks_of_list(&items)
            .into_iter()
            .find(|t| t.hash == hash)
            .unwrap();
        (task.is_selected, task.in_inbox)
    }

    fn mark_rows(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM selected_tasks", (), |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn inbox_and_select_move_a_task_between_stages() {
        let (_tmp, _paths, conn, _todo, id) = project();
        let hash = task_hash("- [ ] Task one", 1);

        inbox(&conn, &id, &hash).unwrap();
        inbox(&conn, &id, &hash).unwrap();
        assert_eq!(marks(&conn, &id, &hash), (false, true));

        select(&conn, &id, &hash).unwrap();
        assert_eq!(marks(&conn, &id, &hash), (true, false));

        inbox(&conn, &id, &hash).unwrap();
        assert_eq!(marks(&conn, &id, &hash), (false, true));
        assert_eq!(mark_rows(&conn), 1);

        keep(&conn, &id, &hash).unwrap();
        assert_eq!(marks(&conn, &id, &hash), (false, false));
        assert_eq!(mark_rows(&conn), 0);
    }

    #[test]
    fn keep_leaves_a_next_task_alone() {
        let (_tmp, _paths, conn, _todo, id) = project();
        let hash = task_hash("- [ ] Task one", 1);
        select(&conn, &id, &hash).unwrap();

        keep(&conn, &id, &hash).unwrap();

        assert_eq!(marks(&conn, &id, &hash), (true, false));
    }

    #[test]
    fn inbox_unknown_project_is_not_found() {
        let (_tmp, _paths, conn, _todo, _id) = project();
        assert!(matches!(
            inbox(&conn, "nope", "h"),
            Err(Error::NotFound("project"))
        ));
        assert!(matches!(
            keep(&conn, "nope", "h"),
            Err(Error::NotFound("project"))
        ));
    }

    #[test]
    fn done_inbox_tasks_lose_their_mark() {
        let (_tmp, paths, conn, _todo, id) = project();
        let one = task_hash("- [ ] Task one", 1);
        inbox(&conn, &id, &one).unwrap();
        inbox(&conn, &id, &task_hash("- [x] Task two (done)", 1)).unwrap();

        cleanup_selected(&conn).unwrap();
        assert_eq!(mark_rows(&conn), 1);

        let task = toggle(&conn, &paths, &id, &one).unwrap();
        assert!(task.is_done && !task.in_inbox);
        assert_eq!(mark_rows(&conn), 0);
    }

    #[test]
    fn next_up_ignores_inbox_tasks() {
        let (tmp, _paths, conn, _todo, _id) = project();
        add_project(&conn, tmp.path(), "Q", "open", "## M\n- [ ] a\n", &[]);
        inbox(&conn, "Q", &task_hash("- [ ] a", 1)).unwrap();

        assert!(next_up(&conn).unwrap().is_empty());
    }

    /// Deletes task `line` (its first occurrence) from `text`, checks the result, then checks
    /// that restoring gives `text` back.
    fn delete_and_restore(text: &str, line: &str, expected: &str) -> Deleted {
        let (kept, deleted) = remove_task(text, &task_hash(line, 1)).unwrap();
        assert_eq!(kept, expected);
        assert_eq!(
            insert_lines(&kept, deleted.line_number, &deleted.text),
            text
        );
        deleted
    }

    #[test]
    fn remove_task_alone() {
        let deleted = delete_and_restore("# T\n- [ ] a\n- [ ] b\n", "- [ ] a", "# T\n- [ ] b\n");
        assert_eq!(
            deleted,
            Deleted {
                line_number: 1,
                text: "- [ ] a\n".into()
            }
        );
    }

    #[test]
    fn remove_task_takes_its_indented_block() {
        let text = "- [ ] a\n  - [ ] a1\n    note\n\n  - [x] later\n- [ ] b\n";
        let deleted = delete_and_restore(text, "- [ ] a", "\n  - [x] later\n- [ ] b\n");
        assert_eq!(deleted.text, "- [ ] a\n  - [ ] a1\n    note\n");
    }

    #[test]
    fn remove_task_on_the_last_line_without_newline() {
        delete_and_restore("- [ ] a\n- [ ] b", "- [ ] b", "- [ ] a\n");
        delete_and_restore("- [ ] a", "- [ ] a", "");
    }

    #[test]
    fn remove_task_picks_the_right_duplicate() {
        let text = "- [ ] a\n- [ ] b\n- [ ] a\n";
        let (kept, deleted) = remove_task(text, &task_hash("- [ ] a", 2)).unwrap();
        assert_eq!(kept, "- [ ] a\n- [ ] b\n");
        assert_eq!(deleted.line_number, 2);
    }

    #[test]
    fn remove_task_ignores_subtasks_and_unknown_hashes() {
        let text = "- [ ] a\n  - [ ] a1\n";
        assert!(remove_task(text, &task_hash("  - [ ] a1", 1)).is_none());
        assert!(remove_task(text, "nope").is_none());
    }

    #[test]
    fn insert_lines_past_the_end_appends() {
        assert_eq!(
            insert_lines("- [ ] a", 5, "- [ ] b\n"),
            "- [ ] a\n- [ ] b\n"
        );
        assert_eq!(
            insert_lines("- [ ] a\n", 0, "- [ ] b"),
            "- [ ] b\n- [ ] a\n"
        );
    }

    #[test]
    fn delete_edits_the_file_in_place_commits_and_clears_the_mark() {
        let (_tmp, paths, conn, todo, id) = project();
        let hash = task_hash("- [ ] Task one", 1);
        inbox(&conn, &id, &hash).unwrap();

        let deleted = delete(&conn, &paths, &id, &hash).unwrap();

        assert_eq!(deleted.text, "- [ ] Task one\n");
        let hardlink = paths.hardlink(&id, "project");
        assert_eq!(
            fs::read_to_string(&hardlink).unwrap(),
            "# My Project\n\n- [x] Task two (done)\n- [ ] Task three\n"
        );
        assert_eq!(
            fs::read_to_string(&todo).unwrap(),
            fs::read_to_string(&hardlink).unwrap()
        );
        assert_eq!(mark_rows(&conn), 0);
        let log = git::run(&paths.root, &["log", "-1", "--format=%s"]).unwrap();
        assert_eq!(log, "activity: p1.md\n");

        restore(&conn, &paths, &id, deleted.line_number, &deleted.text).unwrap();
        assert_eq!(fs::read_to_string(&hardlink).unwrap(), TODO);
    }

    #[test]
    fn delete_keeps_crlf_line_endings() {
        let (_tmp, paths, conn, todo, id) = project();
        fs::write(&todo, "- [ ] a\r\n  - [ ] a1\r\n- [ ] b\r\n").unwrap();

        let deleted = delete(&conn, &paths, &id, &task_hash("- [ ] a", 1)).unwrap();
        assert_eq!(fs::read_to_string(&todo).unwrap(), "- [ ] b\r\n");

        restore(&conn, &paths, &id, deleted.line_number, &deleted.text).unwrap();
        assert_eq!(
            fs::read_to_string(&todo).unwrap(),
            "- [ ] a\r\n  - [ ] a1\r\n- [ ] b\r\n"
        );
    }

    #[test]
    fn delete_unknown_task_is_not_found() {
        let (_tmp, paths, conn, todo, id) = project();
        assert!(matches!(
            delete(&conn, &paths, &id, "nope"),
            Err(Error::NotFound("task"))
        ));
        assert!(matches!(
            delete(&conn, &paths, "nope", "h"),
            Err(Error::NotFound("project"))
        ));
        assert_eq!(fs::read_to_string(&todo).unwrap(), TODO);
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

    fn h(line: &str, occurrence: usize) -> String {
        task_hash(line, occurrence)
    }

    /// The project's marks as (hash, stage, created_at), by hash.
    fn mark_list(conn: &Connection) -> Vec<(String, String, String)> {
        conn.prepare("SELECT task_hash, stage, created_at FROM selected_tasks ORDER BY task_hash")
            .unwrap()
            .query_map((), |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    fn last_subject(paths: &Paths) -> String {
        git::run(&paths.root, &["log", "-1", "--format=%s"]).unwrap()
    }

    #[test]
    fn remap_follows_a_duplicate_shifted_by_an_insert_or_delete() {
        let before = "## A\n## B\n- [ ] x\n";
        let after = "## A\n- [ ] x\n## B\n- [ ] x\n";
        assert_eq!(remap(before, after)[&h("- [ ] x", 1)], h("- [ ] x", 2));
        assert_eq!(remap(after, before)[&h("- [ ] x", 2)], h("- [ ] x", 1));
        assert!(!remap(after, before).contains_key(&h("- [ ] x", 1)));
    }

    #[test]
    fn remap_pairs_toggled_lines_and_drops_reworded_ones() {
        let map = remap("- [ ] a\n- [ ] b\n", "- [x] a2\n- [x] b\n");
        assert_eq!(map[&h("- [ ] b", 1)], h("- [ ] b", 1));
        assert!(!map.contains_key(&h("- [ ] a", 1)));
    }

    #[test]
    fn remap_splice_is_exact_next_to_identical_lines() {
        let splice = Splice {
            at: 1,
            removed: 0,
            inserted: 1,
        };
        let map = remap_splice("- [ ] x\n- [ ] x\n", "- [ ] x\n- [ ] x\n- [ ] x\n", &splice);
        assert_eq!(map[&h("- [ ] x", 1)], h("- [ ] x", 1));
        assert_eq!(map[&h("- [ ] x", 2)], h("- [ ] x", 3));
    }

    #[test]
    fn sync_marks_follows_an_editor_insert_and_commits() {
        let (_tmp, paths, conn, todo, id) = project();
        fs::write(&todo, "## A\n## B\n- [ ] x\n## C\n- [ ] x\n").unwrap();
        sync_marks(&conn, &paths, &id).unwrap();
        select(&conn, &id, &h("- [ ] x", 1)).unwrap();
        inbox(&conn, &id, &h("- [ ] x", 2)).unwrap();
        conn.execute("UPDATE selected_tasks SET created_at = '2000-01-01'", ())
            .unwrap();

        fs::write(&todo, "## A\n- [ ] x\n## B\n- [ ] x\n## C\n- [ ] x\n").unwrap();
        sync_marks(&conn, &paths, &id).unwrap();

        let mut expected = vec![
            (
                h("- [ ] x", 2),
                "next".to_string(),
                "2000-01-01".to_string(),
            ),
            (
                h("- [ ] x", 3),
                "inbox".to_string(),
                "2000-01-01".to_string(),
            ),
        ];
        expected.sort();
        assert_eq!(mark_list(&conn), expected);
        assert_eq!(last_subject(&paths), "activity: p1.md\n");

        // Already committed: nothing moves twice.
        sync_marks(&conn, &paths, &id).unwrap();
        assert_eq!(mark_list(&conn), expected);
    }

    #[test]
    fn sync_marks_without_history_or_hardlink_keeps_marks() {
        let (_tmp, paths, conn, todo, id) = project();
        fs::write(&todo, "## A\n## B\n- [ ] x\n").unwrap();
        select(&conn, &id, &h("- [ ] x", 1)).unwrap();
        fs::write(&todo, "## A\n- [ ] x\n## B\n- [ ] x\n").unwrap();
        sync_marks(&conn, &paths, &id).unwrap();
        assert_eq!(mark_list(&conn)[0].0, h("- [ ] x", 1));

        fs::remove_file(paths.hardlink(&id, "project")).unwrap();
        fs::write(&todo, "## A\n## B\n- [ ] x\n").unwrap();
        sync_marks(&conn, &paths, &id).unwrap();
        assert_eq!(mark_list(&conn)[0].0, h("- [ ] x", 1));
    }

    #[test]
    fn delete_and_restore_keep_a_duplicate_s_mark() {
        let (_tmp, paths, conn, todo, id) = project();
        fs::write(&todo, "- [ ] x\n- [ ] x\n").unwrap();
        select(&conn, &id, &h("- [ ] x", 2)).unwrap();

        let deleted = delete(&conn, &paths, &id, &h("- [ ] x", 1)).unwrap();
        assert_eq!(mark_list(&conn)[0].0, h("- [ ] x", 1));

        restore(&conn, &paths, &id, deleted.line_number, &deleted.text).unwrap();
        assert_eq!(mark_list(&conn).len(), 1);
        assert_eq!(mark_list(&conn)[0].0, h("- [ ] x", 2));
    }

    /// Appends to a file holding `before`, checks the result is `after`, returns the hash.
    fn appended(before: &str, task: &str, section: Option<&str>, after: &str) -> String {
        let tmp = tempfile::tempdir().unwrap();
        let path = file(tmp.path(), before);
        let hash = append_task(&path, task, section).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), after);
        hash
    }

    #[test]
    fn append_task_at_the_end_of_the_file() {
        appended(
            "# T\n- [ ] a\n\n\n",
            "b",
            None,
            "# T\n- [ ] a\n- [ ] b\n\n\n",
        );
        appended("", "b", None, "- [ ] b\n");
        appended("- [ ] a", " b ", None, "- [ ] a\n- [ ] b\n");
    }

    #[test]
    fn append_task_at_the_end_of_a_section() {
        let before = "## A\n- [ ] a\n\n## B\n- [ ] b\n";
        appended(
            before,
            "x",
            Some("A"),
            "## A\n- [ ] a\n- [ ] x\n\n## B\n- [ ] b\n",
        );

        let before = "## A\n### A1\n- [ ] a1\n\n## B\n";
        let after = "## A\n### A1\n- [ ] a1\n- [ ] x\n\n## B\n";
        appended(before, "x", Some("A"), after);

        appended("## A\n\n## B\n", "x", Some("A"), "## A\n- [ ] x\n\n## B\n");
    }

    #[test]
    fn append_task_keeps_crlf() {
        let before = "## A\r\n- [ ] a\r\n## B\r\n";
        appended(
            before,
            "x",
            Some("A"),
            "## A\r\n- [ ] a\r\n- [ ] x\r\n## B\r\n",
        );
    }

    #[test]
    fn append_task_gives_a_duplicate_its_own_hash() {
        let hash = appended("- [ ] x\n", "x", None, "- [ ] x\n- [ ] x\n");
        assert_eq!(hash, h("- [ ] x", 2));
    }

    #[test]
    fn append_task_rejects_bad_text_and_unknown_headings() {
        let tmp = tempfile::tempdir().unwrap();
        let path = file(tmp.path(), TODO);
        for (text, section) in [
            ("", None),
            ("  ", None),
            ("a\nb", None),
            ("a\rb", None),
            ("a", Some("Nope")),
        ] {
            assert!(matches!(
                append_task(&path, text, section),
                Err(Error::Invalid(_))
            ));
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), TODO);
    }

    #[test]
    fn sections_lists_headings_with_levels() {
        let tmp = tempfile::tempdir().unwrap();
        let sections = sections(&file(tmp.path(), MIXED_CONTENT));
        let expected = [("Section one", 1), ("Section two", 2)];
        let got: Vec<_> = sections
            .iter()
            .map(|s| (s.text.as_str(), s.level))
            .collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn add_writes_through_the_hardlink_commits_and_marks() {
        use std::os::unix::fs::MetadataExt;
        let (_tmp, paths, conn, todo, id) = project();
        let hardlink = paths.hardlink(&id, "project");

        let inboxed = add(&conn, &paths, &id, "In", None, "inbox").unwrap();
        let next = add(&conn, &paths, &id, "Next", None, "next").unwrap();
        let backlog = add(&conn, &paths, &id, "Later", None, "backlog").unwrap();

        assert_eq!(
            fs::read_to_string(&hardlink).unwrap(),
            format!("{TODO}- [ ] In\n- [ ] Next\n- [ ] Later\n")
        );
        assert_eq!(
            fs::metadata(&todo).unwrap().ino(),
            fs::metadata(&hardlink).unwrap().ino()
        );
        assert_eq!(last_subject(&paths), "activity: p1.md\n");
        assert_eq!(marks(&conn, &id, &inboxed), (false, true));
        assert_eq!(marks(&conn, &id, &next), (true, false));
        assert_eq!(marks(&conn, &id, &backlog), (false, false));
    }

    #[test]
    fn add_rejects_bad_stages_and_learning_projects() {
        let (_tmp, paths, conn, todo, id) = project();
        assert!(matches!(
            add(&conn, &paths, &id, "x", None, "later"),
            Err(Error::Invalid(_))
        ));
        conn.execute("UPDATE projects SET type = 'learning'", ())
            .unwrap();
        assert!(matches!(
            add(&conn, &paths, &id, "x", None, "inbox"),
            Err(Error::Invalid(_))
        ));
        assert_eq!(fs::read_to_string(&todo).unwrap(), TODO);
    }

    #[test]
    fn add_above_a_marked_duplicate_keeps_its_mark() {
        let (_tmp, paths, conn, todo, id) = project();
        fs::write(&todo, "## A\n- [ ] x\n## B\n- [ ] x\n").unwrap();
        select(&conn, &id, &h("- [ ] x", 1)).unwrap();
        select(&conn, &id, &h("- [ ] x", 2)).unwrap();

        let hash = add(&conn, &paths, &id, "x", Some("A"), "inbox").unwrap();

        assert_eq!(hash, h("- [ ] x", 2));
        assert_eq!(marks(&conn, &id, &h("- [ ] x", 1)), (true, false));
        assert_eq!(marks(&conn, &id, &hash), (false, true));
        assert_eq!(marks(&conn, &id, &h("- [ ] x", 3)), (true, false));
        assert_eq!(mark_rows(&conn), 3);
    }
}
