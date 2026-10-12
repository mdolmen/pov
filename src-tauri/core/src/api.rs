//! The commands the frontend calls, by name with JSON arguments. The app's single `api` Tauri
//! command and the UI tests' bridge both go through [`dispatch`], so they run the same code.

use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::projects::{self, NewProject, ProjectUpdate};
use crate::{Error, Paths, Result, activity, db, tasks, timelog};

#[derive(Deserialize)]
struct ProjectArgs {
    project_id: String,
}

#[derive(Deserialize)]
struct TaskArgs {
    project_id: String,
    task_hash: String,
}

#[derive(Deserialize)]
struct RestoreTaskArgs {
    project_id: String,
    line_number: usize,
    text: String,
}

#[derive(Deserialize)]
struct AddProjectArgs {
    name: String,
    file_path: PathBuf,
    #[serde(rename = "type", default = "default_type")]
    project_type: String,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default)]
    sub_status: Option<String>,
}

fn default_type() -> String {
    "project".into()
}

fn default_status() -> String {
    "open".into()
}

#[derive(Deserialize)]
struct UpdateProjectArgs {
    project_id: String,
    name: Option<String>,
    status: Option<String>,
    #[serde(default, deserialize_with = "present")]
    sub_status: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    paused_until: Option<Option<String>>,
}

/// Wraps a field that is present, even as `null`, in `Some`; an absent field stays `None`.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

#[derive(Deserialize)]
struct ListTimeArgs {
    project_id: String,
    #[serde(default = "default_days")]
    days: u64,
}

fn default_days() -> u64 {
    120
}

#[derive(Deserialize)]
struct ActivityArgs {
    #[serde(rename = "type")]
    project_type: String,
    #[serde(default = "default_days")]
    days: u64,
    #[serde(default)]
    project_id: Option<String>,
}

#[derive(Deserialize)]
struct AddTimeArgs {
    project_id: String,
    minutes: i64,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    topic: Option<String>,
}

fn parse<T: DeserializeOwned>(args: Value) -> Result<T> {
    serde_json::from_value(args).map_err(|e| Error::Invalid(format!("bad arguments: {e}")))
}

fn json<T: Serialize>(value: T) -> Result<Value> {
    Ok(serde_json::to_value(value)?)
}

/// Runs command `cmd` with its JSON arguments and returns its JSON result.
pub fn dispatch(paths: &Paths, cmd: &str, args: Value) -> Result<Value> {
    let conn = db::open(paths)?;
    match cmd {
        "list_tasks" => {
            let a: ProjectArgs = parse(args)?;
            json(tasks::list(&conn, &a.project_id)?)
        }
        "toggle_task" => {
            let a: TaskArgs = parse(args)?;
            json(tasks::toggle(&conn, paths, &a.project_id, &a.task_hash)?)
        }
        "select_task" => {
            let a: TaskArgs = parse(args)?;
            json(tasks::select(&conn, &a.project_id, &a.task_hash)?)
        }
        "unselect_task" => {
            let a: TaskArgs = parse(args)?;
            json(tasks::unselect(&conn, &a.project_id, &a.task_hash)?)
        }
        "inbox_task" => {
            let a: TaskArgs = parse(args)?;
            json(tasks::inbox(&conn, &a.project_id, &a.task_hash)?)
        }
        "keep_task" => {
            let a: TaskArgs = parse(args)?;
            json(tasks::keep(&conn, &a.project_id, &a.task_hash)?)
        }
        "delete_task" => {
            let a: TaskArgs = parse(args)?;
            json(tasks::delete(&conn, paths, &a.project_id, &a.task_hash)?)
        }
        "restore_task" => {
            let a: RestoreTaskArgs = parse(args)?;
            json(tasks::restore(
                &conn,
                paths,
                &a.project_id,
                a.line_number,
                &a.text,
            )?)
        }
        "cleanup_selected" => json(tasks::cleanup_selected(&conn)?),
        "next_up" => json(tasks::next_up(&conn)?),
        "list_projects" => json(projects::list(&conn, paths)?),
        "add_project" => {
            let a: AddProjectArgs = parse(args)?;
            let project = NewProject {
                name: a.name,
                file_path: a.file_path,
                project_type: a.project_type,
                status: a.status,
                sub_status: a.sub_status,
            };
            let id = projects::add(&conn, paths, &project)?;
            json(projects::get(&conn, paths, &id)?)
        }
        "update_project" => {
            let a: UpdateProjectArgs = parse(args)?;
            let changes = ProjectUpdate {
                name: a.name,
                status: a.status,
                sub_status: a.sub_status,
                paused_until: a.paused_until,
            };
            json(projects::update(&conn, paths, &a.project_id, &changes)?)
        }
        "list_time" => {
            let a: ListTimeArgs = parse(args)?;
            json(timelog::list_days(&conn, &a.project_id, a.days)?)
        }
        "add_time" => {
            let a: AddTimeArgs = parse(args)?;
            let entry = timelog::add_entry(
                &conn,
                &a.project_id,
                a.minutes,
                a.date.as_deref(),
                a.topic.as_deref(),
            )?;
            json(entry)
        }
        "list_topics" => {
            let a: ProjectArgs = parse(args)?;
            json(timelog::topics(&conn, &a.project_id)?)
        }
        "activity" => {
            let a: ActivityArgs = parse(args)?;
            match a.project_id {
                Some(id) => json(activity::project_days(&conn, paths, &id, a.days)?),
                None => json(activity::heatmap(&conn, paths, &a.project_type, a.days)?),
            }
        }
        "remove_project" => {
            let a: ProjectArgs = parse(args)?;
            json(projects::remove(&conn, paths, &a.project_id)?)
        }
        _ => Err(Error::Invalid(format!("unknown command '{cmd}'"))),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::*;
    use crate::storage;

    fn paths() -> (tempfile::TempDir, Paths) {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("pov"));
        storage::init(&paths).unwrap();
        db::init(&paths).unwrap();
        (tmp, paths)
    }

    #[test]
    fn unknown_command_and_bad_arguments_are_errors() {
        let (_tmp, paths) = paths();
        let err = dispatch(&paths, "nope", json!({})).unwrap_err();
        assert_eq!(err.to_string(), "unknown command 'nope'");
        let err = dispatch(&paths, "list_tasks", json!({"id": "p"})).unwrap_err();
        assert!(
            err.to_string()
                .starts_with("bad arguments: missing field `project_id`"),
            "{err}"
        );
    }

    #[test]
    fn errors_read_like_the_python_api() {
        let (_tmp, paths) = paths();
        let err = dispatch(&paths, "list_tasks", json!({"project_id": "nope"})).unwrap_err();
        assert_eq!(err.to_string(), "project not found");
    }

    #[test]
    fn update_project_tells_null_from_absent() {
        let (tmp, paths) = paths();
        let todo = tmp.path().join("TODO.md");
        fs::write(&todo, "- [ ] one\n").unwrap();
        let added = dispatch(
            &paths,
            "add_project",
            json!({"name": "P", "file_path": todo, "status": "archived", "sub_status": "paused"}),
        )
        .unwrap();
        assert_eq!(added["type"], "project");
        let id = added["id"].clone();

        let renamed = dispatch(
            &paths,
            "update_project",
            json!({"project_id": id, "name": "Q"}),
        );
        assert_eq!(renamed.unwrap()["sub_status"], "paused");

        let cleared = dispatch(
            &paths,
            "update_project",
            json!({"project_id": id, "sub_status": null}),
        );
        assert_eq!(cleared.unwrap()["sub_status"], Value::Null);
    }

    #[test]
    fn select_then_list_tasks() {
        let (tmp, paths) = paths();
        let todo = tmp.path().join("TODO.md");
        fs::write(&todo, "# T\n- [ ] one\n").unwrap();
        db::open(&paths)
            .unwrap()
            .execute(
                "INSERT INTO projects (id, name, file_path) VALUES ('p', 'P', ?)",
                [todo.to_str().unwrap()],
            )
            .unwrap();
        let hash = tasks::task_hash("- [ ] one", 1);

        let selected = dispatch(
            &paths,
            "select_task",
            json!({"project_id": "p", "task_hash": hash}),
        );
        assert_eq!(selected.unwrap(), Value::Null);

        let items = dispatch(&paths, "list_tasks", json!({"project_id": "p"})).unwrap();
        assert_eq!(
            items[0],
            json!({"kind": "heading", "text": "T", "level": 1})
        );
        assert_eq!(items[1]["kind"], "task");
        assert_eq!(items[1]["hash"], hash);
        assert_eq!(items[1]["is_selected"], true);
        assert_eq!(items[1]["subtasks"], json!([]));
    }

    #[test]
    fn inbox_then_keep() {
        let (tmp, paths) = paths();
        let todo = tmp.path().join("TODO.md");
        fs::write(&todo, "- [ ] one\n").unwrap();
        db::open(&paths)
            .unwrap()
            .execute(
                "INSERT INTO projects (id, name, file_path) VALUES ('p', 'P', ?)",
                [todo.to_str().unwrap()],
            )
            .unwrap();
        let args = json!({"project_id": "p", "task_hash": tasks::task_hash("- [ ] one", 1)});

        dispatch(&paths, "inbox_task", args.clone()).unwrap();
        let items = dispatch(&paths, "list_tasks", json!({"project_id": "p"})).unwrap();
        assert_eq!(items[0]["in_inbox"], true);
        assert_eq!(items[0]["is_selected"], false);

        dispatch(&paths, "keep_task", args).unwrap();
        let items = dispatch(&paths, "list_tasks", json!({"project_id": "p"})).unwrap();
        assert_eq!(items[0]["in_inbox"], false);
    }

    #[test]
    fn delete_then_restore_a_task() {
        let (tmp, paths) = paths();
        let todo = tmp.path().join("TODO.md");
        fs::write(&todo, "- [ ] one\n- [ ] two\n").unwrap();
        db::open(&paths)
            .unwrap()
            .execute(
                "INSERT INTO projects (id, name, file_path) VALUES ('p', 'P', ?)",
                [todo.to_str().unwrap()],
            )
            .unwrap();
        let hash = tasks::task_hash("- [ ] one", 1);

        let deleted = dispatch(
            &paths,
            "delete_task",
            json!({"project_id": "p", "task_hash": hash}),
        )
        .unwrap();
        assert_eq!(deleted, json!({"line_number": 0, "text": "- [ ] one\n"}));
        assert_eq!(fs::read_to_string(&todo).unwrap(), "- [ ] two\n");

        let restored = dispatch(
            &paths,
            "restore_task",
            json!({"project_id": "p", "line_number": 0, "text": "- [ ] one\n"}),
        );
        assert_eq!(restored.unwrap(), Value::Null);
        assert_eq!(fs::read_to_string(&todo).unwrap(), "- [ ] one\n- [ ] two\n");
    }

    #[test]
    fn next_up_lists_the_milestone_of_a_next_task() {
        let (tmp, paths) = paths();
        let todo = tmp.path().join("TODO.md");
        fs::write(&todo, "## M\n- [ ] one\n- [x] two\n").unwrap();
        db::open(&paths)
            .unwrap()
            .execute(
                "INSERT INTO projects (id, name, file_path) VALUES ('p', 'P', ?)",
                [todo.to_str().unwrap()],
            )
            .unwrap();
        let hash = tasks::task_hash("- [ ] one", 1);
        dispatch(
            &paths,
            "select_task",
            json!({"project_id": "p", "task_hash": hash}),
        )
        .unwrap();

        let entries = dispatch(&paths, "next_up", json!({})).unwrap();

        assert_eq!(
            entries,
            json!([{
                "project_id": "p", "project_name": "P", "project_type": "project",
                "milestone": "M", "task_text": null, "task_hash": null,
                "next_count": 1, "done": 1, "total": 2,
            }])
        );
    }

    #[test]
    fn activity_of_one_project() {
        let (tmp, paths) = paths();
        let todo = tmp.path().join("TODO.md");
        fs::write(&todo, "- [ ] one\n").unwrap();
        db::open(&paths)
            .unwrap()
            .execute(
                "INSERT INTO projects (id, name, file_path) VALUES ('p', 'P', ?)",
                [todo.to_str().unwrap()],
            )
            .unwrap();

        let days = dispatch(
            &paths,
            "activity",
            json!({"type": "project", "days": 56, "project_id": "p"}),
        );
        assert_eq!(days.unwrap(), json!([]));
    }
}
