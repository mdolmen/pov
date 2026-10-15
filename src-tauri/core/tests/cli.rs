//! Runs the `pov-cli` binary on a temp `HOME`. Ported from the Python `test_cli.py` and the
//! `import-time` cases of `test_timelog.py`.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Output, Stdio};

use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::TempDir;

const TABLE: &str = "\
| DATE       | TIME | TOPIC          |
| ---------- | ---- | -------------- |
| 2026-01-31 | 2,00 | Proba          |
| 2026-01-31 | 1,50 | Statistics     |
| 2026-02-01 | 0,25 |                |
";

/// A temp `HOME` holding the data dir, plus a folder for the user's files.
struct Env {
    home: TempDir,
    files: TempDir,
}

impl Env {
    fn new() -> Self {
        Self {
            home: tempfile::tempdir().unwrap(),
            files: tempfile::tempdir().unwrap(),
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pov-cli"));
        command
            .args(args)
            .env("HOME", self.home.path())
            .env_remove("POV_ENV")
            .envs([("GIT_AUTHOR_NAME", "t"), ("GIT_AUTHOR_EMAIL", "t@t")])
            .envs([("GIT_COMMITTER_NAME", "t"), ("GIT_COMMITTER_EMAIL", "t@t")]);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    /// Starts `pov-cli mcp` and completes the MCP handshake.
    fn mcp(&self) -> Mcp {
        let mut child = self
            .command(&["mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut mcp = Mcp { child, stdout };
        let init = mcp.request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0"},
            }),
        );
        assert_eq!(init["serverInfo"]["name"], "pov");
        mcp.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        mcp
    }

    /// Writes a file in the user's folder and returns its path as a string.
    fn file(&self, name: &str, content: &str) -> String {
        let path = self.files.path().join(name);
        fs::write(&path, content).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn missing(&self, name: &str) -> String {
        self.files.path().join(name).to_string_lossy().into_owned()
    }

    fn db(&self) -> Connection {
        // Debug builds of the CLI use the dev data dir, like the app.
        let dir = if cfg!(debug_assertions) {
            "pov-dev"
        } else {
            "pov"
        };
        let path: PathBuf = self
            .home
            .path()
            .join(".local/share")
            .join(dir)
            .join("pov.db");
        Connection::open(path).unwrap()
    }

    fn time_entries(&self) -> i64 {
        self.db()
            .query_row("SELECT COUNT(*) FROM time_entries", (), |r| r.get(0))
            .unwrap()
    }

    fn seed_learning(&self, name: &str) {
        let src = self.file(&format!("{name}.md"), "- [ ] x\n");
        assert_eq!(
            code(&self.run(&["add", &src, "--name", name, "--type", "learning"])),
            0
        );
    }
}

/// A running `pov-cli mcp`, talking JSON-RPC one line per message.
struct Mcp {
    child: Child,
    stdout: BufReader<ChildStdout>,
}

impl Mcp {
    fn send(&mut self, message: &Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    /// Sends a request and returns the `result` of the response with the same id.
    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let mut line = String::new();
            assert_ne!(
                self.stdout.read_line(&mut line).unwrap(),
                0,
                "server closed stdout"
            );
            let message: Value = serde_json::from_str(&line).unwrap();
            if message["id"] == id {
                return message["result"].clone();
            }
        }
    }

    fn call(&mut self, id: u64, tool: &str, arguments: Value) -> Value {
        self.request(
            id,
            "tools/call",
            json!({"name": tool, "arguments": arguments}),
        )
    }

    /// Closes stdin and returns the exit code.
    fn close(mut self) -> i32 {
        drop(self.child.stdin.take());
        self.child.wait().unwrap().code().unwrap()
    }
}

/// The text of a tool result.
fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap()
}

fn code(out: &Output) -> i32 {
    out.status.code().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn add_then_list() {
    let env = Env::new();
    let src = env.file("TODO.md", "- [ ] task\n");

    let out = env.run(&["add", &src, "--name", "Demo"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("added: Demo"));

    let row: (String, String, String) = env
        .db()
        .query_row("SELECT name, type, status FROM projects", (), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!(row, ("Demo".into(), "project".into(), "open".into()));

    let out = env.run(&["list"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("Demo"));
    assert!(stdout(&out).contains("project"));
}

#[test]
fn list_aligns_columns() {
    let env = Env::new();
    assert_eq!(stdout(&env.run(&["list"])), "(no projects)\n");

    let a = env.file("a.md", "- [ ] x\n");
    let b = env.file("b.md", "- [ ] x\n");
    env.run(&["add", &a, "--name", "Long name"]);
    env.run(&[
        "add", &b, "--name", "B", "--type", "learning", "--status", "archived",
    ]);

    let a = fs::canonicalize(&a).unwrap();
    let b = fs::canonicalize(&b).unwrap();
    assert_eq!(
        stdout(&env.run(&["list"])),
        format!(
            "B          learning  archived  {}\nLong name  project   open      {}\n",
            b.display(),
            a.display()
        )
    );
}

#[test]
fn add_default_name_is_file_stem() {
    let env = Env::new();
    let src = env.file("MyProject.md", "- [ ] x\n");
    let out = env.run(&["add", &src]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("added: MyProject"));
}

#[test]
fn add_learning_type() {
    let env = Env::new();
    let src = env.file("L.md", "- [ ] x\n");
    assert_eq!(code(&env.run(&["add", &src, "--type", "learning"])), 0);
    let kind: String = env
        .db()
        .query_row("SELECT type FROM projects", (), |r| r.get(0))
        .unwrap();
    assert_eq!(kind, "learning");
}

#[test]
fn add_bad_type_is_a_usage_error() {
    let env = Env::new();
    let src = env.file("L.md", "- [ ] x\n");
    assert_eq!(code(&env.run(&["add", &src, "--type", "nope"])), 2);
}

#[test]
fn add_missing_file_returns_2() {
    let env = Env::new();
    let out = env.run(&["add", &env.missing("nope.md")]);
    assert_eq!(code(&out), 2);
    assert!(stderr(&out).contains("not found"));
}

#[test]
fn add_duplicate_returns_1() {
    let env = Env::new();
    let src = env.file("dup.md", "- [ ] x\n");
    assert_eq!(code(&env.run(&["add", &src, "--name", "A"])), 0);
    let out = env.run(&["add", &src, "--name", "A2"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("already tracked"));
}

#[test]
fn remove() {
    let env = Env::new();
    let src = env.file("TODO.md", "- [ ] x\n");
    env.run(&["add", &src, "--name", "Demo"]);

    let out = env.run(&["remove", "Demo"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("removed: Demo"));

    let count: i64 = env
        .db()
        .query_row("SELECT COUNT(*) FROM projects", (), |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    // Original file untouched
    assert!(Path::new(&src).exists());
}

#[test]
fn remove_unknown_returns_1() {
    let env = Env::new();
    let out = env.run(&["remove", "no-such-project"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("no project named"));
}

#[test]
fn import_time() {
    let env = Env::new();
    env.seed_learning("Maths");
    let time = env.file("TIME.md", TABLE);

    let out = env.run(&["import-time", "Maths", &time]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert_eq!(stdout(&out), "imported: 3 entries (3.75h) into Maths\n");

    let rows: Vec<(String, i64, Option<String>)> = env
        .db()
        .prepare("SELECT date, minutes, topic FROM time_entries ORDER BY id")
        .unwrap()
        .query_map((), |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        rows,
        [
            ("2026-01-31".into(), 120, Some("Proba".into())),
            ("2026-01-31".into(), 90, Some("Statistics".into())),
            ("2026-02-01".into(), 15, None),
        ]
    );
}

#[test]
fn import_time_refuses_to_duplicate() {
    let env = Env::new();
    env.seed_learning("Maths");
    let time = env.file("TIME.md", TABLE);
    assert_eq!(code(&env.run(&["import-time", "Maths", &time])), 0);

    let out = env.run(&["import-time", "Maths", &time]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("--replace"));

    assert_eq!(
        code(&env.run(&["import-time", "Maths", &time, "--replace"])),
        0
    );
    assert_eq!(env.time_entries(), 3);
}

#[test]
fn import_time_append_keeps_existing() {
    let env = Env::new();
    env.seed_learning("Maths");
    let time = env.file("TIME.md", TABLE);
    assert_eq!(code(&env.run(&["import-time", "Maths", &time])), 0);

    assert_eq!(
        code(&env.run(&["import-time", "Maths", &time, "--append"])),
        0
    );
    assert_eq!(env.time_entries(), 6);
}

#[test]
fn import_time_replace_and_append_conflict() {
    let env = Env::new();
    env.seed_learning("Maths");
    let time = env.file("TIME.md", TABLE);
    let out = env.run(&["import-time", "Maths", &time, "--replace", "--append"]);
    assert_eq!(code(&out), 2);
}

#[test]
fn import_time_unknown_project() {
    let env = Env::new();
    let time = env.file("TIME.md", TABLE);
    let out = env.run(&["import-time", "Nope", &time]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("no project named"));
}

#[test]
fn import_time_missing_file() {
    let env = Env::new();
    env.seed_learning("Maths");
    let out = env.run(&["import-time", "Maths", &env.missing("nope.md")]);
    assert_eq!(code(&out), 2);
    assert!(stderr(&out).contains("not found"));
}

#[test]
fn import_time_parse_error_returns_2() {
    let env = Env::new();
    env.seed_learning("Maths");
    let time = env.file("TIME.md", "| 2026-01-31 | two | Proba |\n");
    let out = env.run(&["import-time", "Maths", &time]);
    assert_eq!(code(&out), 2);
    assert!(stderr(&out).contains("bad duration"));
}

#[test]
fn mcp_lists_tools_and_adds_a_task_to_the_inbox() {
    let env = Env::new();
    let src = env.file("TODO.md", "## A\n- [ ] a\n## B\n");
    assert_eq!(code(&env.run(&["add", &src, "--name", "Demo"])), 0);
    let mut mcp = env.mcp();

    let tools = mcp.request(2, "tools/list", json!({}));
    let mut names: Vec<_> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["add_task", "list_projects", "list_sections"]);
    assert!(
        tools
            .to_string()
            .contains(r#""enum":["inbox","backlog","next"]"#)
    );

    let projects = mcp.call(3, "list_projects", json!({}));
    let projects: Value = serde_json::from_str(text(&projects)).unwrap();
    assert_eq!(projects[0]["name"], "Demo");

    let sections = mcp.call(4, "list_sections", json!({"project": "demo"}));
    assert!(text(&sections).contains("\"A\""));

    let added = mcp.call(
        5,
        "add_task",
        json!({"project": "DEMO", "text": "x", "section": "A"}),
    );
    assert_ne!(added["isError"], true, "{added}");
    assert_eq!(
        fs::read_to_string(&src).unwrap(),
        "## A\n- [ ] a\n- [ ] x\n## B\n"
    );
    let stage: String = env
        .db()
        .query_row("SELECT stage FROM selected_tasks", (), |r| r.get(0))
        .unwrap();
    assert_eq!(stage, "inbox");

    let missing = mcp.call(6, "add_task", json!({"project": "nope", "text": "x"}));
    assert_eq!(missing["isError"], true);
    assert_eq!(text(&missing), "no project 'nope'; projects: Demo");

    assert_eq!(mcp.close(), 0);
}
