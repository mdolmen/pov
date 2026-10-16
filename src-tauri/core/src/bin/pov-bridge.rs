//! Test-only HTTP bridge to [`pov_core::api::dispatch`] for the UI tests; never shipped.
//!
//! Serves the dev data dir under `$HOME`. Prints `{"port": N}` on start, then answers
//! `POST /<command>` with JSON arguments: the JSON result, or 400 with the error text.
//!
//! Besides the app's commands, it serves what tools do through `pov mcp`, to seed tests:
//! `add_task` (with metadata) and `finish_task`.

use std::error::Error;
use std::io::Write;

use pov_core::{Paths, api, db, storage, tasks};
use serde::Deserialize;
use serde_json::Value;
use tiny_http::{Header, Response, Server};

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let paths = Paths::data_dir(true)?;
    storage::init(&paths)?;
    db::init(&paths)?;

    let server = Server::http("127.0.0.1:0")?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or("not an IP address")?;
    println!("{{\"port\": {port}}}");
    std::io::stdout().flush()?;

    let json = Header::from_bytes("Content-Type", "application/json").expect("static header");
    for mut request in server.incoming_requests() {
        let cmd = request.url().trim_start_matches('/').to_string();
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body)?;

        let result = serde_json::from_str::<Value>(if body.is_empty() { "{}" } else { &body })
            .map_err(pov_core::Error::from)
            .and_then(|args| run(&paths, &cmd, args));
        let response = match result {
            Ok(value) => Response::from_string(value.to_string()).with_header(json.clone()),
            Err(e) => Response::from_string(e.to_string()).with_status_code(400),
        };
        request.respond(response)?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct AddTaskArgs {
    project_id: String,
    text: String,
    #[serde(default = "inbox")]
    stage: String,
    #[serde(default)]
    meta: Option<Value>,
}

fn inbox() -> String {
    "inbox".into()
}

#[derive(Deserialize)]
struct FinishTaskArgs {
    id: i64,
}

/// A tool's command, else an app command.
fn run(paths: &Paths, cmd: &str, args: Value) -> pov_core::Result<Value> {
    let conn = db::open(paths)?;
    match cmd {
        "add_task" => {
            let a: AddTaskArgs = serde_json::from_value(args)?;
            let hash = tasks::add(
                &conn,
                paths,
                &a.project_id,
                &a.text,
                None,
                &a.stage,
                a.meta.as_ref(),
            )?;
            Ok(Value::String(hash))
        }
        "finish_task" => {
            let a: FinishTaskArgs = serde_json::from_value(args)?;
            tasks::finish(&conn, a.id)?;
            Ok(Value::Null)
        }
        _ => api::dispatch(paths, cmd, args),
    }
}
