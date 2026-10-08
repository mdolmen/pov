#!/usr/bin/env bash
# Launches the real debug app on a temp HOME and checks what UI tests can't reach: the Rust
# startup (data dir, git repo, database) and the Rust watcher committing an edit.
# A window appears while it runs.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
home=$(mktemp -d)
data="$home/.local/share/pov-dev"
pid=""

cleanup() {
  if [ -n "$pid" ]; then
    pkill -TERM -P "$pid" 2>/dev/null || true
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  rm -rf "$home"
}
trap cleanup EXIT

fail() { echo "FAIL: $1" >&2; exit 1; }

# Polls a command for up to 20 seconds.
wait_for() {
  for _ in $(seq 100); do
    if eval "$1"; then return 0; fi
    kill -0 "$pid" 2>/dev/null || fail "the app exited ($2)"
    sleep 0.2
  done
  fail "$2"
}

(cd "$root/src-tauri" && cargo build --quiet -p pov)

# HOME is a temp dir: give git an identity, and keep uv on the real caches since debug builds
# start the backend with `uv run`.
export GIT_AUTHOR_NAME="pov smoke" GIT_AUTHOR_EMAIL="smoke@pov"
export GIT_COMMITTER_NAME="pov smoke" GIT_COMMITTER_EMAIL="smoke@pov"
export UV_CACHE_DIR="${UV_CACHE_DIR:-$HOME/.cache/uv}"
export UV_PYTHON_INSTALL_DIR="${UV_PYTHON_INSTALL_DIR:-$HOME/.local/share/uv/python}"

HOME="$home" "$root/src-tauri/target/debug/pov" >/dev/null 2>&1 &
pid=$!

wait_for '[ -d "$data/.git" ] && [ -f "$data/pov.db" ] && [ -d "$data/projects" ]' \
  "data dir, git repo and database created"
echo "ok: data dir, git repo and database created"

# The watcher starts right after init; keep editing until a commit shows up.
wait_for 'echo "- [ ] edit $RANDOM" > "$data/projects/smoke.md";
  git -C "$data" log --format=%s | grep -qx "activity: smoke.md"' \
  "watcher commits an edited file"
echo "ok: watcher commits an edited file"

kill -0 "$pid" 2>/dev/null || fail "the app is still running"
echo "ok: the app is still running"
