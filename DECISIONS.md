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
