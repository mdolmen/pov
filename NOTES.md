# pov — Projects Overview

## Idea

Centralize all the TODO.md files across projects into a minimalist, locally-run desktop app.
Read-only markdown rendering with interactive checkboxes that sync back to the source files.

## Stack

| Layer | Technology |
|---|---|
| App shell | Tauri (system tray, file browser, packaging) |
| Frontend | Vite + React + TailwindCSS + Shadcn |
| Backend | Python FastAPI (Tauri sidecar) |
| File watching | Python watchdog |
| Persistence | SQLite |

Everything runs locally. The app packages into a native macOS `.app`, launchable without any terminal.

## Data & File Layout

```
~/.local/share/pov/
  config.json          # list of projects (path, name, status, type)
  pov.db               # SQLite database
  projects/
    my-project.md      # hardlink → original TODO.md (same inode)
    other-project.md   # hardlink → original TODO.md
  learning/
    maths.md           # hardlink → Maths/TODO.md
    papers.md          # hardlink → Papers/TODO.md
    books.md           # hardlink → Books/TODO.md
    videos.md          # hardlink → Videos/TODO.md
  .git/                # git repo tracking all hardlinked files
```

Hardlinks (not symlinks) are used so the pov folder shares the same inode as the original file. Changes to either path are immediately visible to both. The pov folder is a git repo — watchdog detects changes and auto-commits, giving a clean activity history per project independent of the original repo. Writes from the app go through the hardlink and reach the original file directly.

Fallback: files on a different filesystem can't be hardlinked — use mtime-only activity tracking for those (edge case).

Activity tracking = last git commit date in the pov repo for that file (falls back to mtime).

## Architecture

```
Tauri (.app)
├── WebView  →  Vite + React frontend
└── Sidecar  →  Python FastAPI server (http://127.0.0.1:PORT)
                ├── /projects      CRUD + metadata
                ├── /tasks         read, toggle checkbox, mark selected
                └── /activity      mtime-based activity data
```

The frontend talks to the FastAPI sidecar over localhost. Tauri handles:
- System tray icon (menu bar, top-right) — no Dock icon (`activation_policy = "Accessory"`)
- Click tray icon → floating panel appears near the icon; click away → hides
- Right-click tray icon → context menu: Show, Quit
- Native file browser dialog (to pick a TODO.md when adding a project)
- App lifecycle (start/stop the sidecar)

## CLI

```bash
pov add <path/to/TODO.md>   # add a project from terminal
pov list                    # list tracked projects
pov remove <name>           # remove a project (keeps original file)
```

The CLI is a thin Python script, installed alongside the app. It writes to the same config.json and creates the symlink.

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
- Top-right edit icon → opens the file in vim in a terminal window

### Activity registration

- Any checkbox toggle from the app registers activity on the project (updates a timestamp in SQLite)
- File mtime changes (from external edits) are detected by watchdog and update activity

### Selected tasks

- "Selected" = flagged as "handle next"
- State persisted in SQLite (keyed by file path + line number or task content hash)
- Count of selected tasks shown on the project card

## Continuous Learning

Same task list rendering as projects. Each section (Maths, Papers, Books, Videos) maps to a single TODO.md file.

Maths is the only section with time tracking (TIME.md). Deferred to post-MVP.

## Out of scope for MVP

- TIME.md / time tracking
- Analytics on file edits
- Any sync or cloud features
