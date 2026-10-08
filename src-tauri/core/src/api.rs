//! The commands the frontend calls, by name with JSON arguments. The app's single `api` Tauri
//! command and the UI tests' bridge both go through [`dispatch`], so they run the same code.

use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::projects::{self, NewProject, ProjectUpdate};
use crate::{Error, Paths, Result, db, tasks};

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
        "cleanup_selected" => json(tasks::cleanup_selected(&conn)?),
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
}
