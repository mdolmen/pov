//! Test-only HTTP bridge to [`pov_core::api::dispatch`] for the UI tests; never shipped.
//!
//! Serves the dev data dir under `$HOME`. Prints `{"port": N}` on start, then answers
//! `POST /<command>` with JSON arguments: the JSON result, or 400 with the error text.

use std::error::Error;
use std::io::Write;

use pov_core::{Paths, api, db, storage};
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
            .and_then(|args| api::dispatch(&paths, &cmd, args));
        let response = match result {
            Ok(value) => Response::from_string(value.to_string()).with_header(json.clone()),
            Err(e) => Response::from_string(e.to_string()).with_status_code(400),
        };
        request.respond(response)?;
    }
    Ok(())
}
