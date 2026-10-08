//! The commands the frontend calls, by name with JSON arguments. The app's single `api` Tauri
//! command and the UI tests' bridge both go through [`dispatch`], so they run the same code.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{Error, Paths, Result, db, tasks};

#[derive(serde::Deserialize)]
struct ProjectArgs {
    project_id: String,
}

#[derive(serde::Deserialize)]
struct TaskArgs {
    project_id: String,
    task_hash: String,
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
