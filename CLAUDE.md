# CLAUDE.md — pov

macOS desktop app tracking TODO.md files across projects: Tauri + React frontend, Rust core
(`pov-core`, `src-tauri/core/`). The `pov` CLI is the `pov-cli` binary of `pov-core` (`cli`
feature). Read `TODO.md` for the build plan and `DECISIONS.md` for past decisions before making
changes.

These are behavioral guidelines (derived from Karpathy's notes on LLM coding pitfalls). They
bias toward caution over speed; use judgment on trivial tasks.

## 1. Think before coding

- State assumptions explicitly. If a requirement is ambiguous, ask — don't pick silently.
- If multiple interpretations exist, surface them. If a simpler approach exists, say so and
  push back when warranted.
- If something is unclear, stop and name what's confusing.

## 2. Simplicity first

- Minimum code that solves the problem. Nothing speculative.
- No features, abstractions, "flexibility", or error handling beyond what was asked.
- If you write 200 lines and it could be 50, rewrite it. Ask: "would a senior engineer call
  this overcomplicated?"

## 3. Surgical changes

- Touch only what the task requires. Don't "improve" adjacent code, comments, or formatting;
  match existing style even if you'd do it differently.
- Clean up orphans **your** changes create (unused imports/vars). Don't delete pre-existing
  dead code — mention it instead.
- Every changed line should trace directly to the request.

## 4. Goal-driven execution

- Turn tasks into verifiable goals: "fix bug" → "write a failing test that reproduces it, then
  make it pass".
- For multi-step work, state a brief plan with a verify step each, then loop until green.

---

## Project-specific guardrails

- **Decision log.** When the user settles a design or product choice, record it in
  `DECISIONS.md`: a dated entry with Decision / Why / Constraints / Plan, brief, required
  information only. `TODO.md` links to it instead of repeating the reasoning.
- **No data migration by accident.** `pov.db` schema and the `~/.local/share/pov/` layout are
  shared by the app and the CLI; schema changes go through the migration in `init_db`.
- **Tests hit real SQLite and real files** in a temp dir. No mocks.

## Rust

- `cargo fmt` and `cargo clippy -- -D warnings` clean on every commit.
- Errors: a `thiserror` enum in the core, `anyhow` only in the CLI binary. Tauri commands
  return `Result<T, String>`, mapped at the boundary. No `unwrap()` / `expect()` outside tests
  and startup invariants.
- Core logic in plain functions over `&Connection` / `&Path`, with no Tauri types, so the app,
  the CLI and tests share it. Tauri commands are thin wrappers.
- `serde` structs mirror the field names in `src/types.ts` (`#[serde(rename_all)]` only when
  needed); one struct per response shape.
- Prefer owned, simple types (`String`, `Vec`, `PathBuf`) over lifetimes and generics until
  profiling says otherwise. No `unsafe`.
- Blocking code (`rusqlite`, `std::process`) runs via `tauri::async_runtime::spawn_blocking`
  from commands. No async runtime in the core.
- Few, boring dependencies. `Cargo.lock` committed; edition and `rust-version` pinned.
- Tests next to the code (`#[cfg(test)] mod tests`), real SQLite in a `tempfile::TempDir`.
- `///` doc comments on public items; naming per the Rust API guidelines.

## Git

- Commit often. Each commit is self-contained: it builds, it makes sense on its own, and it
  does one thing. A feature spanning backend + frontend can still be one commit if the pieces
  only make sense together — but unrelated cleanups go in their own commit.
- Commit messages in English, following
  [Conventional Commits](https://www.conventionalcommits.org/) —
  `<type>(optional scope): description`.
- **Type**: one of `feat`, `fix`, `docs`, `refactor`, `test`, `build`, `ci`, `chore`, `perf`.
  An optional scope names the area (e.g. `feat(cli):`).
- **Subject**: single line, imperative mood, lower-case after the colon, no trailing period.
  Keep it tight.
- **Body** (only when needed): blank line after subject, then a short paragraph explaining the
  *why*. If there are multiple distinct points, use one bullet (`- `) per point instead of
  prose.
- No `Co-Authored-By` trailers unless explicitly requested.
