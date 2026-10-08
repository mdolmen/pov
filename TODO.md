# pov — TODO

## Phase 1 — Project scaffold

- [x] Init Tauri v2 project with Vite + React + TypeScript
- [x] Configure TailwindCSS + Shadcn
- [x] Configure `activation_policy = "Accessory"` (no Dock icon)
- [x] Set up system tray icon with right-click menu (Show, Quit)
- [x] Configure floating panel window (click away to hide)
- [x] Init Python FastAPI project structure (`backend/`)
- [x] Configure Tauri sidecar to bundle and launch the Python backend
- [x] Port negotiation: Tauri picks a free port, passes it to the frontend via env
- [x] Init SQLite schema (projects, tasks_selected, activity)
- [x] Init `~/.local/share/pov/` folder structure on first launch
- [x] Init git repo in `~/.local/share/pov/` on first launch

## Phase 2 — Backend: projects

- [x] Test setup: pytest + pytest-asyncio, in-memory SQLite, temp dir fixtures
- [x] `GET /projects` — list all projects with metadata (name, status, task count, selected count, last activity)
- [x] Test: `GET /projects` returns correct metadata
- [x] `POST /projects` — add a project: create hardlink, git add, initial commit, write config.json
- [x] Test: `POST /projects` creates hardlink, writes config.json, makes initial git commit
- [x] Test: `POST /projects` falls back gracefully when hardlink fails (different filesystem)
- [x] `DELETE /projects/:id` — remove a project: delete hardlink, remove from config.json (original file untouched)
- [x] Test: `DELETE /projects/:id` removes hardlink and config entry, original file untouched
- [x] `PATCH /projects/:id` — update name, status (open / paused / done / canceled)
- [x] Test: `PATCH /projects/:id` updates name and status correctly
- [x] Activity computation: read `git log` on the hardlinked file, classify as this week / this month / older / none
- [x] Fallback to mtime for files that couldn't be hardlinked (different filesystem)
- [x] Test: activity classifies correctly as this week / this month / older / none from git log output
- [x] Test: activity falls back to mtime when git log is unavailable
- [x] watchdog: watch `~/.local/share/pov/projects/` and `learning/` — on file change, `git add + git commit`
- [x] Test: file change triggers git add + commit (mock subprocess)

## Phase 3 — Backend: tasks

- [x] `GET /projects/:id/tasks` — parse TODO.md, return list of tasks (text, checked, line number, content hash)
- [x] Test: parser handles checked, unchecked, nested subtasks, plain headings, mixed content
- [x] Test: content hash is stable across line number shifts (insert a line above)
- [x] `PATCH /projects/:id/tasks/:hash` — toggle checkbox: rewrite the correct line in the file, update git
- [x] Test: toggle checkbox rewrites the correct line in the file
- [x] Test: a task with all subtasks checked is considered done
- [x] `POST /projects/:id/tasks/:hash/select` — mark task as selected (handle next), persist in SQLite
- [x] `DELETE /projects/:id/tasks/:hash/select` — unmark selected
- [x] Test: select persists in SQLite; unselect removes it
- [x] Task identity keyed by content hash (robust to line number shifts from external edits)

## Phase 4 — Frontend: project list

- [x] Main view: grid/list of project cards
- [x] Project card: name, task count, selected count (bold, right-aligned), activity border color
- [x] Border color logic: grey / light green / green based on last activity date
- [x] Archived section with grey / red / green border based on sub-status
- [x] "+" button → triggers Tauri native file browser dialog → calls `POST /projects`
- [x] Modal after file selection: enter project name, choose status

## Phase 5 — Frontend: sidebar

- [x] Sidebar component, hidden by default
- [x] Toggle button (or keyboard shortcut) to show/hide
- [x] Section: Projects → Opened, Archived
- [x] Section: Learning → Opened, Archived

## Phase 6 — Frontend: task list view

- [x] Markdown renderer: headings, plain text, checkboxes (no edit)
- [x] Checkbox click → `PATCH /projects/:id/tasks/:hash` → optimistic UI update
- [x] "+" icon on hover at right of each task → marks as selected
- [x] Double-click on task → marks as selected
- [x] Selected tasks visually distinct (on top of the list, a separator between those and the rest)
- [x] When a task is selected it moves to the top section of the list
- [x] Top-right edit icon → Tauri command to open file in vim in a terminal window
- [x] A task with subtask (markdown title) is considered done when all of the subtasks are done
- [x] Done tasks are moved to the bottom of the list, in the interface only, no changes to the underlying file
- [x] Order projects by number of selected tasks

## Phase 7 — CLI

- [x] Python script `pov` (entry point via `pyproject.toml`)
- [x] `pov add <path>` — create hardlink, update config.json, git add + commit
- [x] `pov list` — print tracked projects
- [x] `pov remove <name>` — remove hardlink + config entry
- [x] Install path: "Install CLI" option in tray menu, symlinks `pov` into `/usr/local/bin`

## Phase 8 — Activity tracker (heatmap)

Per-tab git-style activity heatmap pinned at the bottom of the project
list. Tracks task toggles and underlying .md file edits, scoped to the
projects of the current tab (projects vs learning). Source of truth is
the existing pov data git repo — every toggle and watcher-detected file
edit already produces a commit there.

### Backend

- [x] `GET /activity?type=project|learning&days=120` — shell out to
  `git log --since="120 days ago" --pretty=format:"%aI%x09%s"` on
  `POV_DIR`, parse commit messages of the form `activity: <id>.md` /
  `add: <name>`, look up project type via the id segment, bucket by
  local date. Returns `[{date: "YYYY-MM-DD", count: N}]` for the last
  120 days.
- [x] Tests: per-type filtering, day bucketing, empty-day handling,
  graceful behavior when the git repo is empty

### Frontend

- [x] `ActivityHeatmap` component — sliding window of the last ~120 days
  (4 months, month-granularity x-axis labels), one cell per day, columns
  per week, days as rows
- [x] Quantile-based color scale (e.g. 0 / q25 / q50 / q75 / q90 of
  non-zero days), stone palette base + green ramp for filled cells
- [x] Cell tooltip on hover: `<count> activities on <date>`
- [x] Wire into `ProjectList`: instantiate per active tab (refetch when
  tab changes); fixed at the bottom of the page
- [x] Show/hide toggle:
  - "Hide" button above the heatmap, right-aligned
  - "Show" button at the bottom of the page, right-aligned, when hidden
  - Persist per tab in `localStorage` (key: `pov.heatmap.<tab>.visible`)

## Phase 9 — Packaging

- [x] Configure Tauri bundler for macOS `.app`
- [x] Bundle Python backend as a self-contained binary (PyInstaller)
- [x] App icon
- [x] Test cold launch (no terminal, no dev tools)
- [x] Sign and notarize for macOS Gatekeeper (or document how to bypass for local use)

## Phase 10 — Time tracking (learning projects)

Learning project pages get a second heatmap, below the task list, showing
time spent per day on that project (blue ramp, to distinguish it from the
green activity ramp). Time is entered by hand from a modal: a duration in
15-minute steps plus an optional topic. Unlike activity, this data has no
git source of truth — it lives in SQLite.

### Navigation

- [x] Lift `activeTab` from `ProjectList` into `App` state and pass it down
  as a controlled prop, so returning from a project page lands on the tab
  it was opened from (learning project → learning tab)

### Backend

- [x] `time_entries` table in `pov/db.py`: `id`, `project_id`
  (FK → projects, ON DELETE CASCADE), `date` (TEXT, `YYYY-MM-DD`),
  `minutes` (INTEGER, multiple of 15), `topic` (TEXT, nullable),
  `created_at`; index on `(project_id, date)`
- [x] `pov/routers/time.py`, mounted in `main.py`:
  - `GET /projects/:id/time?days=N` — per-day totals for the window,
    `[{date, minutes}]`, zero days omitted
  - `POST /projects/:id/time` — record an entry `{date, minutes, topic?}`;
    validate `minutes > 0` and `minutes % 15 == 0`, 404 on unknown project
  - `GET /projects/:id/time/topics` — distinct topics used on this project,
    most recently used first
- [x] Tests: day bucketing (several entries same day), empty window, minutes
  validation rejects non-multiples of 15 and non-positive values, unknown
  project 404, topic list ordering and de-duplication
- [x] Fold recorded time into the project card activity level: the color band
  takes the more recent of the file's git/mtime activity and the last
  recorded time entry

### Frontend

- [x] Extract the grid/scale/labels of `ActivityHeatmap` into a presentational
  `Heatmap` component (props: `data: Record<string, number>`, `months`,
  `shades`, tooltip formatter). `ActivityHeatmap` keeps the green ramp;
  no visual change to the project list
- [x] `TimeHeatmap` component — fetches `GET /projects/:id/time`, blue ramp
  (`#ebe9e2` base + 4 blue shades), tooltip `<Xh YYmin> on <date>`
- [x] Wire into `TaskList`, pinned below the task list, only when
  `project.type === "learning"`; same hide/show toggle and `localStorage`
  persistence as the activity heatmap (key: `pov.time.<projectId>.visible`)
- [x] "+" button at the top right of the time heatmap → `AddTimeModal`
- [x] `AddTimeModal`: date (defaults to today), duration stepper in 15-min
  increments, free-text topic input with suggestions from
  `GET /projects/:id/time/topics` (typing a new topic is allowed).
  On submit → `POST` then refresh the heatmap and the topic list

### Import

- [x] `pov/timelog.py` — parse a TIME.md table (`| DATE | TIME | TOPIC |`,
  hours with a comma or dot separator, rounded to 15 minutes)
- [x] `pov import-time <project> <path>` — one-shot import into the real DB;
  refuses to run twice unless `--replace`
- [x] Tests: hour → minute conversion, rounding, malformed rows, duplicate
  import guard, unknown project
- [x] `dev/seed.py` seeds `debug/TIME.md` into the Maths learning project

## Phase 11 — Queue stage

Task list goes from three piles (next / backlog / done) to four:
next → queue → backlog → done. "Queue" = committed to, but not yet
next. Same left-band visual cue as "next", in amber instead of violet.
A task is in at most one stage; both stages live in the existing
`selected_tasks` table, so done-task cleanup and project deletion
cascade cover both for free.

### Backend

- [ ] `selected_tasks.stage` column (`TEXT NOT NULL DEFAULT 'next'`,
  `'next' | 'queue'`), added via the `PRAGMA table_info` migration in
  `init_db` (existing rows become `next`)
- [ ] `TaskResponse.is_queued: bool`; `is_selected` now means
  `stage = 'next'` only
- [ ] `POST /projects/:id/tasks/:hash/select` upserts `stage = 'next'`
  (moves a queued task up)
- [ ] `POST /projects/:id/tasks/:hash/queue` upserts `stage = 'queue'`;
  `DELETE /projects/:id/tasks/:hash/queue` removes the row
- [ ] `ProjectResponse.queued_count`; `selected_count` counts `next` only
- [ ] Tests: migration keeps existing rows as `next`, select ↔ queue
  moves (never two rows), unqueue, counts per stage, `/tasks/cleanup`
  removes done queued tasks

### Frontend

- [ ] `Task.is_queued`, `Project.queued_count` in `types.ts`;
  `queue` / `unqueue` in `useTasks` with the same optimistic update
  pattern as `select`
- [ ] `QUEUED_BAND = "#f59e0b"` (amber) left band in `TaskCard`
- [ ] Queue section between "next" and backlog in `TaskList`, separated by
  a `Divider`; `renderPendingItems` skips queued tasks like selected ones
- [ ] Hover actions: backlog task gets "+" (→ next) and a second icon
  (→ queue); queued task gets "↑" (→ next) and "×" (→ backlog); next
  task keeps "×". Double-click stays backlog ↔ next
- [ ] `ProjectCard`: queued count shown muted (stone-400, regular weight)
  left of the bold next count; project ordering unchanged (next count)

## Phase 12 — MCP server

Let external tools (Claude Code, other MCP clients) add tasks to a
tracked project. Runs as a stdio MCP server spawned by the client via the
CLI (`pov mcp`), independent of the app being open. It writes the
TODO.md through the hardlink, so the watcher commits it like any external
edit, and writes stages straight to `pov.db` like the CLI does.

### Core

- [ ] `append_task(file_path, text, heading=None) -> str` in
  `pov/tasks.py`: append `- [ ] <text>` at the end of the section under
  `heading` (exact heading text match), or at end of file when `None`;
  returns the new task's hash. Reject empty / multi-line text and unknown
  headings (`ValueError`)
- [ ] Tests: append to end of file, append inside a section followed by
  another heading, trailing-newline handling, duplicate text gets a
  distinct hash (occurrence counter), unknown heading rejected

### Server

- [ ] Add the `mcp` Python SDK dependency; `pov/mcp_server.py` built with
  `FastMCP`
- [ ] Tool `list_projects()` — id, name, type, status, task / next /
  queued counts (open projects first)
- [ ] Tool `list_sections(project)` — headings of the project's TODO.md,
  so the client can target one
- [ ] Tool `add_task(project, text, section=None, stage="backlog")` —
  `stage` ∈ `backlog | queue | next`; `project` matches by id or
  case-insensitive name, error listing candidates on miss / ambiguity;
  returns the task hash
- [ ] `pov mcp` subcommand in `cli.py` runs the server over stdio
- [ ] Tests: each tool against a temp `POV_DIR` (unknown project,
  ambiguous name, stage persisted in `selected_tasks`, section targeting)

### Integration

- [ ] Refetch tasks / projects on window focus so tasks added while the
  app is open show up without navigating away
- [ ] README: how to register the server
  (`claude mcp add pov -- pov mcp`, generic JSON config for other clients)
- [ ] Check the PyInstaller bundle still builds with the `mcp` dependency
  (the CLI symlink points into the bundle)

## Phase 13 — Port the backend to Rust

Replace the Python sidecar (FastAPI + PyInstaller) with Rust inside the
existing Tauri crate, for longevity, stability and startup/memory. The
React frontend stays; HTTP routes become Tauri commands, so the
localhost port goes away (no more `CORS *` with no auth, no port race, no
orphaned backend, no `--onefile` unpack at launch). Same `pov.db` schema
and `~/.local/share/pov/` layout, so there is no data migration. Ports
whatever Python exists at the time, so Phases 11 and 12 included if
they land first. Every commit leaves the app working: Python keeps
serving the routes not yet ported.

### Guidelines

- [ ] Edit `CLAUDE.md`: add a "Rust style" section next to "Python style"
  (removed at the end of this phase):
  - [ ] `cargo fmt` and `cargo clippy -- -D warnings` clean on every commit
  - [ ] Errors: `thiserror` enum in the core, `anyhow` only in the CLI binary commands return `Result<T, String>` mapped at the boundary; no `unwrap()` / `expect()` outside tests and startup invariants
  - [ ] Core logic in plain functions over `&Connection` / `&Path`, no Tauri types, so the app, the CLI and tests share it; Tauri commands are thin wrappers
  - [ ] `serde` structs mirror `types.ts` field names (`#[serde(rename_all)]` only when needed); one struct per response shape
  - [ ] Prefer owned, simple types (`String`, `Vec`, `PathBuf`) over lifetime and generics until profiling says otherwise; no `unsafe`
  - [ ] Sync code (`rusqlite`, `std::process`) run via `tauri::async_runtime::spawn_blocking` from commands; no async runtime in the core
  - [ ] Few, boring dependencies; `Cargo.lock` committed; pin the edition and `rust-version`
  - [ ] Tests next to the code (`#[cfg(test)] mod tests`), real SQLite in a `tempfile::TempDir`, no mocks — same rule as the Python suite
  - [ ] `///` doc comments on public items; naming per Rust API guidelines

### Safety net

- [ ] Golden outputs: script that runs each Python endpoint against
  `fixtures/` and saves the JSON; the Rust tests assert the same output
- [ ] Port the pytest cases first, as failing Rust tests, router by router

### Core (`src-tauri/src/core/`)

- [ ] `storage`: data dir (`pov-dev` under `cfg(debug_assertions)` or
  `POV_ENV=dev`), folder + config + git repo init
- [ ] `db`: `rusqlite` (bundled), schema + `PRAGMA table_info` migrations,
  `foreign_keys = ON` on every connection
- [ ] `tasks`: parser + `task_hash` byte-identical to Python (sha256 of
  the normalized line, first 16 hex, `\x00#n` occurrence suffix). Mind
  `str.splitlines()` vs `lines()` (`\x0c`, `\x1c`, ``...) and the
  non-UTF-8 → empty list fallback
- [ ] Test: hashes equal the Python ones on every fixture, including
  duplicate lines and odd line separators
- [ ] `projects` (hardlink + fallback, config.json), `activity` (`git`
  subprocess, mtime fallback), `timelog` (TIME.md parser)
- [ ] `watcher`: `notify` crate on `projects/` and `learning/`, `.md`
  modify → `git add` + `git commit`; started in Tauri `setup`

### Commands (one commit per router, frontend switched in the same commit)

- [ ] tasks → projects → timelog → activity as `#[tauri::command]`s
- [ ] Frontend: replace `apiFetch` calls (14, in hooks and modals) with
  typed `invoke` wrappers in `src/lib/backend.ts`
- [ ] Remove the sidecar: `main.py` spawn in `lib.rs`, `get_backend_port`,
  `tauri-plugin-shell` + `shell:*` capabilities, `externalBin`

### CLI

- [ ] `pov` as a second binary in the crate (`src/bin/pov.rs`, `clap`):
  `add`, `list`, `remove`, `import-time` (+ `mcp` via `rmcp` if Phase 12
  is done), same flags and output as today
- [ ] Test: CLI cases from `test_cli.py`
- [ ] Bundle it next to the app executable; `install_cli` unchanged

### Cleanup

- [ ] Delete `backend/`, `build.py`, `beforeBuildCommand` Python step
- [ ] README: prerequisites (no uv), dev, test (`cargo test`), build
- [ ] `CLAUDE.md`: drop the Python style section
- [ ] Check cold launch time, memory and `.dmg` size before / after

## Maintenance

- [x] Fix `pov add`, returns 128

## Post-MVP

- [ ] Analytics on file edit history
