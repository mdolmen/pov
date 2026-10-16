//! Tracked projects: the DB row, the hardlink in the data repo, and the config.json entry.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::activity::{self, Level};
use crate::{Error, Paths, Result, git, tasks};

const COLUMNS: &str = "id, name, file_path, status, sub_status, type, has_hardlink, paused_until";

/// A `projects` row.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub file_path: PathBuf,
    pub status: String,
    pub sub_status: Option<String>,
    pub project_type: String,
    pub has_hardlink: bool,
    pub paused_until: Option<String>,
}

/// A project as the frontend sees it (`Project` in `src/types.ts`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub status: String,
    pub sub_status: Option<String>,
    #[serde(rename = "type")]
    pub project_type: String,
    pub has_hardlink: bool,
    pub task_count: usize,
    pub selected_count: i64,
    pub inbox_count: i64,
    pub delegated_count: i64,
    pub activity: Level,
    pub paused_until: Option<String>,
    /// The tools tasks can be delegated to, from `config.json`.
    pub tools: Vec<String>,
}

/// A tool tasks are delegated to, from `config.json` `tools`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Tool {
    /// Absolute path: the app has no shell `PATH`.
    pub command: PathBuf,
    /// Holds the pid of the running tool.
    pub pidfile: PathBuf,
}

/// A project to start tracking.
#[derive(Debug, Clone)]
pub struct NewProject {
    pub name: String,
    pub file_path: PathBuf,
    pub project_type: String,
    pub status: String,
    pub sub_status: Option<String>,
}

/// Fields to change on a project; `None` leaves a field as is.
#[derive(Debug, Clone, Default)]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub status: Option<String>,
    /// `Some(None)` clears it.
    pub sub_status: Option<Option<String>>,
    /// `Some(None)` or `Some(Some(""))` clears it.
    pub paused_until: Option<Option<String>>,
}

fn row_of(row: &Row) -> rusqlite::Result<ProjectRow> {
    Ok(ProjectRow {
        id: row.get(0)?,
        name: row.get(1)?,
        file_path: PathBuf::from(row.get::<_, String>(2)?),
        status: row.get(3)?,
        sub_status: row.get(4)?,
        project_type: row.get(5)?,
        has_hardlink: row.get(6)?,
        paused_until: row.get(7)?,
    })
}

fn check(field: &str, value: &str, allowed: &[&str]) -> Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "{field} must be one of {allowed:?}, got '{value}'"
        )))
    }
}

fn check_sub_status(sub_status: Option<&str>) -> Result<()> {
    sub_status.map_or(Ok(()), |s| {
        check("sub_status", s, &["paused", "done", "canceled"])
    })
}

/// A project's tracked file and type, or `NotFound` when the project doesn't exist.
pub(crate) fn file_and_type(conn: &Connection, project_id: &str) -> Result<(PathBuf, String)> {
    let row = find_row(conn, project_id)?;
    Ok((row.file_path, row.project_type))
}

fn find_row(conn: &Connection, project_id: &str) -> Result<ProjectRow> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM projects WHERE id = ?"),
        [project_id],
        row_of,
    )
    .optional()?
    .ok_or(Error::NotFound("project"))
}

fn summary(
    conn: &Connection,
    paths: &Paths,
    row: ProjectRow,
    tools: Vec<String>,
) -> Result<Project> {
    // Marks count only on tasks still open in the file, like Next up: a task deleted or checked
    // in an editor leaves its mark behind.
    let open: HashSet<String> = tasks::parse_tasks(&row.file_path)
        .into_iter()
        .filter(|t| !t.is_done())
        .map(|t| t.hash)
        .collect();
    let (mut selected_count, mut inbox_count, mut delegated_count) = (0, 0, 0);
    let mut stmt =
        conn.prepare("SELECT task_hash, stage FROM selected_tasks WHERE project_id = ?")?;
    let marks = stmt.query_map([&row.id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    for mark in marks {
        let (hash, stage) = mark?;
        if open.contains(&hash) {
            match stage.as_str() {
                "next" => selected_count += 1,
                "inbox" => inbox_count += 1,
                "delegated" => delegated_count += 1,
                _ => {}
            }
        }
    }
    // Time recorded on a project counts as activity: reading a PDF or working off-file leaves
    // no trace in git, but the user did work on it.
    let last_time_entry: Option<String> = conn.query_row(
        "SELECT MAX(date) FROM time_entries WHERE project_id = ?",
        [&row.id],
        |r| r.get(0),
    )?;
    let hardlink = row
        .has_hardlink
        .then(|| paths.hardlink(&row.id, &row.project_type));
    let mut level = activity::level(paths, &row.file_path, hardlink.as_deref());
    if let Some(day) = last_time_entry {
        level = activity::most_recent(level, activity::classify_day(&day));
    }

    Ok(Project {
        task_count: open.len(),
        file_path: row.file_path.to_string_lossy().into_owned(),
        id: row.id,
        name: row.name,
        status: row.status,
        sub_status: row.sub_status,
        project_type: row.project_type,
        has_hardlink: row.has_hardlink,
        selected_count,
        inbox_count,
        delegated_count,
        activity: level,
        paused_until: row.paused_until,
        tools,
    })
}

/// All projects, most recently added first, with their counts and activity.
pub fn list(conn: &Connection, paths: &Paths) -> Result<Vec<Project>> {
    let rows: Vec<ProjectRow> = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM projects ORDER BY created_at DESC"
        ))?
        .query_map((), row_of)?
        .collect::<rusqlite::Result<_>>()?;
    let mut linked = config_tools(paths)?.linked;
    rows.into_iter()
        .map(|row| {
            let tools = linked.remove(&row.id).unwrap_or_default();
            summary(conn, paths, row, tools)
        })
        .collect()
}

/// One project, with its counts and activity.
pub fn get(conn: &Connection, paths: &Paths, project_id: &str) -> Result<Project> {
    let row = find_row(conn, project_id)?;
    summary(conn, paths, row, tools(paths, project_id)?)
}

/// All project rows, by name.
pub fn list_by_name(conn: &Connection) -> Result<Vec<ProjectRow>> {
    let rows = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM projects ORDER BY name COLLATE NOCASE"
        ))?
        .query_map((), row_of)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// The project with exactly this name.
pub fn find_by_name(conn: &Connection, name: &str) -> Result<ProjectRow> {
    let mut rows: Vec<ProjectRow> = conn
        .prepare(&format!("SELECT {COLUMNS} FROM projects WHERE name = ?"))?
        .query_map([name], row_of)?
        .collect::<rusqlite::Result<_>>()?;
    match rows.len() {
        0 => Err(Error::NotFound("project")),
        1 => Ok(rows.remove(0)),
        _ => Err(Error::AmbiguousProject(name.to_string())),
    }
}

/// The project with id `query`, else the one named `query` in any case. A miss or several
/// matches is `Invalid`, naming the candidates.
pub fn resolve(conn: &Connection, query: &str) -> Result<ProjectRow> {
    match find_row(conn, query) {
        Err(Error::NotFound(_)) => {}
        found => return found,
    }
    let mut rows: Vec<ProjectRow> = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM projects WHERE name = ? COLLATE NOCASE ORDER BY name"
        ))?
        .query_map([query], row_of)?
        .collect::<rusqlite::Result<_>>()?;
    match rows.len() {
        1 => Ok(rows.remove(0)),
        0 => {
            let names: Vec<_> = list_by_name(conn)?.into_iter().map(|r| r.name).collect();
            Err(Error::Invalid(format!(
                "no project '{query}'; projects: {}",
                names.join(", ")
            )))
        }
        _ => {
            let matches: Vec<_> = rows
                .iter()
                .map(|r| format!("{} ({})", r.id, r.name))
                .collect();
            Err(Error::Invalid(format!(
                "several projects named '{query}': {}",
                matches.join(", ")
            )))
        }
    }
}

/// `true` when the hardlink was made, `false` when the file is on another filesystem.
fn hardlinked(result: io::Result<()>) -> Result<bool> {
    match result {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn expand_user(path: &Path) -> PathBuf {
    match (path.strip_prefix("~"), std::env::var_os("HOME")) {
        (Ok(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => path.to_path_buf(),
    }
}

fn read_config(paths: &Paths) -> Result<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(&paths.config)?)?)
}

/// The `config.json` tools.
struct ConfigTools {
    /// `tools`, by name.
    defined: HashMap<String, Tool>,
    /// The tools each project links to, by project id.
    linked: HashMap<String, Vec<String>>,
}

/// Reads the tools of `config.json`. A project's tool names missing from `tools` are ignored,
/// like malformed entries.
fn config_tools(paths: &Paths) -> Result<ConfigTools> {
    let config = read_config(paths)?;
    let defined: HashMap<String, Tool> = config
        .get("tools")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(name, tool)| Some((name.clone(), Tool::deserialize(tool).ok()?)))
        .collect();
    let linked = config
        .get("projects")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let id = p.get("id")?.as_str()?.to_string();
            let names = p
                .get("tools")?
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .filter(|name| defined.contains_key(*name))
                .map(String::from)
                .collect();
            Some((id, names))
        })
        .collect();
    Ok(ConfigTools { defined, linked })
}

/// The tools a project links to in `config.json`.
pub fn tools(paths: &Paths, project_id: &str) -> Result<Vec<String>> {
    Ok(config_tools(paths)?
        .linked
        .remove(project_id)
        .unwrap_or_default())
}

/// A tool defined in `config.json`, or `NotFound`.
pub fn tool(paths: &Paths, name: &str) -> Result<Tool> {
    config_tools(paths)?
        .defined
        .remove(name)
        .ok_or(Error::NotFound("tool"))
}

fn write_config(paths: &Paths, config: &Value) -> Result<()> {
    fs::write(&paths.config, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

/// Starts tracking a file: hardlink into the data repo (committed), DB row, config entry.
///
/// Files on another filesystem can't be hardlinked; they're tracked without one. Returns the
/// new project's id.
pub fn add(conn: &Connection, paths: &Paths, project: &NewProject) -> Result<String> {
    check("type", &project.project_type, &["project", "learning"])?;
    check("status", &project.status, &["open", "archived"])?;
    check_sub_status(project.sub_status.as_deref())?;

    let given = expand_user(&project.file_path);
    let src = fs::canonicalize(&given).map_err(|_| Error::FileNotFound(given))?;
    let src_str = src.to_string_lossy().into_owned();
    let tracked = conn
        .query_row(
            "SELECT 1 FROM projects WHERE file_path = ?",
            [&src_str],
            |_| Ok(()),
        )
        .optional()?;
    if tracked.is_some() {
        return Err(Error::ProjectExists(src));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let dst = paths.hardlink(&id, &project.project_type);
    let has_hardlink = hardlinked(fs::hard_link(&src, &dst))?;
    if has_hardlink {
        git::add_commit(&paths.root, &dst, &format!("add: {}", project.name))?;
    }

    conn.execute(
        "INSERT INTO projects (id, name, file_path, status, sub_status, has_hardlink, type)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        (
            &id,
            &project.name,
            &src_str,
            &project.status,
            &project.sub_status,
            has_hardlink,
            &project.project_type,
        ),
    )?;

    let mut config = read_config(paths)?;
    if let Some(obj) = config.as_object_mut() {
        let projects = obj.entry("projects").or_insert_with(|| json!([]));
        if let Some(list) = projects.as_array_mut() {
            list.push(json!({"id": id, "name": project.name, "path": src_str}));
        }
    }
    write_config(paths, &config)?;
    Ok(id)
}

/// Stops tracking a project: hardlink, DB row and config entry. The original file stays.
pub fn remove(conn: &Connection, paths: &Paths, project_id: &str) -> Result<()> {
    let row = find_row(conn, project_id)?;
    let dst = paths.hardlink(project_id, &row.project_type);
    if dst.exists() {
        fs::remove_file(&dst)?;
    }

    conn.execute("DELETE FROM projects WHERE id = ?", [project_id])?;

    let mut config = read_config(paths)?;
    if let Some(list) = config.get_mut("projects").and_then(Value::as_array_mut) {
        list.retain(|p| p.get("id").and_then(Value::as_str) != Some(project_id));
    }
    write_config(paths, &config)
}

/// Changes a project's name, status, sub-status or pause date, and returns it.
pub fn update(
    conn: &Connection,
    paths: &Paths,
    project_id: &str,
    changes: &ProjectUpdate,
) -> Result<Project> {
    find_row(conn, project_id)?;
    if let Some(status) = &changes.status {
        check("status", status, &["open", "archived"])?;
    }
    if let Some(sub_status) = &changes.sub_status {
        check_sub_status(sub_status.as_deref())?;
    }

    let tx = conn.unchecked_transaction()?;
    let set = |column: &str, value: Option<&str>| {
        tx.execute(
            &format!("UPDATE projects SET {column} = ? WHERE id = ?"),
            (value, project_id),
        )
    };
    if let Some(name) = &changes.name {
        set("name", Some(name))?;
    }
    if let Some(status) = &changes.status {
        set("status", Some(status))?;
    }
    if let Some(sub_status) = &changes.sub_status {
        set("sub_status", sub_status.as_deref())?;
    }
    if let Some(paused_until) = &changes.paused_until {
        set(
            "paused_until",
            paused_until.as_deref().filter(|d| !d.is_empty()),
        )?;
    }
    tx.commit()?;

    get(conn, paths, project_id)
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::time::{Duration, SystemTime};

    use super::*;
    use crate::{db, storage, timelog};

    struct Env {
        tmp: tempfile::TempDir,
        paths: Paths,
        conn: Connection,
        todo: PathBuf,
    }

    impl Env {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let paths = Paths::new(tmp.path().join("pov"));
            storage::init(&paths).unwrap();
            db::init(&paths).unwrap();
            let conn = db::open(&paths).unwrap();
            let todo = tmp.path().join("TODO.md");
            fs::write(
                &todo,
                "# My Project\n\n- [ ] Task one\n- [x] Task two (done)\n- [ ] Task three\n",
            )
            .unwrap();
            Self {
                tmp,
                paths,
                conn,
                todo,
            }
        }

        fn add(&self, name: &str, file_path: &Path, project_type: &str) -> Result<String> {
            let project = NewProject {
                name: name.into(),
                file_path: file_path.into(),
                project_type: project_type.into(),
                status: "open".into(),
                sub_status: None,
            };
            add(&self.conn, &self.paths, &project)
        }

        fn pdf(&self) -> PathBuf {
            let pdf = self.tmp.path().join("book.pdf");
            fs::write(&pdf, b"%PDF-1.4\n\xe2\x28\xa1 binary\n").unwrap();
            pdf
        }

        fn config_ids(&self) -> Vec<String> {
            let config = read_config(&self.paths).unwrap();
            config["projects"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["id"].as_str().unwrap().to_string())
                .collect()
        }

        /// Defines tools `a` and `b` and links project `id` to `tools`, as edited by hand.
        fn link_tools(&self, id: &str, tools: Value) {
            let mut config = read_config(&self.paths).unwrap();
            config["tools"] = json!({
                "a": {"command": "/bin/a", "pidfile": "/tmp/a.pid"},
                "b": {"command": "/bin/b", "pidfile": "/tmp/b.pid"},
                "bad": {"command": "/bin/bad"},
            });
            for p in config["projects"].as_array_mut().unwrap() {
                if p["id"] == id {
                    p["tools"] = tools.clone();
                }
            }
            write_config(&self.paths, &config).unwrap();
        }
    }

    #[test]
    fn list_is_empty_at_first() {
        let env = Env::new();
        assert!(list(&env.conn, &env.paths).unwrap().is_empty());
    }

    #[test]
    fn add_links_commits_and_records_the_project() {
        let env = Env::new();
        let id = env.add("My Project", &env.todo, "project").unwrap();

        let project = get(&env.conn, &env.paths, &id).unwrap();
        assert_eq!(project.name, "My Project");
        assert_eq!(project.status, "open");
        assert_eq!(project.task_count, 2);
        assert_eq!(project.selected_count, 0);
        assert!(project.has_hardlink);
        assert!(env.paths.hardlink(&id, "project").exists());
        assert_eq!(env.config_ids(), [id]);
        let log = git::run(&env.paths.root, &["log", "--format=%s"]).unwrap();
        assert!(log.contains("add: My Project"), "{log}");
    }

    #[test]
    fn counts_next_and_inbox_tasks_apart() {
        let env = Env::new();
        let id = env.add("P", &env.todo, "project").unwrap();
        tasks::select(&env.conn, &id, &tasks::task_hash("- [ ] Task one", 1)).unwrap();
        tasks::inbox(&env.conn, &id, &tasks::task_hash("- [ ] Task three", 1)).unwrap();
        tasks::inbox(
            &env.conn,
            &id,
            &tasks::task_hash("- [x] Task two (done)", 1),
        )
        .unwrap();

        // Marks of tasks no longer in the file don't count.
        tasks::select(&env.conn, &id, &tasks::task_hash("- [ ] Deleted", 1)).unwrap();

        let project = get(&env.conn, &env.paths, &id).unwrap();

        // The done task's mark doesn't count either, as in Next up.
        assert_eq!((project.selected_count, project.inbox_count), (1, 1));
    }

    #[test]
    fn tools_come_from_config() {
        let env = Env::new();
        let id = env.add("P", &env.todo, "project").unwrap();
        assert!(get(&env.conn, &env.paths, &id).unwrap().tools.is_empty());

        env.link_tools(&id, json!(["b", "ghost", "bad", "a"]));

        assert_eq!(get(&env.conn, &env.paths, &id).unwrap().tools, ["b", "a"]);
        assert_eq!(list(&env.conn, &env.paths).unwrap()[0].tools, ["b", "a"]);
        assert_eq!(
            tool(&env.paths, "a").unwrap(),
            Tool {
                command: "/bin/a".into(),
                pidfile: "/tmp/a.pid".into()
            }
        );
        assert!(matches!(
            tool(&env.paths, "ghost"),
            Err(Error::NotFound("tool"))
        ));
        assert!(matches!(
            tool(&env.paths, "bad"),
            Err(Error::NotFound("tool"))
        ));
    }

    #[test]
    fn add_and_remove_keep_hand_written_tools() {
        let env = Env::new();
        let id = env.add("P", &env.todo, "project").unwrap();
        env.link_tools(&id, json!(["a"]));

        let other = env.add("Book", &env.pdf(), "learning").unwrap();
        remove(&env.conn, &env.paths, &other).unwrap();

        assert_eq!(tools(&env.paths, &id).unwrap(), ["a"]);
        assert!(tool(&env.paths, "b").is_ok());
    }

    #[test]
    fn counts_open_delegated_tasks() {
        let env = Env::new();
        let id = env.add("P", &env.todo, "project").unwrap();
        for line in ["- [ ] Task one", "- [x] Task two (done)"] {
            env.conn
                .execute(
                    "INSERT INTO selected_tasks (project_id, task_hash, stage)
                     VALUES (?, ?, 'delegated')",
                    [&id, &tasks::task_hash(line, 1)],
                )
                .unwrap();
        }

        let project = get(&env.conn, &env.paths, &id).unwrap();

        assert_eq!(
            (
                project.selected_count,
                project.inbox_count,
                project.delegated_count
            ),
            (0, 0, 1)
        );
    }

    #[test]
    fn add_keeps_config_key_order() {
        let env = Env::new();
        env.add("P", &env.todo, "project").unwrap();
        let config = fs::read_to_string(&env.paths.config).unwrap();
        assert!(
            config.starts_with("{\n  \"projects\": [\n    {\n      \"id\""),
            "{config}"
        );
        assert!(config.ends_with("  \"learning\": {}\n}"), "{config}");
    }

    #[test]
    fn add_missing_file_is_an_error() {
        let env = Env::new();
        let err = env
            .add("X", Path::new("/nonexistent/TODO.md"), "project")
            .unwrap_err();
        assert!(matches!(err, Error::FileNotFound(_)));
    }

    #[test]
    fn add_twice_is_an_error_without_side_effects() {
        let env = Env::new();
        env.add("P", &env.todo, "project").unwrap();
        let err = env.add("Q", &env.todo, "project").unwrap_err();
        assert!(matches!(err, Error::ProjectExists(_)));
        assert_eq!(env.config_ids().len(), 1);
        assert_eq!(fs::read_dir(&env.paths.projects).unwrap().count(), 1);
    }

    #[test]
    fn cross_device_link_means_no_hardlink() {
        let exdev = io::Error::from(io::ErrorKind::CrossesDevices);
        assert!(!hardlinked(Err(exdev)).unwrap());
        assert!(hardlinked(Ok(())).unwrap());
        assert!(hardlinked(Err(io::Error::from(io::ErrorKind::PermissionDenied))).is_err());
    }

    #[test]
    fn recorded_time_counts_as_activity() {
        let env = Env::new();
        let pdf = env.pdf();
        let old = SystemTime::now() - Duration::from_secs(90 * 86_400);
        File::options()
            .write(true)
            .open(&pdf)
            .unwrap()
            .set_modified(old)
            .unwrap();
        let id = env.add("Book", &pdf, "learning").unwrap();
        assert_eq!(
            get(&env.conn, &env.paths, &id).unwrap().activity,
            Level::Older
        );

        timelog::add_entry(&env.conn, &id, 60, None, None).unwrap();
        assert_eq!(
            get(&env.conn, &env.paths, &id).unwrap().activity,
            Level::ThisWeek
        );
    }

    #[test]
    fn recorded_time_does_not_downgrade_file_activity() {
        let env = Env::new();
        let id = env.add("P", &env.todo, "project").unwrap();
        timelog::add_entry(&env.conn, &id, 60, Some("2020-01-01"), None).unwrap();
        assert_eq!(
            get(&env.conn, &env.paths, &id).unwrap().activity,
            Level::ThisWeek
        );
    }

    #[test]
    fn list_tolerates_a_binary_file() {
        let env = Env::new();
        env.add("P", &env.todo, "project").unwrap();
        env.add("Book", &env.pdf(), "learning").unwrap();

        let projects = list(&env.conn, &env.paths).unwrap();
        let book = projects.iter().find(|p| p.name == "Book").unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(book.task_count, 0);
    }

    #[test]
    fn remove_unlinks_and_forgets_but_keeps_the_original() {
        let env = Env::new();
        let id = env.add("P", &env.todo, "project").unwrap();
        let hardlink = env.paths.hardlink(&id, "project");

        remove(&env.conn, &env.paths, &id).unwrap();

        assert!(!hardlink.exists());
        assert!(env.todo.exists());
        assert!(env.config_ids().is_empty());
        assert!(list(&env.conn, &env.paths).unwrap().is_empty());
    }

    #[test]
    fn unknown_project_is_not_found() {
        let env = Env::new();
        let rename = ProjectUpdate {
            name: Some("X".into()),
            ..Default::default()
        };
        assert!(matches!(
            remove(&env.conn, &env.paths, "nope"),
            Err(Error::NotFound(_))
        ));
        assert!(matches!(
            update(&env.conn, &env.paths, "nope", &rename),
            Err(Error::NotFound(_))
        ));
        assert!(matches!(
            find_by_name(&env.conn, "nope"),
            Err(Error::NotFound(_))
        ));
    }

    #[test]
    fn find_by_name_rejects_ambiguous_names() {
        let env = Env::new();
        env.add("Same", &env.todo, "project").unwrap();
        env.add("Same", &env.pdf(), "learning").unwrap();
        assert!(matches!(
            find_by_name(&env.conn, "Same"),
            Err(Error::AmbiguousProject(_))
        ));
    }

    #[test]
    fn update_name_status_and_pause() {
        let env = Env::new();
        let id = env.add("Old", &env.todo, "project").unwrap();

        let changes = ProjectUpdate {
            name: Some("New".into()),
            status: Some("archived".into()),
            sub_status: Some(Some("done".into())),
            paused_until: Some(Some("2026-12-01".into())),
        };
        let project = update(&env.conn, &env.paths, &id, &changes).unwrap();
        assert_eq!(project.name, "New");
        assert_eq!(project.status, "archived");
        assert_eq!(project.sub_status.as_deref(), Some("done"));
        assert_eq!(project.paused_until.as_deref(), Some("2026-12-01"));

        let clear = ProjectUpdate {
            paused_until: Some(Some(String::new())),
            ..Default::default()
        };
        let project = update(&env.conn, &env.paths, &id, &clear).unwrap();
        assert_eq!(project.paused_until, None);
        assert_eq!(project.name, "New");
    }

    #[test]
    fn update_rejects_unknown_status() {
        let env = Env::new();
        let id = env.add("P", &env.todo, "project").unwrap();
        let changes = ProjectUpdate {
            status: Some("closed".into()),
            ..Default::default()
        };
        assert!(matches!(
            update(&env.conn, &env.paths, &id, &changes),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn resolve_by_id_then_by_name_in_any_case() {
        let env = Env::new();
        let id = env.add("Pov", &env.todo, "project").unwrap();
        assert_eq!(resolve(&env.conn, &id).unwrap().id, id);
        assert_eq!(resolve(&env.conn, "pOV").unwrap().id, id);
    }

    #[test]
    fn resolve_names_the_candidates_on_a_miss_or_a_tie() {
        let env = Env::new();
        let a = env.add("Same", &env.todo, "project").unwrap();
        let b = env.add("same", &env.pdf(), "learning").unwrap();
        let Err(Error::Invalid(tie)) = resolve(&env.conn, "SAME") else {
            panic!("expected a tie");
        };
        assert!(tie.contains(&a) && tie.contains(&b), "{tie}");
        let Err(Error::Invalid(miss)) = resolve(&env.conn, "nope") else {
            panic!("expected a miss");
        };
        assert_eq!(miss, "no project 'nope'; projects: Same, same");
    }
}
