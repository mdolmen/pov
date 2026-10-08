// Starts the real backend once for all UI tests, on a temp HOME so it never touches real data.

import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createInterface } from "node:readline";
import type { TestProject } from "vitest/node";

declare module "vitest" {
  export interface ProvidedContext {
    backendPort: number;
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

  const home = mkdtempSync(join(tmpdir(), "pov-ui-"));
  const backend = spawn(python, ["main.py"], {
    cwd: backendDir,
    env: { ...process.env, ...GIT_IDENTITY, HOME: home, POV_ENV: "dev" },
    stdio: ["ignore", "pipe", "inherit"],
  });

  const port = await new Promise<number>((done, fail) => {
    backend.once("exit", (code) => fail(new Error(`backend exited with ${code}`)));
    createInterface({ input: backend.stdout }).once("line", (line) => {
      done(JSON.parse(line).port);
    });
  });
  for (let i = 0; i < 100; i++) {
    const ok = await fetch(`http://127.0.0.1:${port}/health`).then((r) => r.ok, () => false);
    if (ok) break;
    await new Promise((r) => setTimeout(r, 100));
  }

  project.provide("backendPort", port);
  project.provide("povHome", home);

  return async () => {
    const exited = new Promise((done) => backend.once("exit", done));
    backend.kill();
    await exited;
    rmSync(home, { recursive: true, force: true });
  };
}
