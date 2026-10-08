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
