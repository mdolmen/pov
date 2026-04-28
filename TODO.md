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

- [ ] Test setup: pytest + pytest-asyncio, in-memory SQLite, temp dir fixtures
- [ ] `GET /projects` — list all projects with metadata (name, status, task count, selected count, last activity)
- [ ] Test: `GET /projects` returns correct metadata
- [ ] `POST /projects` — add a project: create hardlink, git add, initial commit, write config.json
- [ ] Test: `POST /projects` creates hardlink, writes config.json, makes initial git commit
- [ ] Test: `POST /projects` falls back gracefully when hardlink fails (different filesystem)
- [ ] `DELETE /projects/:id` — remove a project: delete hardlink, remove from config.json (original file untouched)
- [ ] Test: `DELETE /projects/:id` removes hardlink and config entry, original file untouched
- [ ] `PATCH /projects/:id` — update name, status (open / paused / done / canceled)
- [ ] Test: `PATCH /projects/:id` updates name and status correctly
- [ ] Activity computation: read `git log` on the hardlinked file, classify as this week / this month / older / none
- [ ] Fallback to mtime for files that couldn't be hardlinked (different filesystem)
- [ ] Test: activity classifies correctly as this week / this month / older / none from git log output
- [ ] Test: activity falls back to mtime when git log is unavailable
- [ ] watchdog: watch `~/.local/share/pov/projects/` and `learning/` — on file change, `git add + git commit`
- [ ] Test: file change triggers git add + commit (mock subprocess)

## Phase 3 — Backend: tasks

- [ ] `GET /projects/:id/tasks` — parse TODO.md, return list of tasks (text, checked, line number, content hash)
- [ ] Test: parser handles checked, unchecked, nested subtasks, plain headings, mixed content
- [ ] Test: content hash is stable across line number shifts (insert a line above)
- [ ] `PATCH /projects/:id/tasks/:hash` — toggle checkbox: rewrite the correct line in the file, update git
- [ ] Test: toggle checkbox rewrites the correct line in the file
- [ ] Test: a task with all subtasks checked is considered done
- [ ] `POST /projects/:id/tasks/:hash/select` — mark task as selected (handle next), persist in SQLite
- [ ] `DELETE /projects/:id/tasks/:hash/select` — unmark selected
- [ ] Test: select persists in SQLite; unselect removes it
- [ ] Task identity keyed by content hash (robust to line number shifts from external edits)

## Phase 4 — Frontend: project list

- [ ] Main view: grid/list of project cards
- [ ] Project card: name, task count, selected count (bold, right-aligned), activity border color
- [ ] Border color logic: grey / light green / green based on last activity date
- [ ] Archived section with grey / red / green border based on sub-status
- [ ] "+" button → triggers Tauri native file browser dialog → calls `POST /projects`
- [ ] Modal after file selection: enter project name, choose status

## Phase 5 — Frontend: sidebar

- [ ] Sidebar component, hidden by default
- [ ] Toggle button (or keyboard shortcut) to show/hide
- [ ] Section: Projects → Opened, Archived
- [ ] Section: Continuous Learning → Maths, Papers, Books, Videos
- [ ] Navigate to project on click

## Phase 6 — Frontend: task list view

- [ ] Markdown renderer: headings, plain text, checkboxes (no edit)
- [ ] Checkbox click → `PATCH /projects/:id/tasks/:hash` → optimistic UI update
- [ ] "+" icon on hover at right of each task → marks as selected
- [ ] Double-click on task → marks as selected
- [ ] Selected tasks visually distinct (on top of the list, a separator between those and the rest)
- [ ] When a task is selected it moves to the top section of the list
- [ ] Top-right edit icon → Tauri command to open file in vim in a terminal window
- [ ] A task with subtask (markdown title) is considered done when all of the subtasks are done
- [ ] Done tasks are moved to the bottom of the list, in the interface only, no changes to the underlying file

## Phase 7 — CLI

- [ ] Python script `pov` (entry point via `pyproject.toml`)
- [ ] `pov add <path>` — create hardlink, update config.json, git add + commit
- [ ] `pov list` — print tracked projects
- [ ] `pov remove <name>` — remove hardlink + config entry
- [ ] Install path: "Install CLI" option in tray menu, symlinks `pov` into `/usr/local/bin`

## Phase 9 — Continuous Learning

- [ ] Add Maths, Papers, Books, Videos as fixed entries in config (type = "learning")
- [ ] Same task list view as projects
- [ ] Sidebar entries navigate to the correct learning section

## Phase 10 — Packaging

- [ ] Configure Tauri bundler for macOS `.app`
- [ ] Bundle Python backend as a self-contained binary (PyInstaller)
- [ ] App icon
- [ ] Test cold launch (no terminal, no dev tools)
- [ ] Sign and notarize for macOS Gatekeeper (or document how to bypass for local use)

## Post-MVP

- [ ] TIME.md / time tracking for Maths
- [ ] Analytics on file edit history
- [ ] Auto-update mechanism
