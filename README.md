# Projects Overview (pov)

A macOS desktop app for tracking TODO.md files across projects. Built with Tauri, a React frontend and a Rust core (`src-tauri/core/`), which also provides the `pov` CLI.

![Project Overview](screens/projects.png)

## Prerequisites

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/) (v18+)

## Development

Install dependencies:

```sh
npm install
```

Run the app in dev mode (hot-reloading frontend):

```sh
npm run tauri dev
```

Dev builds use `~/.local/share/pov-dev/` for data, keeping it separate from production data.

Build the dev CLI (used by **Settings → Install CLI** in dev mode, which symlinks it):

```sh
cd src-tauri && cargo build -p pov-core --features cli --bin pov-cli
```

## Testing

UI feature tests use Vitest + React Testing Library. They render the real frontend and run
the real Rust commands through a test-only HTTP bridge (`pov-bridge`) on a temp `HOME`, faking
only native calls (file dialog, editor, CLI install):

```sh
npm test
```

Rust core and CLI tests, against real SQLite and files in a temp dir (no mocks):

```sh
cd src-tauri && cargo test -p pov-core --all-features
```

A launch smoke test starts the real debug app on a temp `HOME` and checks the Rust startup and
file watcher (a window appears while it runs):

```sh
npm run smoke
```

## Build

Build the app:

```sh
npm run tauri build
```

The output is a `.dmg` at `src-tauri/target/release/bundle/dmg/`.

`beforeBuildCommand` also runs `scripts/build-cli.sh`, which builds the release `pov-cli` and copies it to `src-tauri/binaries/` (gitignored) for bundling.

## Gatekeeper (first launch)

The app is unsigned. macOS blocks unsigned apps on first open. Fix with:

```sh
xattr -d com.apple.quarantine "/Applications/Projects Overview.app"
```

Or: right-click the `.app` in Finder → Open → click Open in the dialog.

## CLI

The app ships a `pov` CLI. Install it from **Settings → Install CLI** inside the app. It gets copied to `~/.local/bin/pov` — make sure that's on your `PATH`.

```sh
pov add path/to/TODO.md           # track a project
pov add path/to/TODO.md --type learning
pov list                          # list tracked projects
pov remove "project name"         # stop tracking
pov import-time "Maths" TIME.md   # import a time log into a project
pov mcp                           # MCP server for Claude Code (see below)
```

`import-time` reads a markdown table of `| DATE | TIME | TOPIC |` rows, where
TIME is a number of hours (comma or dot separator), and records the entries
against the project's time heatmap. If the project already has time entries it
refuses to run, unless given `--append` (keep them) or `--replace` (drop them).

### Claude Code and other MCP clients

`pov mcp` is an MCP server on stdio: it lets Claude Code add tasks to your projects. Added tasks
land in the project's inbox, for you to keep, mark next or delete in the app. The app doesn't
need to be running.

```sh
claude mcp add --scope user pov -- ~/.local/bin/pov mcp
```

The shell expands `~` to an absolute path, which the client needs: GUI clients don't get your
shell's `PATH`. `--scope user` makes it available in every project.

Other clients take a JSON config; use the absolute path:

```json
{
  "mcpServers": {
    "pov": { "command": "/Users/<you>/.local/bin/pov", "args": ["mcp"] }
  }
}
```

Tools: `list_projects`, `list_sections` (a project's headings), `add_task` (project, text,
optional section and stage). For development, register `src-tauri/target/debug/pov-cli mcp`
instead: a debug build uses the dev data dir.

## Data locations

| Mode | Path |
|------|------|
| Production | `~/.local/share/pov/` |
| Dev | `~/.local/share/pov-dev/` |

Each directory contains a SQLite database (`pov.db`) and a git-tracked folder of hardlinked TODO.md files.
