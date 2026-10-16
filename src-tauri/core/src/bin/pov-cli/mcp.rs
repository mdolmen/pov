//! `pov mcp`: an MCP server on stdin / stdout, spawned by Claude Code or another MCP client.
//!
//! Its tools let a model find a project and add a task to it; the task lands in the inbox for
//! the user to triage. Agents tasks are delegated to answer through it too: subtasks, links,
//! then the task back to the inbox. Each call opens its own DB connection and runs the sync core on a
//! blocking thread; the busy timeout waits out the app.

use pov_core::{Paths, db, projects, tasks};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::schemars::JsonSchema;
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const INSTRUCTIONS: &str = "pov tracks the user's projects, each a TODO.md file of Markdown \
checkboxes. Use add_task when the user asks to note, track or remember something to do for \
one of these projects; it lands in the project's inbox for the user to triage. An agent a task \
was delegated to reports with add_subtask and add_links, then hands the task back with \
finish_task; list_delegated lists its tasks.";

/// Serves until the client closes stdin.
pub fn serve(paths: Paths) -> std::io::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let server = Server::new(paths)
                .serve(rmcp::transport::stdio())
                .await
                .map_err(std::io::Error::other)?;
            server.waiting().await.map_err(std::io::Error::other)?;
            Ok(())
        })
}

#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct ProjectArgs {
    /// Project id or name (any case), from list_projects.
    project: String,
}

/// Where a new task goes: the inbox (for the user to triage), the backlog, or next up.
#[derive(Deserialize, JsonSchema, Default, Clone, Copy)]
#[schemars(crate = "rmcp::schemars")]
#[serde(rename_all = "lowercase")]
enum Stage {
    #[default]
    Inbox,
    Backlog,
    Next,
}

impl Stage {
    fn as_str(self) -> &'static str {
        match self {
            Stage::Inbox => "inbox",
            Stage::Backlog => "backlog",
            Stage::Next => "next",
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct AddTaskArgs {
    /// Project id or name (any case), from list_projects.
    project: String,
    /// The task, one line of text, without the `- [ ]` checkbox.
    text: String,
    /// Heading to add the task under, exactly as list_sections gives it; the end of the file
    /// when omitted.
    section: Option<String>,
    /// Defaults to inbox; only set it when the user asks.
    #[serde(default)]
    stage: Stage,
    /// A JSON object stored with the task, e.g. `{"source": {...}}` for where it came from;
    /// pov reads `note` and `links` (see add_links).
    metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct ListDelegatedArgs {
    /// The agent's tool name, as in pov's Settings.
    tool: String,
}

#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct AddSubtaskArgs {
    /// The task's id, as pov sent it or list_delegated gives it.
    id: i64,
    /// One line of text, without the checkbox; Markdown links (`[label](target)`) open on click.
    text: String,
    /// Defaults to checked: work done, for the user to review.
    #[serde(default = "checked")]
    checked: bool,
}

fn checked() -> bool {
    true
}

/// A link shown in the task's details: a URL or an absolute file path.
#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct LinkArg {
    label: String,
    /// A URL or an absolute file path.
    target: String,
    /// The app to open it with (macOS app name, e.g. "LibreOffice"); the default app when
    /// omitted.
    app: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct AddLinksArgs {
    /// The task's id, as pov sent it or list_delegated gives it.
    id: i64,
    links: Vec<LinkArg>,
}

#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct TaskIdArgs {
    /// The task's id, as pov sent it or list_delegated gives it.
    id: i64,
}

#[derive(Serialize)]
struct ProjectSummary {
    id: String,
    name: String,
    #[serde(rename = "type")]
    project_type: String,
    status: String,
    open_tasks: usize,
    next: i64,
    inbox: i64,
}

#[derive(Clone)]
struct Server {
    paths: Paths,
    tool_router: ToolRouter<Self>,
}

impl Server {
    fn new(paths: Paths) -> Self {
        Self {
            paths,
            tool_router: Self::tool_router(),
        }
    }

    /// Runs `f` on a blocking thread with its own connection; errors become tool errors.
    async fn run<T: Serialize + Send + 'static>(
        &self,
        f: impl FnOnce(&Connection, &Paths) -> pov_core::Result<T> + Send + 'static,
    ) -> Result<String, String> {
        let paths = self.paths.clone();
        let out = tokio::task::spawn_blocking(move || f(&db::open(&paths)?, &paths))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&out).map_err(|e| e.to_string())
    }
}

#[tool_router]
impl Server {
    #[tool(
        description = "List the user's tracked projects (open ones first) with their open \
                          task, next and inbox counts."
    )]
    async fn list_projects(&self) -> Result<String, String> {
        self.run(|conn, paths| {
            let mut list: Vec<_> = projects::list(conn, paths)?
                .into_iter()
                .map(|p| ProjectSummary {
                    id: p.id,
                    name: p.name,
                    project_type: p.project_type,
                    status: p.status,
                    open_tasks: p.task_count,
                    next: p.selected_count,
                    inbox: p.inbox_count,
                })
                .collect();
            list.sort_by_key(|p| (p.status != "open", p.name.to_lowercase()));
            Ok(list)
        })
        .await
    }

    #[tool(
        description = "List a project's headings (text and level), where add_task can put \
                          a task."
    )]
    async fn list_sections(
        &self,
        Parameters(args): Parameters<ProjectArgs>,
    ) -> Result<String, String> {
        self.run(move |conn, _| {
            let row = projects::resolve(conn, &args.project)?;
            Ok(tasks::sections(&row.file_path))
        })
        .await
    }

    #[tool(
        description = "Add one task to a project's TODO.md, at the end of a section or of \
                          the file. It lands in the inbox for the user to triage. Returns the \
                          task's hash."
    )]
    async fn add_task(&self, Parameters(args): Parameters<AddTaskArgs>) -> Result<String, String> {
        self.run(move |conn, paths| {
            let row = projects::resolve(conn, &args.project)?;
            let hash = tasks::add(
                conn,
                paths,
                &row.id,
                &args.text,
                args.section.as_deref(),
                args.stage.as_str(),
                args.metadata.map(serde_json::Value::Object).as_ref(),
            )?;
            Ok(serde_json::json!({ "hash": hash, "project": row.name }))
        })
        .await
    }

    #[tool(
        description = "List the tasks delegated to an agent: id, project, text, subtasks and \
                          metadata (note, links, source)."
    )]
    async fn list_delegated(
        &self,
        Parameters(args): Parameters<ListDelegatedArgs>,
    ) -> Result<String, String> {
        self.run(move |conn, paths| tasks::delegated(conn, paths, &args.tool))
            .await
    }

    #[tool(
        description = "Add a subtask under a delegated task, after its last subtask; checked \
                          by default. Returns the subtask's hash."
    )]
    async fn add_subtask(
        &self,
        Parameters(args): Parameters<AddSubtaskArgs>,
    ) -> Result<String, String> {
        self.run(move |conn, paths| {
            let hash = tasks::add_subtask(conn, paths, args.id, &args.text, args.checked)?;
            Ok(serde_json::json!({ "hash": hash }))
        })
        .await
    }

    #[tool(
        description = "Attach links to a task's details: URLs or file paths the user opens \
                          from pov, optionally with a given app."
    )]
    async fn add_links(
        &self,
        Parameters(args): Parameters<AddLinksArgs>,
    ) -> Result<String, String> {
        self.run(move |conn, _| {
            let links: Vec<tasks::Link> = args
                .links
                .into_iter()
                .map(|l| tasks::Link {
                    label: l.label,
                    target: l.target,
                    app: l.app,
                })
                .collect();
            tasks::add_links(conn, args.id, &links)
        })
        .await
    }

    #[tool(
        description = "Hand a delegated task back to the user: it leaves the agent and lands \
                          in the inbox for review. Call it once the work is done, or failed (add \
                          an open subtask saying why first)."
    )]
    async fn finish_task(
        &self,
        Parameters(args): Parameters<TaskIdArgs>,
    ) -> Result<String, String> {
        self.run(move |conn, _| tasks::finish(conn, args.id)).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("pov", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}
