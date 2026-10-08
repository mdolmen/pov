// Starts the real backend once for all UI tests, on a temp HOME so it never touches real data.

import { type ChildProcess, execFileSync, spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createInterface } from "node:readline";
import type { TestProject } from "vitest/node";

declare module "vitest" {
  export interface ProvidedContext {
    backendPort: number;
    bridgePort: number;
    povHome: string;
  }
}

// HOME is a temp dir, so git can't see the user's ~/.gitconfig identity.
export const GIT_IDENTITY = {
  GIT_AUTHOR_NAME: "pov tests",
  GIT_AUTHOR_EMAIL: "tests@pov",
  GIT_COMMITTER_NAME: "pov tests",
  GIT_COMMITTER_EMAIL: "tests@pov",
};

export default async function setup(project: TestProject) {
  const backendDir = resolve("backend");
  const python = join(backendDir, ".venv/bin/python");
  if (!existsSync(python)) {
    throw new Error("backend venv missing: run `uv sync --directory backend` first");
  }

  // The Rust commands' test-only HTTP bridge (see pov-core's `bridge` feature).
  const srcTauri = resolve("src-tauri");
  execFileSync(
    "cargo",
    ["build", "--quiet", "-p", "pov-core", "--features", "bridge", "--bin", "pov-bridge"],
    { cwd: srcTauri, stdio: "inherit" },
  );

  const home = mkdtempSync(join(tmpdir(), "pov-ui-"));
  const env = { ...process.env, ...GIT_IDENTITY, HOME: home, POV_ENV: "dev" };
  // One after the other: both initialize the data dir and its git repo.
  const backend = spawn(python, ["main.py"], { cwd: backendDir, env, stdio: ["ignore", "pipe", "inherit"] });
  const backendPort = await readPort(backend, "backend");
  for (let i = 0; i < 100; i++) {
    const ok = await fetch(`http://127.0.0.1:${backendPort}/health`).then((r) => r.ok, () => false);
    if (ok) break;
    await new Promise((r) => setTimeout(r, 100));
  }
  const bridge = spawn(join(srcTauri, "target/debug/pov-bridge"), [], {
    env,
    stdio: ["ignore", "pipe", "inherit"],
  });
  const bridgePort = await readPort(bridge, "bridge");

  project.provide("backendPort", backendPort);
  project.provide("bridgePort", bridgePort);
  project.provide("povHome", home);

  return async () => {
    await Promise.all([stop(backend), stop(bridge)]);
    rmSync(home, { recursive: true, force: true });
  };
}

/** Reads the `{"port": N}` line a server prints when it starts. */
function readPort(child: ChildProcess, name: string): Promise<number> {
  return new Promise((done, fail) => {
    child.once("exit", (code) => fail(new Error(`${name} exited with ${code}`)));
    createInterface({ input: child.stdout! }).once("line", (line) => done(JSON.parse(line).port));
  });
}

function stop(child: ChildProcess): Promise<unknown> {
  const exited = new Promise((done) => child.once("exit", done));
  child.kill();
  return exited;
}
