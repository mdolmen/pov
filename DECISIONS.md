# pov — Decisions

## 2026-10-08 — Port the backend from Python to Rust

- **Decision:** replace the Python sidecar (FastAPI + PyInstaller) with Rust inside the existing
  Tauri crate. The React frontend stays; HTTP routes become Tauri commands.
- **Why:** longevity, stability, startup time and memory.
  - One toolchain fewer (no Python, uv or PyInstaller).
  - No localhost port: no `CORS *` without auth, no port race, no orphaned backend.
  - No `--onefile` unpack at launch.
- **Constraints:**
  - Same `pov.db` schema and `~/.local/share/pov/` layout, so no data migration.
  - Port whatever Python exists at the time (Phases 11 and 12 if they land first).
  - Every commit leaves the app working: Python serves the routes not yet ported.
- **Plan:** TODO.md, Phase 13.

## 2026-10-08 — Rust core as a separate `pov-core` crate

- **Decision:** domain logic goes in `src-tauri/core/` (`pov-core`, Cargo workspace member, no
  Tauri dependency). The Tauri app depends on it; the CLI becomes a binary of it.
- **Why:** `cargo test -p pov-core` runs without compiling Tauri; the CLI doesn't link Tauri; a
  module named `core` would clash with Rust's built-in `core` crate.

## 2026-10-08 — The Rust port copies Python behavior exactly

- **Decision:** byte-for-byte parity with Python, quirks included, checked by golden outputs
  recorded from the Python code.
- **Why:** golden tests can then tell a porting bug from a deliberate change.
- **Constraints:** quirks found are listed in TODO.md › Maintenance and fixed later, one commit
  each.

## 2026-10-08 — UI tests with Vitest + React Testing Library

- **Decision:** one test file per UI feature, Vitest + React Testing Library in jsdom, against the
  real backend on a temp data dir; each test writes its own TODO.md files. Only native calls are mocked
  (`mockIPC`: backend port, file dialog, open in editor, install CLI, opener).
- **Why:** no framework can drive the Tauri window on macOS (WKWebView has no automation
  protocol; tauri-driver is Linux/Windows only). Vitest reuses the Vite config and is fast.
- **Constraints:**
  - No CSS in jsdom: hover visibility and layout aren't tested.
  - Rust startup (storage, db, watcher) is covered by a separate launch smoke test.
  - Once routes become Tauri commands, the tests reach them through a test-only bridge.
- **Plan:** written now, before the Phase 13 Commands block, as its safety net.

## 2026-10-08 — One command dispatch shared by the app and the UI tests

- **Decision:** `pov-core::api::dispatch(paths, cmd, args)` maps a command name and JSON args
  to a core call. The app exposes it as a single `api` Tauri command; the UI tests reach it
  through `pov-bridge`, a test-only HTTP server behind the `bridge` feature.
- **Why:** the tests run the same command table as the app, so a route can't work in tests and
  be missing or wired differently in the app.
- **Constraints:** the frontend calls typed wrappers in `src/lib/backend.ts`, never
  `invoke("api")` directly; `pov-bridge` never ships.

## 2026-10-09 — Rust CLI and the end of Python

- **Decision:** the CLI is `pov-cli`, a `pov-core` binary behind the `cli` feature; all Python is
  removed, including `dev/seed.py` and `dev/golden.py`.
- **Why:** a `pov-core` binary named `pov` would overwrite the app binary in `target/`;
  `install_cli` already installs `pov-cli` as `~/.local/bin/pov`. Parity is done, so the goldens
  no longer need a recorder.
- **Constraints:**
  - A debug `pov-cli` uses `pov-dev`, like the app (the Python dev CLI used prod data).
  - The goldens are frozen fixtures; new behavior is tested in Rust only.

## 2026-10-09 — Checkbox states and open task count

- **Decision:** any checkbox state other than `[ ]` (`[x]`, `[-]`, `[~]`, …) closes a task;
  toggling a closed task writes `[ ]`, toggling an open one writes `[x]`. A project's
  `task_count` counts its top-level tasks that aren't done.
- **Why:** `[-]` (cancelled) and `[~]` (deferred) were open forever, since toggling couldn't
  clear them; counting subtasks inflated the card count against what the task list shows open.

## 2026-10-09 — Split view UI with Next up

- **Decision:** one window, two panes: projects on the left under a "Next up" list, the
  selected project's tasks on the right. Next up shows one entry per open project: its first
  `##` milestone holding a task marked next (the task itself when it sits before any `##`),
  ordered by that milestone's next count, then name. In the task pane, done tasks stay in place
  by default; "Hide done" is saved per project.
- **Why:** opening on what to handle next beats opening on where it lives; milestones keep
  Next up short and say which phase each project is in; no back-and-forth navigation.
- **Constraints:**
  - The window grows from 600×800 to 1040×760.
  - Next up reads every open project's file: a new `next_up` command, not N `list_tasks`
    calls.
  - "Hide done" lives in `localStorage`, like the time heatmap toggle: UI state, not data.
- **Plan:** TODO.md, Phase 14. Mock: `debug/ui/v2.html`.

## 2026-10-09 — No sidebar

- **Decision:** the sidebar and its open / archived filters are gone. The burger button opens
  Settings directly; the archived projects of a tab always show, under a collapsed "Archived
  (n)" disclosure, when there are any.
- **Why:** the filters were rarely changed, and the drawer was one more step to reach Settings.

## 2026-10-09 — Queue stage

- **Decision:** a second task mark, "queued" (committed to, not next yet), stored as
  `selected_tasks.stage` (`next` | `queue`); a task is next, queued or neither. Queued tasks stay
  in place in the task pane with an amber band and "Queue" / "Queued" text pills, like "Mark
  next" / "Next".
- **Why:** separates what's planned from what's handled now, without a new table: done-task
  cleanup and project deletion already cover the rows.
- **Constraints:** Next up reads next tasks only; a project with only queued tasks isn't in it.
  Tile ordering stays by next count.
- **Plan:** TODO.md, Phase 11.

## 2026-10-12 — Queue becomes Inbox

- **Decision:** the "queued" stage is renamed "inbox" (`selected_tasks.stage = 'inbox'`): tasks
  added by external tools (Phase 12), waiting for triage. An inbox task offers "Mark next",
  "Keep" (to backlog) and a red × that deletes it from the file, with an undo toast. The tile
  badge and the filter count the inbox. The UI no longer puts tasks in the inbox by hand.
- **Why:** the queue was meant for tool-added tasks; triage needs a reject action, and one
  click per task with undo beats a confirmation dialog.
- **Constraints:**
  - Deleting edits the user's file: in place (hardlink), CRLF kept, committed to the data repo
    like a toggle, so git history keeps every deleted task.
  - Phase 12 `add_task` defaults to `stage = "inbox"`.
- **Plan:** TODO.md, Phase 11 › Inbox.

## 2026-10-13 — MCP server

- **Decision:** external tools (Claude Code, other MCP clients) add tasks to a tracked project
  through a stdio MCP server, spawned by the client as `pov mcp` (the `pov-cli` binary).
- **Why:** works whether the app is open or not.
- **Constraints:**
  - Like the CLI, it works on the data dir directly: writes the project's file in place and
    commits it to the data repo itself, as `toggle` does, since the watcher only runs with the
    app.
  - Needs Phase 11 for the `inbox` stage.
- **Plan:** TODO.md, Phase 12.

## 2026-10-13 — Marks follow shifted lines

- **Decision:** when a file's lines shift, next / inbox marks move to their tasks' new hashes.
  Edits from outside pov (editor, another process) are diffed against the data repo's last
  commit of the file. pov's own edits (add, delete, restore) use the exact lines they changed.
- **Why:** a hash counts identical lines, so inserting `x` above a marked `x`, or deleting the
  first of two, moved the mark to the wrong task. A diff can't tell neighbouring identical lines
  apart, so pov's own edits don't rely on it.
- **Constraints:**
  - The DB write lock (`BEGIN IMMEDIATE`) is held from the diff to the git commit, so the app's
    watcher and `pov mcp` never move the same edit's marks twice.
  - The app syncs every project at startup, for edits made while it was closed.
  - Not covered: projects without a hardlink (no history), reworded lines, and moved lines
    that have duplicates.

## 2026-10-13 — Tile bars: last 8 days, counting repo commits

- **Decision:** a tile's two bars of four show the last 8 days, one cell per day: oldest at the
  top of the left bar, today at the bottom of the right one (they were 8 weeks, one cell per
  week). They count activity like the bottom heatmap: the commits of the git repo holding the
  project's file, else its `activity:` commits in the data repo.
- **Why:** the bands only counted TODO.md edits through the hardlink, so code work didn't show,
  and an editor saving by rename (Claude Code's Edit tool does) broke the hardlink and froze them.
- **Constraints:** projects sharing a repo show the same bars. The tile's activity level (dimmed
  tile) still reads the hardlink's last `activity:` commit.

## 2026-10-14 — Tiles ordered by next tasks, then activity

- **Decision:** project tiles are ordered by next task count, then by activity over the tile's
  8 days (the summed shades of its cells, then the commits they count), then by activity level
  (this week, this month, older, none: greyed-out tiles after idle white ones), then by name.
- **Why:** among projects without next tasks, the ones being worked on come first.
- **Constraints:** the list fetches the 8-day counts of its tab's projects and passes them to
  the tiles; tiles may reorder once those counts arrive. Next up keeps its own order.

## 2026-10-16 — A task is done by its own checkbox

- **Decision:** a task is done when its own checkbox is checked, whatever its subtasks. Checking
  a task checks its open subtasks; checking or unchecking a subtask never changes its parent.
- **Why:** a tool that delegates work adds its subtasks already checked; the task must stay open
  for review. Replaces "a task with subtasks is done when all of them are checked" (Phase 6).
- **Constraints:** hand-edited files with an open parent over checked subtasks show that task
  open again.
- **Plan:** TODO.md, Phase 15 › Task done rule.

## 2026-10-16 — Marks and metadata follow reworded tasks

- **Decision:** when a diff block replaces N checkbox lines with N checkbox lines at the same
  indentation, they are paired in order: marks and metadata move to the new lines. Tools refer
  to a task by its metadata ID, never by its hash.
- **Why:** rewording a task lost its next / inbox mark, and would lose a delegated task's link to
  its tool.
- **Constraints:**
  - A block that changes the number of lines still drops them.
  - Metadata of a task gone from the file is deleted; its mark row stays, so an undo in the
    editor brings the mark back.
- **Plan:** TODO.md, Phase 15 › Reworded tasks.

## 2026-10-16 — Delegating tasks to external tools

- **Decision:** a project lists tools in `config.json`. "Delegate" on a task moves it to a
  `delegated` stage and starts the tool, unless it's already running. The tool reads its tasks
  and writes back through `pov mcp`: subtasks, links in the task's metadata, then the task back to
  the inbox. A tool that fails adds an open subtask with the reason and sends the task back too.
- **Metadata:** optional JSON per task in SQLite, set by the tool that adds the task (triage-mail:
  the source mail, saved as `.eml`), plus the note typed when delegating and the links tools add.
  An info icon on the task shows the note and opens the links (URL in the browser, path with
  the default app).
- **Why:** pull, not push: pov stays without a server or network code, and nothing is lost when
  the tool is down. A tool run once per delegation needs no daemon on an 8 GB laptop.
- **Constraints:**
  - One instance per tool: pov starts it only when its child has exited and the tool's pidfile
    names no live process; the tool also takes that pidfile as a lock.
  - No undelegate: a delegated task has no actions until the tool sends it back.
  - "Delegate" shows only on tasks with metadata, in projects with tools; several tools give a
    menu.
  - Tool commands are absolute paths: the app has no shell `PATH`.
- **Plan:** TODO.md, Phase 15. First tool: epc-os (local Ollama), drafts mails and documents.

## 2026-10-16 — Next up subtitle: nearest heading

- **Decision:** a Next up milestone is the nearest `##` or deeper heading above the next task,
  not only `##`; `done` / `total` count the tasks under that heading.
- **Why:** a phase split into `###` steps says more by naming the step being worked on.

## 2026-10-16 — Delegate any task, tools in Settings

- **Decision:** "Delegate" shows on every open task in a project with tools; delegating a task
  without metadata creates its row. Tools are defined in Settings (name, command, pidfile) and
  linked in Project settings, written to `config.json`; editing it by hand still works.
- **Why:** only a tool could give a task metadata, and tools were hand-edited JSON: in practice
  nothing could be delegated.
- **Constraints:**
  - The tool, not the stage, makes a task delegated: a next / inbox mark left on it is ignored.
  - Command and pidfile are absolute paths; removing a tool unlinks it from every project.
- **Plan:** TODO.md, Phase 15 › Delegate any task, Tools in Settings. Replaces "Delegate shows
  only on tasks with metadata" (2026-10-16, delegating tasks to external tools).
