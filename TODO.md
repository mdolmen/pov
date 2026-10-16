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

Tasks get a second mark next to "next": "queue" = committed to, but not yet next. A task is backlog, queued or next, never two. Both marks live in `selected_tasks`, so the done-task cleanup and the project-deletion cascade cover both for free. In the split view (Phase 14) queued tasks stay in place, like next ones, marked by an amber band instead of violet. Next up is unchanged: it only reads next tasks.

### Backend (`pov-core`)

- [x] `db::init`: `selected_tasks.stage` column (`TEXT NOT NULL DEFAULT 'next'`), added with `has_column`; existing rows become `next`
- [x] `TaskView.is_queued`
- [x] `is_selected` means `stage = 'next'` only: `selected_hashes` (so `next_up`) reads next rows only
- [x] `tasks::select` upserts `stage = 'next'` (`ON CONFLICT … DO UPDATE`, moves a queued task up)
- [x] `tasks::queue` upserts `stage = 'queue'`; `tasks::unqueue` deletes the row
- [x] `Project.queued_count`; `selected_count` counts `next` only
- [x] `queue_task` / `unqueue_task` in `api::dispatch`
- [x] Tests (`db`, `tasks`, `projects`, `api`):
  - [x] old `selected_tasks` without `stage` migrates to `next`
  - [x] select ↔ queue moves, never two rows
  - [x] unqueue removes the row
  - [x] counts per stage
  - [x] done queued task removed by `toggle` and `cleanup_selected`
  - [x] queued tasks absent from `next_up`

### Frontend

- [x] `Task.is_queued`, `Project.queued_count` in `types.ts`
- [x] `queueTask` / `unqueueTask` in `backend.ts`
- [x] `queue` / `unqueue` in `useTasks`, optimistic like `select` (one stage clears the other)
- [x] Amber `--pov-queue` / `--pov-queue-soft` tokens (+ `--color-queue*`) in `index.css`
- [x] `TaskRow`:
  - [x] amber inset band for queued tasks
  - [x] backlog: "Queue" and "Mark next" pills on hover
  - [x] queued: "Queued" pill (click → backlog), "Mark next" on hover
  - [x] next: "Next" pill unchanged (click → backlog)
  - [x] double-click: backlog ↔ next, queued → next
- [x] `ProjectTile`: "N next · M queued of K open", queued muted, regular weight
- [x] `ProjectTiles` ordering unchanged (next count)
- [x] Remove number of queued tasks from the subtitle of the project card
- [x] Add a notification tip on project card to show number of queued tasks
- [x] Add a "Queued" button beside "Hide done" to show only queued tasks
- [x] Choose a way to add action to queued tasks (delete, move to backlog, move to next): see Inbox below

### Tests (`tests/ui`)

- [x] New `queue-task.test.tsx`:
  - [x] queue from backlog
  - [x] queued → next, queued → backlog
  - [x] a task is never queued and next at once
  - [x] tile counts
  - [x] queued task absent from Next up
  - [x] done queued task loses its mark

### Inbox (DECISIONS.md, 2026-10-12)

Rename "queued" to "inbox": tasks added by external tools, waiting for triage. Triage actions: "Mark next", "Keep" (to backlog), red × (delete, with undo).

Rename:

- [x] `db::init`: `UPDATE selected_tasks SET stage = 'inbox' WHERE stage = 'queue'` (idempotent)
- [x] Core: `tasks::queue` / `unqueue` → `tasks::inbox` / `keep`; `TaskView.is_queued` → `in_inbox`; `Project.queued_count` → `inbox_count`
- [x] API: `queue_task` / `unqueue_task` → `inbox_task` / `keep_task`
- [x] Frontend: `types.ts`, `backend.ts`, `useTasks` (`inbox` / `keep`), `--pov-inbox*` tokens (amber kept)
- [x] Backlog tasks lose the "Queue" pill: only external tools put tasks in the inbox
- [x] Tile badge: inbox count, `aria-label` "N in inbox"
- [x] Filter pill "Queued" → "Inbox"
- [x] Rename `queue-task.test.tsx` → `inbox.test.tsx`; tests seed the inbox through the bridge (`inbox_task`)
- [x] Test: migration turns `queue` rows into `inbox`

Triage actions on an inbox task:

- [x] "Mark next" pill on hover (unchanged)
- [x] "Keep" pill on hover → backlog (replaces the "Queued" pill)
- [x] Red × on hover, far right, apart from the pills → delete
- [x] Double-click → next (unchanged)

Delete:

- [x] `tasks::delete(conn, paths, project_id, hash) -> Result<Deleted>`: removes the task line and its indented block (subtasks, notes)
- [x] `Deleted { line_number, text }`: the removed lines, for undo
- [x] Writes in place (hardlink), keeps CRLF, commits `activity: <id>.md` like `toggle`, deletes the `selected_tasks` row
- [x] `tasks::restore(conn, paths, project_id, line_number, text)`: inserts the lines back at `line_number` (clamped to the end of file), commits
- [x] `delete_task` / `restore_task` in `api::dispatch`
- [x] Tests: task alone, task with subtasks, last line without newline, CRLF, duplicate text deletes the right occurrence, hardlink still shares the inode, unknown hash `NotFound`, restore puts the file back byte for byte
- [x] `deleteTask` / `restoreTask` in `backend.ts`; `remove` in `useTasks` (optimistic, refetch on error)
- [x] Undo toast at the bottom of the task pane: "Deleted “<task>” · Undo", 5 s, one at a time (a new delete replaces it)
- [x] Undo → `restore_task`, then `inbox_task` on the restored hash, refetch
- [x] UI tests: × removes the task and the file line, Undo restores both and the inbox mark, toast disappears, tile badge count follows

## Phase 12 — MCP server

See DECISIONS.md (2026-10-13).

### Core (`pov-core`, sync, no MCP types)

- [x] Marks follow shifted lines (DECISIONS.md, 2026-10-13):
  - [x] `remap`: diff of the data repo's `HEAD` version against the file, for outside edits
  - [x] `remap_splice`: exact pairing around the lines `add` / `delete` / `restore` changed
  - [x] `edit`: DB write lock, sync pending edits, edit, move marks, commit; used by `toggle` / `delete` / `restore`
  - [x] Watcher calls `sync_marks`; app startup calls `sync_all`
  - [x] Tests: shifted duplicate, chain of 3, no history, no hardlink, delete / undo, watcher
- [x] `tasks::append_task(path, text, section) -> Result<String>`, returns the new task's hash:
  - [x] appends `- [ ] <text>` at the end of the section under heading `section` (exact text, any level)
  - [x] a section ends at the next heading of the same or higher level
  - [x] `None` appends at end of file
  - [x] writes in place (never rename-replace, which breaks the hardlink)
  - [x] keeps CRLF files CRLF
  - [x] `Error::Invalid` on empty or multi-line text, or an unknown heading
- [x] `tasks::sections(path)`: the file's headings (text, level)
- [x] `projects::resolve(conn, query)`: exact id, else case-insensitive name; error listing candidates on miss / ambiguity
- [x] `tasks::add(conn, paths, project_id, text, section, stage)`:
  - [x] `append_task`
  - [x] commit `activity: <id>.md` on the hardlink, like `toggle`
  - [x] `select` / `inbox` per `stage` (`inbox | backlog | next`)
- [x] Tests:
  - [x] append at end of file
  - [x] append inside a section followed by another heading
  - [x] append inside a section with `###` children
  - [x] missing trailing newline
  - [x] CRLF kept
  - [x] hardlink still shares the inode
  - [x] duplicate text gets a distinct hash
  - [x] unknown heading and multi-line text rejected
  - [x] `resolve` by id, by name in any case, ambiguous, unknown
  - [x] `add` persists the stage and commits

### Server (`pov-cli mcp`)

- [x] `rmcp` (`server`, `transport-io`) + `tokio` (`rt`, `macros`, `io-std`), optional, behind the `cli` feature (core stays sync)
- [x] Split `src/bin/pov-cli.rs` into `src/bin/pov-cli/{main,mcp}.rs`
- [x] Tool `list_projects()`: id, name, type, status, task / next / inbox counts, open first (`projects::list`)
- [x] Tool `list_sections(project)`
- [x] Tool `add_task(project, text, section?, stage = "inbox")`: returns the task hash
- [x] Core errors become tool errors (`is_error`), not protocol errors
- [x] Each call opens its own connection and runs the core in `spawn_blocking` (busy timeout covers the app)
- [x] `pov mcp` subcommand serves over stdio until stdin closes
- [x] Test (`tests/cli.rs`): spawn `pov-cli mcp` on a temp `HOME`:
  - [x] `initialize`, `tools/list`
  - [x] `tools/call add_task`: task in the file, stage in `pov.db`

### Integration

- [x] Refetch projects, Next up and the open task list on window focus (`useTasks` exposes `refresh`)
- [x] UI test: write the file directly, fire `focus`, the task shows
- [x] README + NOTES.md: `claude mcp add pov -- ~/.local/bin/pov mcp` (absolute path: GUI clients lack the shell `PATH`)
- [x] README + NOTES.md: generic JSON config for other clients; debug `pov-cli` uses `pov-dev`
- [x] `scripts/build-cli.sh` release build still bundles
- [x] Record the `pov-cli` and `.dmg` size change from `rmcp` + `tokio` in NOTES.md

## Phase 13 — Port the backend to Rust

See DECISIONS.md (2026-10-08).

### Guidelines

- [x] Edit `CLAUDE.md`: add a "Rust style" section next to "Python style"
  (removed at the end of this phase):
  - [x] `cargo fmt` and `cargo clippy -- -D warnings` clean on every commit
  - [x] Errors: `thiserror` enum in the core, `anyhow` only in the CLI binary commands return `Result<T, String>` mapped at the boundary; no `unwrap()` / `expect()` outside tests and startup invariants
  - [x] Core logic in plain functions over `&Connection` / `&Path`, no Tauri types, so the app, the CLI and tests share it; Tauri commands are thin wrappers
  - [x] `serde` structs mirror `types.ts` field names (`#[serde(rename_all)]` only when needed); one struct per response shape
  - [x] Prefer owned, simple types (`String`, `Vec`, `PathBuf`) over lifetime and generics until profiling says otherwise; no `unsafe`
  - [x] Sync code (`rusqlite`, `std::process`) run via `tauri::async_runtime::spawn_blocking` from commands; no async runtime in the core
  - [x] Few, boring dependencies; `Cargo.lock` committed; pin the edition and `rust-version`
  - [x] Tests next to the code (`#[cfg(test)] mod tests`), real SQLite in a `tempfile::TempDir`, no mocks — same rule as the Python suite
  - [x] `///` doc comments on public items; naming per Rust API guidelines

### Safety net

- [x] Golden outputs: script that runs each Python endpoint against
  `fixtures/` and saves the JSON; the Rust tests assert the same output
- [x] Per module, port the pytest cases first and see them fail, then
  implement; tests and code land in the same commit

### Core (`pov-core` crate, `src-tauri/core/`)

- [x] `storage`: data dir (`pov-dev` under `cfg(debug_assertions)` or
  `POV_ENV=dev`), folder + config + git repo init
- [x] `db`: `rusqlite` (bundled), schema + `PRAGMA table_info` migrations,
  `foreign_keys = ON` on every connection
- [x] `tasks`: parser + `task_hash` byte-identical to Python (sha256 of
  the normalized line, first 16 hex, `\x00#n` occurrence suffix). Mind
  `str.splitlines()` vs `lines()` (`\x0c`, `\x1c`, ``...) and the
  non-UTF-8 → empty list fallback
- [x] Test: hashes equal the Python ones on every fixture, including
  duplicate lines and odd line separators
- [x] `projects` (hardlink + fallback, config.json), `activity` (`git`
  subprocess, mtime fallback), `timelog` (TIME.md parser)
- [x] `watcher`: `notify` crate on `projects/` and `learning/`, `.md`
  modify → `git add` + `git commit`; started in Tauri `setup`

### Commands (one commit per router, frontend switched in the same commit)

- [x] `pov-core::api::dispatch` + a single `api` Tauri command; test-only HTTP bridge
  (`pov-bridge`) so the UI tests call the same dispatch
- [x] tasks → projects → timelog → activity through the dispatch
- [x] Frontend: replace `apiFetch` calls (14, in hooks and modals) with
  typed wrappers over `invoke("api")` in `src/lib/backend.ts`
- [x] Remove the Python server: sidecar spawn, `get_backend_port`, `tauri-plugin-shell` +
  `shell:*` capabilities, `pov-backend` in `externalBin`, `main.py` and the routers (the
  Python CLI stays until the CLI block)

### CLI

- [x] `pov` as a second binary in the crate (`src/bin/pov-cli.rs`, `clap`):
  `add`, `list`, `remove`, `import-time` (+ `mcp` via `rmcp` if Phase 12
  is done), same flags and output as today
- [x] Test: CLI cases from `test_cli.py`
- [x] Bundle it next to the app executable; `install_cli` unchanged

### Cleanup

- [x] Delete `backend/`, `build.py`, `beforeBuildCommand` Python step
- [x] README: prerequisites (no uv), dev, test (`cargo test`), build
- [x] `CLAUDE.md`: drop the Python style section
- [x] Check cold launch time, memory and `.dmg` size before / after (NOTES.md)

## Tests

UI feature tests (Vitest + React Testing Library, real backend, see DECISIONS.md), written
before the Phase 13 Commands block as its safety net.

- [x] Setup: Vitest + jsdom, backend on a temp `HOME`, Tauri IPC mocks
- [x] Project list: counts, tabs, ordering, archived section, empty state
- [x] Sidebar: open/close, open/archived filters per tab, settings link
- [x] Task list: headings and tasks, back to the right tab, markdown links, open in editor
- [x] Toggle task: moves to Done, file updated, activity commit, untoggle
- [x] Subtasks: single toggle, last subtask completes parent, parent cascade
- [x] Next tasks: select with "+" and double-click, unselect, done unselects, card count,
  cleanup
- [x] Add project: dialog, name from folder, hardlink + commit, learning tab, archived status
- [x] Project settings: rename, pause date, archive, delete with confirmation
- [x] Activity heatmap: per-day counts, hide/show per tab
- [x] Time tracking: stepper, record time, topic suggestions, activity, hide/show per project
- [x] Settings: install CLI, already installed, install error
- [x] Smoke test: launch the real app, Rust storage init and watcher commit

## Phase 14 — Split view UI

See DECISIONS.md (2026-10-09, split view). Reference mock: `debug/ui/v2.html` (gitignored;
open it in a browser). Built on proposal D of `debug/ui/index.html`.

### Backend

- [x] `tasks::next_up(conn)`: one entry per open project, its first `##` milestone (file
  order) holding a selected, not-done task. `###` belongs to the `##` above it; `#` is the file
  title and ignored. Fields: `project_id`, `project_name`, `project_type`, `milestone`
  (`Option<String>`), `task_text` and `task_hash` (the first next task when `milestone` is
  `None`),
  `next_count`, `done`, `total` (the task's subtasks when `milestone` is `None`). Ordered by
  `next_count` desc, then project name
- [x] Test: first milestone wins; `###` counts in its `##`; next tasks before any `##` give a
  task entry with subtask progress; done tasks don't count as next; archived projects
  excluded; ties ordered by name
- [x] `next_up` in `api::dispatch`; `listNextUp()` in `backend.ts`, `NextUpEntry` in `types.ts`
- [x] `activity` command: optional `project_id` limits the git log to that project's hardlink
  (8-week row on each tile); test

### Frontend

- [x] Window 1040×760 in `tauri.conf.json` (min width so both panes fit)
- [x] Color tokens in `index.css` (ground, pane, surface, ink, muted, line, next violet,
  activity ramp) in place of the inline hex values in the components this phase touches
- [x] `App`: split layout, left pane 400px on the darker ground, right pane lighter, hairline
  between; `selectedProject` stays in `App`; Esc closes the right pane
- [x] Right pane, nothing selected: "Choose a project to show its tasks"
- [x] Left header: date, "Next up" + entry count, refresh / add / menu (the existing `Sidebar`
  drawer opens over the left pane)
- [x] `NextUp`: one row per entry: progress ring (done / total), project name (bold), milestone
  or task text below, "N next" pill (milestones only), "x of y done"; click selects the
  project and scrolls the right pane to that milestone or task; selected row highlighted
- [x] Empty Next up: "Nothing marked next" + how to mark tasks
- [x] `ProjectTiles` replaces the `ProjectCard` list: two columns, Projects / Learning segmented
  control on the caption, tile = name, 8-week activity row, "N next of M open"; quiet style
  for older / no activity; selected tile ringed in violet; Archived disclosure under the
  tiles, per the sidebar filters
- [x] Footer: 30-day activity strip for the current tab; click shows the full
  `ActivityHeatmap` (6 months, fits 400px); shown / hidden remembered per tab as today
- [x] `TaskList` → right pane: no back button, × closes; tasks in file order under milestone
  captions with "x of y done"; next tasks keep their place, marked by the violet band, with a
  "Mark next" / "Next" button (double-click kept); no separate Next or Done sections
- [x] Done tasks shown by default, struck through in place; "Hide done" in the header, saved
  per project in `localStorage` (`pov.done.<id>.hidden`, like `pov.time.<id>.visible`);
  "All tasks are done" + "Show done" when nothing is left
- [x] Learning projects keep the time heatmap at the bottom of the right pane
- [x] Toggling or (un)selecting a task refreshes Next up and the tile counts
- [x] Settings view and the modals unchanged

- [x] No sidebar: the burger opens Settings; archived projects always show, collapsed (see
  DECISIONS.md, no sidebar)

### Tests (`tests/ui`)

- [x] Update: project list (tiles, segmented tabs, archived), task list (captions, done
  inline), select task ("Mark next", band in place), toggle task (done stays in place),
  activity heatmap (strip, expand), sidebar (drawer over the left pane)
- [x] Replace "back returns to the tab": closing the pane keeps the tab
- [x] New `next-up.test.tsx`: one entry per project, ordering, milestone / task fallback,
  click opens and scrolls, updates after select and toggle, empty state
- [x] New `hide-done.test.tsx`: hides per project, survives a remount, all-done state

## Phase 15 — Integration with other tools

See DECISIONS.md (2026-10-16). A project links to external tools. "Delegate" on a task hands it to one: the task moves to the `delegated` stage and pov starts the tool, unless it's running. The tool reads its tasks and writes back through `pov mcp` (subtasks, links), then sends the task back to the inbox.

### Task done rule

- [x] `Task::is_done`: own checkbox only, subtasks ignored
- [x] `toggle_cascade`: checking a task checks its open subtasks
- [x] `toggle_cascade`: unchecking a task leaves its subtasks as they are
- [x] `toggle_cascade`: toggling a subtask never changes its parent
- [x] `task_count`, `next_up`, `cleanup_selected` follow the new rule
- [x] Tests: all subtasks checked, parent open → task open
- [x] Tests: check parent → open subtasks checked; uncheck parent → subtasks unchanged
- [x] Tests: subtask toggle leaves the parent line untouched
- [x] UI test: checking the last subtask leaves the task open

### Reworded tasks

- [x] `remap`: a diff block replacing N checkbox lines with N at the same indentation pairs them in order
- [x] Blocks that change the line count still drop their marks
- [x] The same pairing moves `selected_tasks` and `task_meta` rows
- [x] Rename test `remap_pairs_toggled_lines_and_drops_reworded_ones`
- [x] Tests: reword a next task, an inbox task, a task with metadata
- [x] Tests: reword two neighbouring tasks; 2 lines → 1 drops both
- [x] Test: watcher picks up a reword made in an editor

### Metadata (`pov-core`)

- [x] `db::init`: `task_meta` table:
  - [x] `id` INTEGER PRIMARY KEY: the stable task handle for tools
  - [x] `project_id` (FK → projects, ON DELETE CASCADE), `task_hash`, UNIQUE(project_id, task_hash)
  - [x] `tool` TEXT nullable: the tool the task is delegated to
  - [x] `data` JSON TEXT: `note?`, `links?: [{label, target}]`, `source?` (opaque to pov)
- [x] `tasks::add` takes optional metadata and stores the row
- [x] `TaskView.meta`: `id`, `note`, `links`, `has_source`; `TaskView.delegated_to`
- [x] Done tasks keep their metadata; `cleanup_selected` leaves `task_meta` alone
- [x] `sync_marks` drops `task_meta` rows whose task is gone, as for marks
- [x] `delete` returns the metadata in `Deleted`; `restore` puts it back
- [x] Tests: add with metadata, view fields, kept when done, delete / restore, project removal cascade

### Delegation (`pov-core`)

- [x] `config.json` `tools`: `{ "<name>": { "command": "<abs path>", "pidfile": "<abs path>" } }`, edited by hand
- [x] `config.json` project entry: `tools: ["<name>"]`, edited by hand
- [x] `Project.tools` read from config; unknown tool names ignored
- [x] `delegated` stage in `selected_tasks` (no migration: free text column)
- [x] `Project.delegated_count`
- [x] `tasks::delegate(conn, paths, project_id, hash, tool, note)`:
  - [x] `Error::Invalid` if the task has no metadata or the tool isn't linked to the project
  - [x] sets the stage to `delegated`, `task_meta.tool`, and `note` in `data`
- [x] `tasks::delegated(conn, paths, tool)`: `id`, project, text, subtasks, `data`
- [x] `tasks::add_subtask(conn, paths, id, text, checked)`:
  - [x] appends after the task's last subtask, with its indent (2 spaces when none)
  - [x] in place, CRLF kept, committed like `toggle`, marks moved by `remap_splice`
- [x] `tasks::add_links(conn, id, links)`: appends to `data.links`
- [x] `tasks::finish(conn, id)`: stage → `inbox`, `tool` cleared
- [x] Unknown `id` → `NotFound` everywhere
- [x] `delegate_task` in `api::dispatch`
- [x] Tests: each function, delegate refused without metadata / unlinked tool, finish lands in the inbox

### Tool launch (app)

- [ ] `delegate` Tauri command: `tasks::delegate`, then starts the tool unless it's running
- [ ] Running: pov's child for that tool hasn't exited (`try_wait`), or the pidfile's process is alive (`kill -0`)
- [ ] Start: `command` detached, stdio null, never waited on
- [ ] Start failure: error returned to the UI, task stays delegated (the next click retries)
- [ ] Test: fake tool script; two delegations while it runs start it once

### MCP (`pov mcp`)

- [ ] `add_task`: optional `metadata` object
- [ ] Tool `list_delegated(tool)`
- [ ] Tool `add_subtask(id, text, checked = true)`
- [ ] Tool `add_links(id, links)`
- [ ] Tool `finish_task(id)`
- [ ] Test (`tests/cli.rs`): add with metadata, delegate, list, add a subtask and a link, finish; check file and DB

### Frontend

- [ ] `types.ts`: `Task.meta`, `Task.delegated_to`, `Project.tools`, `Project.delegated_count`
- [ ] `backend.ts`: `delegateTask`
- [ ] "Delegate" pill on hover, on tasks with metadata in projects with tools
- [ ] Several tools: the pill opens a menu of them
- [ ] Delegate opens a popover: optional note, "Delegate" confirms
- [ ] Delegated tasks: band in a new colour (`--pov-delegated*` tokens), "<tool>" pill, no actions
- [ ] Info icon on tasks with metadata → popover with the note and the links
- [ ] Link click: `http(s)` → opener `openUrl`, else `openPath`
- [ ] "Delegated" filter pill beside "Inbox"
- [ ] Start failure → error toast
- [ ] New `delegate.test.tsx`:
  - [ ] pill shown only with metadata and tools; menu with several tools
  - [ ] note sent, band and pill shown, no actions
  - [ ] info popover lists links; clicks call the opener (`mockIPC`)
  - [ ] filter; task back in the inbox after `finish_task`

### Docs

- [ ] README + NOTES.md: `tools` in `config.json`, project `tools`
- [ ] README + NOTES.md: tool contract (MCP calls, pidfile, absolute command path)

## Maintenance

- [x] Fix `pov add`, returns 128

Python quirks kept by the Rust port (DECISIONS.md, exact parity); fix one per commit, after
Phase 13:

- [x] `_normalize` replaces every `[.]` in a line, not just the checkbox: tasks differing only
  by `[a]` / `[b]` in their text share a hash (told apart by occurrence only)
- [x] `_set_checked` replaces the first `[ ]` / `[x]` of the line: checking an already checked
  parent ticks a `[ ]` written in its text
- [x] Toggling a task rewrites a CRLF file with LF line endings
- [x] `[-]`, `[~]` and other states count as unchecked, and toggling can't clear them
- [x] `task_count` counts unchecked subtasks too
- [x] Project activity level passes the original path to `git log` in the data repo, which is
  outside it: the level always comes from the file's mtime
- [x] Heatmap buckets commits by the commit's own timezone, not local time
- [x] NaN / infinite hours in TIME.md crash `import-time` (Rust: "bad duration" error)
- [x] Adding an already tracked file links and commits, then crashes on the DB constraint
  (Rust: refused before any side effect)

## Post-MVP

- [ ] Analytics on file edit history
