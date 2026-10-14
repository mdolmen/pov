# pov — Projects Overview

## Idea

Centralize all the TODO.md files across projects into a minimalist, locally-run desktop app.
Read-only markdown rendering with interactive checkboxes that sync back to the source files.

## Stack

| Layer | Technology |
|---|---|
| App shell | Tauri (window, file browser, packaging) |
| Frontend | Vite + React + TailwindCSS + Shadcn |
| Backend | Rust `pov-core` crate, called in-process through one Tauri command |
| File watching | `notify` crate |
| Persistence | SQLite (`rusqlite`) + a git repo of hardlinks |
| CLI | `pov-cli`, a binary of `pov-core` (`clap`) |

Everything runs locally. The app packages into a native macOS `.app`, launchable without any terminal.
The backend started as a Python FastAPI sidecar and was ported to Rust (DECISIONS.md, 2026-10-08).

## Data & File Layout

```
~/.local/share/pov/    # pov-dev/ for dev builds (debug or POV_ENV=dev)
  config.json          # list of projects (id, name, path)
  pov.db               # SQLite database
  projects/
    <project-id>.md    # hardlink → original TODO.md (same inode)
  learning/
    <project-id>.md    # hardlink → e.g. Maths/TODO.md
  .git/                # git repo tracking all hardlinked files
```

Hardlinks (not symlinks) are used so the pov folder shares the same inode as the original file. Changes to either path are immediately visible to both. The pov folder is a git repo — the watcher detects changes and auto-commits (`activity: <id>.md`), giving a clean activity history per project independent of the original repo. Writes from the app go through the hardlink and reach the original file directly.

Fallback: files on a different filesystem can't be hardlinked — use mtime-only activity tracking for those (edge case).

Activity tracking = last git commit date in the pov repo for that file (falls back to mtime).

## Architecture

```
Tauri (.app, one process)
├── WebView  →  Vite + React frontend
│                 └── invoke("api", {cmd, args})   via src/lib/backend.ts
└── Rust     →  pov_core::api::dispatch
                ├── tasks      list, toggle checkbox, select / unselect, cleanup
                ├── projects   list, add, update, remove
                ├── time       per-day totals, add entry, topics
                └── activity   heatmap from the data repo's git log
                + watcher thread (notify) committing edits to the data repo
```

The frontend calls a single `api` Tauri command; `pov_core::api::dispatch` maps the command name
to a core function. No HTTP server, no port. Tauri itself handles:
- Regular app with Dock icon (standard macOS window, movable and resizable)
- Native file browser dialog (to pick a TODO.md when adding a project)
- Opening a TODO.md in the default text editor
- Installing the CLI (Settings → Install CLI)

The UI tests reach the same dispatch through `pov-bridge`, a test-only HTTP server that never ships.

## CLI

```bash
pov add <path/to/TODO.md> [--name N] [--type project|learning] [--status open|archived]
pov list                          # list tracked projects
pov remove <name>                 # remove a project (keeps original file)
pov import-time <name> <TIME.md>  # import a time log [--append | --replace]
pov mcp                           # MCP server on stdio, spawned by an MCP client
```

The CLI is `pov-cli`, a binary of `pov-core` bundled in the `.app` and copied to
`~/.local/bin/pov` by Settings → Install CLI. It shares the core with the app, so it writes the
same `pov.db`, config.json and hardlinks; the app doesn't need to be running.

### MCP server (`pov mcp`)

Lets external tools add tasks to a tracked project (DECISIONS.md, 2026-10-13). Tools:
`list_projects`, `list_sections`, `add_task` (stage `inbox` by default, or `backlog` / `next`).
Built with `rmcp` + `tokio`, behind the `cli` feature: the app and `pov-core` stay sync. Each
call opens its own connection and runs the core on a blocking thread.

Register it with an absolute path: GUI clients don't get the shell's `PATH`.

```sh
claude mcp add --scope user pov -- ~/.local/bin/pov mcp   # the shell expands ~
```

Other clients:

```json
{ "mcpServers": { "pov": { "command": "/Users/<you>/.local/bin/pov", "args": ["mcp"] } } }
```

A debug `pov-cli` uses `pov-dev`, like the app: register `src-tauri/target/debug/pov-cli mcp`
as a separate server (e.g. `pov-dev`) to try changes on dev data. The app shows added tasks when
its window gains focus (projects and Next up also poll every 3 s).

Size cost of `rmcp` + `tokio`, measured 2026-10-13 (release, MiB like the table below; before:
`8283825`):

| | Before | After |
|---|---|---|
| Bundled CLI | 3.0 MB | 7.7 MB |
| `.app` | 18.1 MB | 22.9 MB |
| `.dmg` | 6.9 MB | 8.4 MB |

## UI

### General

- Sidebar hidden by default, toggled with a button or keyboard shortcut
- No markdown editing — read-only rendering
- Checkboxes are interactive (click to toggle done/undone)

### Sidebar structure

```
[Projects]
  Opened       → list of active projects
  Archived     → paused / done / canceled projects

[Continuous Learning]
  Maths
  Papers
  Books
  Videos
```

### Project card (in the Opened list)

```
┌─────────────────────────────────────┐
│ Project Name                  [3]   │  ← bold: number of "selected" tasks
│ 12 tasks                            │
└─────────────────────────────────────┘
```

Border color (Opened):
- Grey — no activity this month
- Light green — activity this month
- Green — activity this week

Border color (Archived):
- Grey — paused
- Red — canceled
- Green — done

### Adding a project

- "+" button on the main project list
- Opens a native file browser dialog (via Tauri) to select a TODO.md
- Prompts for project name and status (open/archived + sub-status)

## Features

### Task list (inside a project)

- Renders the TODO.md as markdown (headings, checkboxes, plain text)
- Click a checkbox → toggles done/undone in the UI and writes to the original file
- Double-click a task (or click the "+" icon aligned to the right) → marks it as "selected" (handle next) — persisted in SQLite
- Top-right edit icon → opens the file in the default text editor
- Done tasks move to the bottom of the list (UI only, the file is unchanged)

### Activity registration

- Any checkbox toggle from the app is committed to the data repo
- External edits are detected by the watcher and committed too
- The project card's color comes from the latest commit for its file (mtime fallback), or the
  latest recorded time entry if more recent
- A per-tab heatmap at the bottom of the project list counts these commits per day

### Selected tasks

- "Selected" = flagged as "handle next"
- State persisted in SQLite, keyed by the task's content hash (survives line shifts)
- Count of selected tasks shown on the project card; projects are ordered by it, then by their
  8-day activity
- "Next" filters the task pane to the open next tasks; "Inbox" and "Next" are exclusive
- "Inbox" = added by an external tool, waiting for triage: amber band; a task is next, in the
  inbox or neither (`selected_tasks.stage`). Triage: "Mark next", "Keep" (to backlog), red ×
  (deletes the task and its indented block from the file, Undo for 5 s). Next up ignores inbox
  tasks; the tile shows the inbox count as a badge; "Inbox" filters the task pane

## Continuous Learning

Same task list rendering as projects, under a separate "learning" tab.

Learning projects also have time tracking: a blue heatmap below the task list, entries added
from a modal (15-minute steps, optional topic) or imported from a TIME.md with
`pov import-time`. Time entries live in SQLite only.

## Out of scope for MVP

- Analytics on file edits
- Any sync or cloud features

## Rust port: before / after

Measured 2026-10-09 on an M-series Mac, release `.dmg`, 4 launches each on an empty temp `HOME`
(first launch, then range). Before: `8cba122`, the last commit with every route served by the
Python sidecar. After: Rust only.

| | Before (Python sidecar) | After (Rust) |
|---|---|---|
| `.dmg` | 36.2 MB | 7.1 MB |
| `.app` | 44.5 MB | 18.0 MB |
| Bundled CLI | 9.7 MB | 3.0 MB |
| Window on screen | 2.18 s (1.22–2.18) | 0.59 s (0.37–0.59) |
| Backend ready | 2.60 s (1.72–2.60) | at window (in-process) |
| RSS after 5 s | 85–123 MB | 56–65 MB |
| Processes | 3 (app + PyInstaller bootloader + server) | 1 |

RSS sums the app's own process tree; WebKit's XPC helpers (not children of the app) are left out
on both sides. "Backend ready" is when the sidecar starts listening on its port.
