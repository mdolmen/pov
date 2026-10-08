// Starts the Rust command bridge once for all UI tests, on a temp HOME so it never touches
// real data.

import { type ChildProcess, execFileSync, spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createInterface } from "node:readline";
import type { TestProject } from "vitest/node";

declare module "vitest" {
  export interface ProvidedContext {
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
  // The Rust commands' test-only HTTP bridge (see pov-core's `bridge` feature).
  const srcTauri = resolve("src-tauri");
  execFileSync(
    "cargo",
    ["build", "--quiet", "-p", "pov-core", "--features", "bridge", "--bin", "pov-bridge"],
    { cwd: srcTauri, stdio: "inherit" },
  );

  const home = mkdtempSync(join(tmpdir(), "pov-ui-"));
  const bridge = spawn(join(srcTauri, "target/debug/pov-bridge"), [], {
    env: { ...process.env, ...GIT_IDENTITY, HOME: home },
    stdio: ["ignore", "pipe", "inherit"],
  });
  const bridgePort = await readPort(bridge);

  project.provide("bridgePort", bridgePort);
  project.provide("povHome", home);

  return async () => {
    const exited = new Promise((done) => bridge.once("exit", done));
    bridge.kill();
    await exited;
    rmSync(home, { recursive: true, force: true });
  };
}

/** Reads the `{"port": N}` line the bridge prints when it starts. */
function readPort(child: ChildProcess): Promise<number> {
  return new Promise((done, fail) => {
    child.once("exit", (code) => fail(new Error(`pov-bridge exited with ${code}`)));
    createInterface({ input: child.stdout! }).once("line", (line) => done(JSON.parse(line).port));
  });
}
