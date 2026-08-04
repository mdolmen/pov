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

## Maintenance

- [ ] Fix `pov add`, returns 128

## Post-MVP

- [ ] Analytics on file edit history
