//! Sending delegated tasks to their agents: a JSON POST to the agent's URL, after starting the
//! agent when nothing answers there.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::Connection;

use crate::projects::{self, Tool};
use crate::tasks::{self, Delegated};
use crate::{Error, Paths, Result};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// How long an agent has to answer a task it was sent.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a started agent has to answer at its URL.
const START_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_EVERY: Duration = Duration::from_millis(200);

/// The agents pov started, by name. Clones share them.
#[derive(Debug, Clone, Default)]
pub struct Launcher(Arc<Mutex<HashMap<String, Child>>>);

impl Launcher {
    /// Runs `command` unless the instance pov started for agent `name` is still running. The
    /// agent runs detached, without stdio, and is never waited on. Returns `true` when it was
    /// started.
    pub fn ensure_running(&self, name: &str, command: &Path) -> Result<bool> {
        // Held through the start: two delegations at once start the agent once.
        let mut children = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let child_running = children
            .get_mut(name)
            .is_some_and(|child| matches!(child.try_wait(), Ok(None)));
        if child_running {
            return Ok(false);
        }
        let child = Command::new(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|e| Error::Invalid(format!("could not start {name}: {e}")))?;
        children.insert(name.to_string(), child);
        Ok(true)
    }
}

/// Why a POST failed.
enum PostError {
    /// Nothing answers at the URL: no request was sent.
    Unreachable(String),
    /// The agent got the request and didn't take it, or the exchange broke.
    Failed(String),
}

/// POSTs `body` as JSON to an `http://host[:port]/path` URL; `Ok` on a 2xx answer.
fn post(url: &str, body: &[u8]) -> std::result::Result<(), PostError> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| PostError::Failed(format!("has a URL that isn't http://: {url}")))?;
    let (authority, path) = rest
        .split_once('/')
        .map_or((rest, "/".to_string()), |(a, p)| (a, format!("/{p}")));
    let addr = if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{authority}:80")
    };
    let addrs = addr
        .to_socket_addrs()
        .map_err(|e| PostError::Unreachable(e.to_string()))?;
    // `localhost` may resolve to IPv6 first while the agent only listens on IPv4.
    let mut refused = "no address".to_string();
    let mut stream = None;
    for a in addrs {
        match TcpStream::connect_timeout(&a, CONNECT_TIMEOUT) {
            Ok(s) => {
                stream = Some(s);
                break;
            }
            Err(e) => refused = e.to_string(),
        }
    }
    let mut stream = stream.ok_or(PostError::Unreachable(refused))?;

    let broke = |e: std::io::Error| PostError::Failed(format!("failed: {e}"));
    stream
        .set_read_timeout(Some(ANSWER_TIMEOUT))
        .map_err(broke)?;
    stream
        .set_write_timeout(Some(ANSWER_TIMEOUT))
        .map_err(broke)?;
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).map_err(broke)?;
    stream.write_all(body).map_err(broke)?;
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).map_err(broke)?;

    let answer = String::from_utf8_lossy(&answer);
    let status = answer.lines().next().unwrap_or_default();
    let (_, reason) = status.split_once(' ').unwrap_or_default();
    match reason.split(' ').next().and_then(|c| c.parse::<u16>().ok()) {
        Some(200..=299) => Ok(()),
        _ => {
            let text = answer.split_once("\r\n\r\n").map_or("", |(_, b)| b).trim();
            let text = if text.is_empty() {
                String::new()
            } else {
                format!(": {text}")
            };
            Err(PostError::Failed(format!("answered {reason}{text}")))
        }
    }
}

/// Sends a delegated task to agent `name`: a JSON POST to its URL. When nothing answers there,
/// starts the agent with its command, if it has one, and sends the task once it answers.
pub fn send(launcher: &Launcher, name: &str, tool: &Tool, task: &Delegated) -> Result<()> {
    let body = serde_json::to_vec(task)?;
    let failed = |e: String| Error::Invalid(format!("{name} {e}"));
    let refused = match post(&tool.url, &body) {
        Ok(()) => return Ok(()),
        Err(PostError::Failed(e)) => return Err(failed(e)),
        Err(PostError::Unreachable(e)) => e,
    };
    let Some(command) = &tool.command else {
        return Err(failed(format!(
            "isn't answering at {}: {refused}",
            tool.url
        )));
    };
    launcher.ensure_running(name, command)?;
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        thread::sleep(RETRY_EVERY);
        match post(&tool.url, &body) {
            Ok(()) => return Ok(()),
            Err(PostError::Failed(e)) => return Err(failed(e)),
            Err(PostError::Unreachable(e)) if Instant::now() >= deadline => {
                return Err(failed(format!(
                    "was started but isn't answering at {}: {e}",
                    tool.url
                )));
            }
            Err(PostError::Unreachable(_)) => {}
        }
    }
}

/// Sends a task to agent `name` and, once the agent took it, delegates it (see
/// [`tasks::delegate`]). When the agent doesn't take it, the task stays as it was.
pub fn delegate(
    conn: &Connection,
    paths: &Paths,
    launcher: &Launcher,
    project_id: &str,
    task_hash: &str,
    name: &str,
    note: Option<&str>,
) -> Result<()> {
    let tool = projects::tool(paths, name)?;
    let delegation = tasks::delegation(conn, paths, project_id, task_hash, name, note)?;
    if let Err(e) = send(launcher, name, &tool, &delegation.task) {
        tasks::cancel(conn, &delegation)?;
        return Err(e);
    }
    tasks::delegate(conn, paths, project_id, task_hash, name, note)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::mpsc;

    use serde_json::json;

    use super::*;
    use crate::{db, storage};

    /// An executable shell script running `body`.
    fn script(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("tool.sh");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Waits up to 5 s for `f` to hold.
    fn eventually(mut f: impl FnMut() -> bool) -> bool {
        (0..500).any(|_| {
            let done = f();
            if !done {
                thread::sleep(Duration::from_millis(10));
            }
            done
        })
    }

    /// A local port nothing listens on.
    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    /// A fake agent: answers every request with `status` and passes on its request line and body.
    fn agent(listener: TcpListener, status: &'static str) -> mpsc::Receiver<(String, String)> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                reader.read_line(&mut request).unwrap();
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(n) = line.to_lowercase().strip_prefix("content-length:") {
                        length = n.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let text = if status.starts_with('2') { "" } else { "busy" };
                let answer = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                    text.len()
                );
                stream.write_all(answer.as_bytes()).unwrap();
                tx.send((request.trim().to_string(), String::from_utf8(body).unwrap()))
                    .unwrap();
            }
        });
        rx
    }

    fn listening() -> (TcpListener, String) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://127.0.0.1:{}/tasks",
            listener.local_addr().unwrap().port()
        );
        (listener, url)
    }

    fn task() -> Delegated {
        Delegated {
            id: 7,
            project_id: "p".into(),
            project_name: "P".into(),
            text: "Draft the report".into(),
            subtasks: vec![],
            data: json!({"note": "in Writer"}),
        }
    }

    #[test]
    fn send_posts_the_task_as_json() {
        let (listener, url) = listening();
        let received = agent(listener, "202 Accepted");
        let tool = Tool { url, command: None };

        send(&Launcher::default(), "t", &tool, &task()).unwrap();

        let (request, body) = received.recv().unwrap();
        assert_eq!(request, "POST /tasks HTTP/1.1");
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["id"], 7);
        assert_eq!(body["text"], "Draft the report");
        assert_eq!(body["data"]["note"], "in Writer");
    }

    #[test]
    fn an_agent_that_refuses_the_task_is_an_error() {
        let (listener, url) = listening();
        let _received = agent(listener, "503 Service Unavailable");
        let tool = Tool { url, command: None };

        let err = send(&Launcher::default(), "t", &tool, &task()).unwrap_err();

        assert_eq!(err.to_string(), "t answered 503 Service Unavailable: busy");
    }

    #[test]
    fn nothing_answering_without_a_command_is_an_error() {
        let url = format!("http://127.0.0.1:{}/tasks", free_port());
        let tool = Tool { url, command: None };

        let err = send(&Launcher::default(), "t", &tool, &task()).unwrap_err();

        assert!(
            err.to_string().starts_with("t isn't answering at http://"),
            "{err}"
        );
    }

    #[test]
    fn the_command_starts_the_agent_then_the_task_is_sent() {
        let tmp = tempfile::tempdir().unwrap();
        let port = free_port();
        let count = tmp.path().join("count");
        // The agent comes up once the command ran.
        let (tx, rx) = mpsc::channel();
        let started = count.clone();
        thread::spawn(move || {
            assert!(eventually(|| started.exists()));
            let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
            tx.send(agent(listener, "200 OK")).unwrap();
        });
        let tool = Tool {
            url: format!("http://127.0.0.1:{port}/tasks"),
            command: Some(script(
                tmp.path(),
                &format!("echo x >> '{}'", count.display()),
            )),
        };

        send(&Launcher::default(), "t", &tool, &task()).unwrap();

        let (_, body) = rx.recv().unwrap().recv().unwrap();
        assert!(body.contains("Draft the report"));
        assert_eq!(fs::read_to_string(&count).unwrap(), "x\n");
    }

    #[test]
    fn starts_an_agent_once_while_it_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let count = tmp.path().join("count");
        let command = script(
            tmp.path(),
            &format!("echo x >> '{}'\nsleep 2", count.display()),
        );
        let launcher = Launcher::default();

        assert!(launcher.ensure_running("t", &command).unwrap());
        assert!(!launcher.clone().ensure_running("t", &command).unwrap());

        assert!(eventually(|| count.exists()));
        assert_eq!(fs::read_to_string(&count).unwrap(), "x\n");
    }

    #[test]
    fn starts_an_agent_again_once_it_exited() {
        let tmp = tempfile::tempdir().unwrap();
        let command = script(tmp.path(), "exit 0");
        let launcher = Launcher::default();

        assert!(launcher.ensure_running("t", &command).unwrap());

        assert!(eventually(|| launcher
            .ensure_running("t", &command)
            .unwrap()));
    }

    #[test]
    fn a_missing_command_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();

        let err = Launcher::default()
            .ensure_running("t", &tmp.path().join("missing"))
            .unwrap_err();

        assert!(err.to_string().starts_with("could not start t: "), "{err}");
    }

    /// A data dir with project "P" (a plain task "one") linked to tool `t` at `url`.
    fn project(url: &str) -> (tempfile::TempDir, Paths, Connection, String) {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("pov"));
        storage::init(&paths).unwrap();
        db::init(&paths).unwrap();
        let conn = db::open(&paths).unwrap();
        let todo = tmp.path().join("TODO.md");
        fs::write(&todo, "- [ ] one\n").unwrap();
        let project = projects::NewProject {
            name: "P".into(),
            file_path: todo,
            project_type: "project".into(),
            status: "open".into(),
            sub_status: None,
        };
        let id = projects::add(&conn, &paths, &project).unwrap();
        projects::save_tool(&paths, "t", url, None).unwrap();
        projects::set_project_tools(&paths, &id, &["t".into()]).unwrap();
        (tmp, paths, conn, id)
    }

    fn task_view(conn: &Connection, id: &str) -> tasks::TaskView {
        match tasks::list(conn, id).unwrap().remove(0) {
            tasks::ListItem::Task(t) => t,
            tasks::ListItem::Heading { .. } => panic!("expected a task"),
        }
    }

    #[test]
    fn delegate_sends_then_delegates() {
        let (listener, url) = listening();
        let received = agent(listener, "202 Accepted");
        let (_tmp, paths, conn, id) = project(&url);
        let hash = tasks::task_hash("- [ ] one", 1);

        delegate(
            &conn,
            &paths,
            &Launcher::default(),
            &id,
            &hash,
            "t",
            Some("Go"),
        )
        .unwrap();

        let task = task_view(&conn, &id);
        assert_eq!(task.delegated_to.as_deref(), Some("t"));
        let (_, body) = received.recv().unwrap();
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["id"], task.meta.unwrap().id);
        assert_eq!(body["project_name"], "P");
        assert_eq!(body["data"], json!({"note": "Go"}));
    }

    #[test]
    fn a_task_the_agent_did_not_take_stays_as_it_was() {
        let url = format!("http://127.0.0.1:{}/tasks", free_port());
        let (_tmp, paths, conn, id) = project(&url);
        let hash = tasks::task_hash("- [ ] one", 1);
        tasks::select(&conn, &id, &hash).unwrap();

        let err = delegate(&conn, &paths, &Launcher::default(), &id, &hash, "t", None);

        assert!(err.is_err());
        let task = task_view(&conn, &id);
        assert_eq!(task.delegated_to, None);
        assert_eq!(task.meta, None);
        assert!(task.is_selected);
    }
}
