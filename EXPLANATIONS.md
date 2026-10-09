# pov — Architecture & Concepts

Personal reference. Explains the role of every folder and file, and the concepts behind the stack choices.

---

## Big picture

The app has three layers, all inside a single process:

```
┌──────────────────────────────────────────────────────────┐
│  macOS (.app)                                            │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │   Tauri (Rust) — the shell                         │  │
│  │                                                    │  │
│  │  ┌───────────────┐   invoke()   ┌───────────────┐  │  │
│  │  │  WebView      │ ───────────► │  pov-core     │  │  │
│  │  │  (React app)  │ ◄─────────── │  the brain    │  │  │
│  │  │  the face     │     JSON     │               │  │  │
│  │  └───────────────┘              │ - TODO.md I/O │  │  │
│  │                                 │ - SQLite      │  │  │
│  │                                 │ - git         │  │  │
│  │                                 │ - file watch  │  │  │
│  │                                 └───────────────┘  │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

- **Tauri** is the native macOS wrapper. It creates the window and handles file dialogs and other OS features. Think of it as the "OS glue".
- **React** (inside Tauri's WebView) is the UI. A WebView is essentially an embedded browser — your React app runs in it exactly like a website, except it's bundled inside the `.app`.
- **pov-core** is the backend: a Rust library (`src-tauri/core/`) that reads and writes the TODO.md files, manages SQLite, runs git and watches files. It has no Tauri code, so the app, the `pov` CLI and the tests all share it.

These parts talk to each other:
- React → pov-core: `invoke("api", { cmd, args })`, a Tauri command call. Tauri serializes the arguments to JSON, runs `pov_core::api::dispatch` on a background thread and sends the JSON result back. No HTTP, no port.
- React → Tauri: other `invoke()` calls for native features (open in editor, install the CLI), plus plugins (file dialog, opener)
- Terminal → pov-core: the `pov` CLI is another binary built from the same crate

The backend used to be a Python FastAPI server running as a sidecar subprocess on localhost; it was ported to Rust for startup time, memory and one toolchain fewer (DECISIONS.md, NOTES.md for the numbers).

---

## Root-level files

### `package.json`
The Node.js project manifest. Declares the frontend dependencies (React, Tailwind, Shadcn…) and the scripts you run (`npm run dev`, `npm run build`, `npm run tauri dev`). Think of it as the `Cargo.toml` of the frontend world.

### `package-lock.json`
Auto-generated. Records the exact version of every installed package (including dependencies of dependencies). You never edit this. Its purpose is reproducibility — `npm install` on any machine produces the exact same result.

### `node_modules/`
Where npm puts all downloaded packages. Never commit this. Never edit anything in here.

### `tsconfig.json` and `tsconfig.node.json`
TypeScript configuration. TypeScript is JavaScript with type annotations — it catches bugs at write time rather than at runtime. These files tell the TypeScript compiler what rules to enforce and how to resolve imports.

- `tsconfig.json` — rules for the React app code (`src/`)
- `tsconfig.node.json` — rules for the build tooling code (`vite.config.ts`)

The split exists because the React app runs in a browser environment, while the build config runs in Node.js — different globals available.

### `vite.config.ts`
Configuration for **Vite**, the build tool and development server. Vite takes your TypeScript + React source files and:
- In dev mode: serves them instantly with hot reload (change a file, the browser updates in ~50ms)
- In build mode: bundles and minifies everything into `dist/` for production

The file also configures the `@` alias — anywhere in the code you write `import X from "@/components/..."` and it resolves to `src/components/...`. Cleaner than relative paths.

### `components.json`
Configuration for **Shadcn**. Tells it where to put components, what Tailwind version to use, what style to apply. You don't edit this often.

### `index.html`
The single HTML file that bootstraps the entire React app. It contains nothing except a `<div id="root">` and a `<script>` that loads the compiled React bundle. React then takes over and renders everything inside that div.

### `.gitignore`
Files and folders git should not track: `node_modules/` (huge, reproducible), `dist/` (generated), `target/` (Rust build artifacts).

### `TODO.md`, `DECISIONS.md`, `NOTES.md`, `CLAUDE.md`
The build plan (phases and checkboxes), the log of design decisions with their reasons, the product notes, and the coding guidelines for Claude Code.

---

## `src/` — The React frontend

This is the UI layer. Everything here is TypeScript + React, compiled by Vite into a bundle that runs in Tauri's WebView.

### `src/main.tsx`
The entry point. This is where React starts. It finds the `<div id="root">` in `index.html` and renders the top-level `<App />` component into it. You rarely touch this file.

### `src/App.tsx`
The root component. Holds the app-level state (open project, active tab, sidebar, filters, settings) and switches between the three views.

### `src/views/`
One file per screen:
- `ProjectList.tsx` — the project cards, with the projects / learning tabs and the activity heatmap
- `TaskList.tsx` — one project's TODO.md: next tasks on top, then the backlog, done tasks at the bottom; time heatmap for learning projects
- `SettingsView.tsx` — Install CLI

### `src/index.css`
The global stylesheet. The first line `@import "tailwindcss"` loads the entire Tailwind CSS framework. Shadcn then appends its design tokens (colors, radius, fonts) below.

### `src/App.css`
Leftover from the Tauri template, no longer imported.

### `src/components/`
Where all UI components live. A component is a self-contained piece of UI — a button, a card, a sidebar. Each component is a function that returns HTML-like syntax (JSX). Here: `ProjectCard`, `Sidebar`, the modals (`AddProjectModal`, `ProjectSettingsModal`, `AddTimeModal`), and the heatmaps (`Heatmap` draws the grid, `ActivityHeatmap` and `TimeHeatmap` feed it data).

#### `src/components/ui/`
Auto-generated Shadcn components. Shadcn is not a traditional component library (like Bootstrap) — it copies the source code of each component directly into your project. You own the code. This means you can read and modify every component freely, unlike a black-box library.

Added with `shadcn add <component>`: `button`, `dialog`, `input`, `select`.

### `src/hooks/`
Custom React hooks that load data and keep it in component state: `useProjects` (project list) and `useTasks` (one project's tasks, with optimistic updates on toggle and select).

### `src/lib/backend.ts`
Typed wrappers over `invoke("api")`, one per backend command (`listTasks`, `toggleTask`, `addProject`…). Components and hooks always go through these, never call `invoke("api")` directly.

### `src/lib/utils.ts`
A single utility function `cn()` added by Shadcn. It merges Tailwind class names intelligently, handling conflicts. Example: `cn("p-4 p-2")` correctly resolves to `"p-2"` instead of both being applied.

### `src/types.ts`
TypeScript shapes of the data the backend returns (`Project`, `Task`, `TimeDay`…). The Rust `serde` structs use the same field names.

### `src/assets/`
Static files (images, SVGs) used directly in the React code. Currently only the template's unused `react.svg`.

### `src/vite-env.d.ts`
TypeScript type declarations for Vite-specific features (like `import.meta.env`). Auto-generated, don't touch.

---

## `src-tauri/` — The Tauri shell and the Rust core

This is the native layer. It wraps the React app, creates the window, and hosts the backend.

### `src-tauri/tauri.conf.json`
The main Tauri configuration. Defines:
- The app window (size, decorations, visibility on start)
- The build hooks: `beforeDevCommand` starts Vite; `beforeBuildCommand` builds the frontend and the CLI (`scripts/build-cli.sh`)
- Bundle settings (icon files, app identifier, `.dmg` target, and `externalBin` — the `pov-cli` binary copied next to the app executable)
- macOS-specific options (`macOSPrivateApi: true`)

### `src-tauri/Cargo.toml`
The Rust project manifest — equivalent of `package.json` for Rust. Declares Rust dependencies ("crates") and their features. Cargo is the Rust package manager. It is also the root of a Cargo **workspace**: the app (`pov`) and the core (`pov-core`) are built together and share one `target/` and one `Cargo.lock`.

### `src-tauri/Cargo.lock`
Same role as `package-lock.json` — locks exact dependency versions. Unlike Node projects (where you often gitignore the lock), you always commit `Cargo.lock` for applications.

### `src-tauri/src/lib.rs`
The Tauri app. Kept thin:
- `setup`: finds the data dir, initializes it (folders, git repo, database) and starts the file watcher
- The `api` command: hands `cmd` and `args` to `pov_core::api::dispatch` on a background thread
- Native commands: `open_in_editor`, `install_cli`, `is_cli_installed`

`lib.rs` contains the logic as a library. `main.rs` just calls `run()` from `lib.rs`. The split is a Tauri convention for technical reasons (cross-platform compilation).

### `src-tauri/core/` — `pov-core`
The backend, as a plain Rust library with no Tauri dependency, so it can be tested with `cargo test -p pov-core` without compiling Tauri. One module per concern in `src/`:
- `paths`, `storage` — where the data dir is and how it is created
- `db` — SQLite connection, schema and migrations
- `tasks` — TODO.md parsing, task hashes, toggling checkboxes, selected tasks
- `projects` — adding (hardlink + git commit), listing, updating, removing projects
- `activity` — activity levels and the heatmap, from the data repo's git log
- `timelog` — time entries and the TIME.md importer
- `watcher` — commits edited files to the data repo
- `git` — runs the `git` command
- `pyline` — text helpers that reproduce Python's string handling, so task hashes match the ones the Python backend stored
- `api` — the command table used by the app and the UI tests
- `error` — the crate's error type

Binaries in `src/bin/`, each behind a Cargo **feature** so they are only built on request:
- `pov-cli.rs` (`cli` feature) — the `pov` command-line tool
- `pov-bridge.rs` (`bridge` feature) — a test-only HTTP server in front of `api::dispatch`, used by the UI tests; never shipped

`tests/` holds integration tests: `cli.rs` runs the real CLI binary, `golden.rs` checks the parser against outputs recorded from the old Python backend (`tests/golden/`, frozen).

### `src-tauri/build.rs`
Runs at compile time before the app is built. Calls `tauri_build::build()` which generates some glue code from `tauri.conf.json`. You never edit this.

### `src-tauri/capabilities/default.json`
Tauri v2's permission system. Explicitly declares what APIs the frontend JavaScript is allowed to call: the core defaults plus the opener and dialog plugins.

### `src-tauri/binaries/`
Where `scripts/build-cli.sh` puts the release CLI, named `pov-cli-<target triple>` as Tauri's `externalBin` expects. Gitignored.

### `src-tauri/icons/`
App icon in every size required by macOS, Windows, and Linux. The `.icns` file is the macOS format (multi-resolution bundle). The `.ico` is Windows. Tauri generates all sizes from a single source icon.

### `src-tauri/gen/schemas/`
Auto-generated JSON schemas that Tauri uses internally for config validation. Never edit.

### `src-tauri/target/`
Rust build artifacts. Very large (several GB). Gitignored. Regenerated on every build.

---

## `tests/ui/` — UI tests
Vitest + React Testing Library, one file per feature. They render the real React app in jsdom and run the real Rust commands through `pov-bridge` on a temp `HOME`; only native calls (file dialog, editor, CLI install) are faked. `globalSetup.ts` builds and starts the bridge, `helpers.tsx` seeds projects through it.

## `scripts/`
- `build-cli.sh` — builds the release `pov-cli` for bundling
- `smoke-app.sh` — launches the real debug app on a temp `HOME` and checks startup and the file watcher

## `fixtures/`
Sample TODO.md files used to try the app by hand.

---

## `public/`
Static files that Vite copies verbatim into `dist/` without processing. Useful for assets that need a stable URL. Currently just holds the default Tauri and Vite SVG logos; `vite.svg` is still the page favicon.

---

## `dist/`
The compiled frontend output produced by `npm run build`. Tauri bundles this folder into the `.app`. Gitignored — it's generated, not authored.

---

## Concepts cheat sheet

| Term | What it means |
|---|---|
| **Component** | A reusable piece of UI, written as a TypeScript function that returns JSX |
| **JSX** | HTML-like syntax inside TypeScript files. `<Button onClick={...}>` compiles to regular JS |
| **Props** | Parameters passed to a component, like function arguments |
| **Hook** | A React function (starting with `use`) that adds behaviour to a component — `useState` for local state, `useEffect` for side effects |
| **TypeScript** | JavaScript + types. Types describe what shape data has, caught at compile time |
| **Tailwind** | A CSS framework where you style things with utility classes directly on elements: `className="flex gap-4 text-sm font-bold"` |
| **Shadcn** | A set of pre-built, accessible UI components (button, dialog, card…) copied into your project as source code |
| **Vite** | The dev server and bundler. In dev mode it's instant. In prod it compiles everything into optimised static files |
| **WebView** | An embedded browser inside a native app. Tauri renders your React app in one |
| **Tauri command** | A Rust function the frontend can call with `invoke()`. Arguments and results travel as JSON |
| **Crate** | A Rust library/package (equivalent of an npm package) |
| **Cargo** | The Rust package manager (equivalent of npm) |
| **Workspace** | Several crates built together, sharing one `target/` and `Cargo.lock` (here: the app and `pov-core`) |
| **Feature** | An optional part of a crate, switched on at build time (`--features cli`) |
| **SQLite** | A file-based SQL database. The entire database is a single `.db` file, no server required |
| **notify** | A Rust crate that watches the filesystem for changes and fires callbacks |
| **Hardlink** | A second name for the same file on disk (same inode). Editing either path edits both |
