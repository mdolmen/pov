//! The `pov` command-line tool, installed as `~/.local/bin/pov` from the app's settings.
//!
//! Works on the data dir directly; the app doesn't need to be running. Named `pov-cli` because
//! the app binary is already `pov`.

mod mcp;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use pov_core::projects::{self, NewProject};
use pov_core::{Error, Paths, db, storage, timelog};
use rusqlite::Connection;

#[derive(Parser)]
#[command(name = "pov", about = "pov CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Track a new project
    Add {
        /// Path to a TODO.md file
        path: PathBuf,
        /// Display name (default: file stem)
        #[arg(long)]
        name: Option<String>,
        /// Project kind
        #[arg(long = "type", default_value = "project", value_parser = ["project", "learning"])]
        project_type: String,
        /// Project status
        #[arg(long, default_value = "open", value_parser = ["open", "archived"])]
        status: String,
    },
    /// List tracked projects
    List,
    /// Stop tracking a project
    Remove {
        /// Exact project name
        name: String,
    },
    /// Import a TIME.md file into a project
    ImportTime {
        /// Exact project name
        name: String,
        /// Path to a TIME.md file
        path: PathBuf,
        /// Delete the project's existing time entries first
        #[arg(long, conflicts_with = "append")]
        replace: bool,
        /// Keep the project's existing time entries
        #[arg(long)]
        append: bool,
    },
    /// Serve the MCP server over stdio, for Claude Code and other MCP clients
    Mcp,
}

/// A message for stderr and the exit code that goes with it.
struct Failure(String, u8);

impl From<Error> for Failure {
    fn from(e: Error) -> Self {
        Failure(e.to_string(), 1)
    }
}

type CmdResult = Result<(), Failure>;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Add {
            path,
            name,
            project_type,
            status,
        } => add(path, name, project_type, status),
        Command::List => list(),
        Command::Remove { name } => remove(&name),
        Command::ImportTime {
            name,
            path,
            replace,
            append,
        } => import_time(&name, &path, replace, append),
        Command::Mcp => serve_mcp(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure(msg, code)) => {
            eprintln!("error: {msg}");
            ExitCode::from(code)
        }
    }
}

/// Opens the same data dir as the app, creating it on first use.
fn open() -> Result<(Paths, Connection), Error> {
    let dev = cfg!(debug_assertions) || std::env::var("POV_ENV").is_ok_and(|v| v == "dev");
    let paths = Paths::data_dir(dev)?;
    storage::init(&paths)?;
    db::init(&paths)?;
    let conn = db::open(&paths)?;
    Ok((paths, conn))
}

fn serve_mcp() -> CmdResult {
    let (paths, _) = open()?;
    mcp::serve(paths).map_err(|e| Failure(e.to_string(), 1))
}

fn add(path: PathBuf, name: Option<String>, project_type: String, status: String) -> CmdResult {
    let (paths, conn) = open()?;
    let name = name.unwrap_or_else(|| {
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let project = NewProject {
        name,
        file_path: path,
        project_type,
        status,
        sub_status: None,
    };
    let id = projects::add(&conn, &paths, &project).map_err(|e| match e {
        Error::FileNotFound(_) => Failure(e.to_string(), 2),
        e => e.into(),
    })?;
    println!("added: {} ({id})", project.name);
    Ok(())
}

fn list() -> CmdResult {
    let (_, conn) = open()?;
    let rows = projects::list_by_name(&conn)?;
    if rows.is_empty() {
        println!("(no projects)");
        return Ok(());
    }
    let width = rows
        .iter()
        .map(|r| r.name.chars().count())
        .max()
        .unwrap_or(0);
    for r in rows {
        println!(
            "{:<width$}  {:<8}  {:<8}  {}",
            r.name,
            r.project_type,
            r.status,
            r.file_path.display()
        );
    }
    Ok(())
}

/// The project named `name`, with `ambiguous` as the message when several share it.
fn find(conn: &Connection, name: &str, ambiguous: &str) -> Result<projects::ProjectRow, Failure> {
    projects::find_by_name(conn, name).map_err(|e| match e {
        Error::NotFound(_) => Failure(format!("no project named '{name}'"), 1),
        Error::AmbiguousProject(_) => Failure(ambiguous.to_string(), 1),
        e => e.into(),
    })
}

fn remove(name: &str) -> CmdResult {
    let (paths, conn) = open()?;
    let ambiguous =
        format!("multiple projects named '{name}'; remove via the GUI or pass an --id (TODO)");
    let row = find(&conn, name, &ambiguous)?;
    projects::remove(&conn, &paths, &row.id)?;
    println!("removed: {name}");
    Ok(())
}

fn import_time(name: &str, path: &Path, replace: bool, append: bool) -> CmdResult {
    if !path.exists() {
        return Err(Failure(Error::FileNotFound(path.to_owned()).to_string(), 2));
    }
    let (_, conn) = open()?;
    let project = find(&conn, name, &format!("multiple projects named '{name}'"))?;
    let rows = timelog::parse(path).map_err(|e| Failure(e.to_string(), 2))?;

    let existing = timelog::count(&conn, &project.id)?;
    if existing > 0 && !(replace || append) {
        return Err(Failure(
            format!(
                "'{name}' already has {existing} time entries; \
                 pass --append to keep them or --replace to overwrite"
            ),
            1,
        ));
    }
    if replace {
        timelog::clear(&conn, &project.id)?;
    }
    let count = timelog::insert(&conn, &project.id, &rows)?;

    let total: i64 = rows.iter().map(|r| r.minutes).sum();
    println!(
        "imported: {count} entries ({:.2}h) into {name}",
        total as f64 / 60.0
    );
    Ok(())
}
