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

Tasks get a second mark next to "next": "queue" = committed to, but not yet next. A task is
backlog, queued or next, never two. Both marks live in `selected_tasks`, so the done-task
cleanup and the project-deletion cascade cover both for free. In the split view (Phase 14)
queued tasks stay in place, like next ones, marked by an amber band instead of violet. Next up
is unchanged: it only reads next tasks.

### Backend (`pov-core`)

- [ ] `db::init`: `selected_tasks.stage` column (`TEXT NOT NULL DEFAULT 'next'`), added with `has_column`; existing rows become `next`
- [ ] `TaskView.is_queued`
- [ ] `is_selected` means `stage = 'next'` only: `selected_hashes` (so `next_up`) reads next rows only
- [ ] `tasks::select` upserts `stage = 'next'` (`ON CONFLICT … DO UPDATE`, moves a queued task up)
- [ ] `tasks::queue` upserts `stage = 'queue'`; `tasks::unqueue` deletes the row
- [ ] `Project.queued_count`; `selected_count` counts `next` only
- [ ] `queue_task` / `unqueue_task` in `api::dispatch`
- [ ] Tests (`db`, `tasks`, `projects`, `api`):
  - [ ] old `selected_tasks` without `stage` migrates to `next`
  - [ ] select ↔ queue moves, never two rows
  - [ ] unqueue removes the row
  - [ ] counts per stage
  - [ ] done queued task removed by `toggle` and `cleanup_selected`
  - [ ] queued tasks absent from `next_up`

### Frontend

- [ ] `Task.is_queued`, `Project.queued_count` in `types.ts`
- [ ] `queueTask` / `unqueueTask` in `backend.ts`
- [ ] `queue` / `unqueue` in `useTasks`, optimistic like `select` (one stage clears the other)
- [ ] Amber `--pov-queue` / `--pov-queue-soft` tokens (+ `--color-queue*`) in `index.css`
- [ ] `TaskRow`:
  - [ ] amber inset band for queued tasks
  - [ ] backlog: "Queue" and "Mark next" pills on hover
  - [ ] queued: "Queued" pill (click → backlog), "Mark next" on hover
  - [ ] next: "Next" pill unchanged (click → backlog)
  - [ ] double-click: backlog ↔ next, queued → next
- [ ] `ProjectTile`: "N next · M queued of K open", queued muted, regular weight
- [ ] `ProjectTiles` ordering unchanged (next count)

### Tests (`tests/ui`)

- [ ] New `queue-task.test.tsx`:
  - [ ] queue from backlog
  - [ ] queued → next, queued → backlog
  - [ ] a task is never queued and next at once
  - [ ] tile counts
  - [ ] queued task absent from Next up
  - [ ] done queued task loses its mark

## Phase 12 — MCP server

Let external tools (Claude Code, other MCP clients) add tasks to a tracked project. A stdio MCP
server spawned by the client as `pov mcp` (the `pov-cli` binary), independent of the app being
open. Like the CLI it works on the data dir directly: it writes the project's file in place and
commits it to the data repo itself, as `toggle` does, since the watcher only runs with the app.
Needs Phase 11 for the `queue` stage.

### Core (`pov-core`, sync, no MCP types)

- [ ] `tasks::append_task(path, text, section) -> Result<String>`, returns the new task's hash:
  - [ ] appends `- [ ] <text>` at the end of the section under heading `section` (exact text, any level)
  - [ ] a section ends at the next heading of the same or higher level
  - [ ] `None` appends at end of file
  - [ ] writes in place (never rename-replace, which breaks the hardlink)
  - [ ] keeps CRLF files CRLF
  - [ ] `Error::Invalid` on empty or multi-line text, or an unknown heading
- [ ] `tasks::sections(path)`: the file's headings (text, level)
- [ ] `projects::resolve(conn, query)`: exact id, else case-insensitive name; error listing candidates on miss / ambiguity
- [ ] `tasks::add(conn, paths, project_id, text, section, stage)`:
  - [ ] `append_task`
  - [ ] commit `activity: <id>.md` on the hardlink, like `toggle`
  - [ ] `select` / `queue` per `stage` (`backlog | queue | next`)
- [ ] Tests:
  - [ ] append at end of file
  - [ ] append inside a section followed by another heading
  - [ ] append inside a section with `###` children
  - [ ] missing trailing newline
  - [ ] CRLF kept
  - [ ] hardlink still shares the inode
  - [ ] duplicate text gets a distinct hash
  - [ ] unknown heading and multi-line text rejected
  - [ ] `resolve` by id, by name in any case, ambiguous, unknown
  - [ ] `add` persists the stage and commits

### Server (`pov-cli mcp`)

- [ ] `rmcp` (`server`, `transport-io`) + `tokio` (`rt`, `macros`, `io-std`), optional, behind the `cli` feature (core stays sync)
- [ ] Split `src/bin/pov-cli.rs` into `src/bin/pov-cli/{main,mcp}.rs`
- [ ] Tool `list_projects()`: id, name, type, status, task / next / queued counts, open first (`projects::list`)
- [ ] Tool `list_sections(project)`
- [ ] Tool `add_task(project, text, section?, stage = "backlog")`: returns the task hash
- [ ] Core errors become tool errors (`is_error`), not protocol errors
- [ ] Each call opens its own connection and runs the core in `spawn_blocking` (busy timeout covers the app)
- [ ] `pov mcp` subcommand serves over stdio until stdin closes
- [ ] Test (`tests/cli.rs`): spawn `pov-cli mcp` on a temp `HOME`:
  - [ ] `initialize`, `tools/list`
  - [ ] `tools/call add_task`: task in the file, stage in `pov.db`

### Integration

- [ ] Refetch projects, Next up and the open task list on window focus (`useTasks` exposes `refresh`)
- [ ] UI test: write the file directly, fire `focus`, the task shows
- [ ] README + NOTES.md: `claude mcp add pov -- ~/.local/bin/pov mcp` (absolute path: GUI clients lack the shell `PATH`)
- [ ] README + NOTES.md: generic JSON config for other clients; debug `pov-cli` uses `pov-dev`
- [ ] `scripts/build-cli.sh` release build still bundles
- [ ] Record the `pov-cli` and `.dmg` size change from `rmcp` + `tokio` in NOTES.md

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
