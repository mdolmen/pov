// Shared helpers: fake native calls, seed projects through the real backend, read the data dir.

import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, inject } from "vitest";
import App from "@/App";
import type { ListItem, Project } from "@/types";

const baseUrl = () => `http://127.0.0.1:${inject("backendPort")}`;

/** The backend's data dir (`~/.local/share/pov-dev` under the temp HOME). */
export const dataDir = () => join(inject("povHome"), ".local/share/pov-dev");

/** Runs a Rust command through the test bridge, as the app's `api` command would. */
async function bridge<T>(cmd: string, args: unknown = {}): Promise<T> {
  const r = await fetch(`http://127.0.0.1:${inject("bridgePort")}/${cmd}`, {
    method: "POST",
    body: JSON.stringify(args),
  });
  if (!r.ok) throw await r.text();
  return r.json();
}

async function api(path: string, init?: RequestInit): Promise<Response> {
  const r = await fetch(`${baseUrl()}${path}`, init);
  if (!r.ok) throw new Error(`${init?.method ?? "GET"} ${path}: ${r.status} ${await r.text()}`);
  return r;
}

/** Deletes every project, so each test starts from an empty list. */
export async function resetData() {
  const projects = await bridge<Project[]>("list_projects");
  for (const p of projects) await bridge("remove_project", { project_id: p.id });
}

/** Writes a TODO.md in its own folder (outside any git repo) and returns its path. */
export function writeTodo(content: string, folder = "Project"): string {
  const dir = join(mkdtempSync(join(inject("povHome"), "files-")), folder);
  mkdirSync(dir);
  const path = join(dir, "TODO.md");
  writeFileSync(path, content);
  return path;
}

interface Seed {
  name: string;
  content?: string;
  file?: string;
  type?: "project" | "learning";
  status?: "open" | "archived";
  sub_status?: "paused" | "done" | "canceled" | null;
}

/** Tracks a project through the real backend. */
export async function addProject(seed: Seed): Promise<Project> {
  const file = seed.file ?? writeTodo(seed.content ?? "- [ ] task\n");
  return bridge<Project>("add_project", { type: "project", status: "open", ...seed, file_path: file });
}

export async function selectTask(projectId: string, hash: string) {
  await bridge("select_task", { project_id: projectId, task_hash: hash });
}

export async function recordTime(projectId: string, minutes: number, date: string, topic?: string) {
  await api(`/projects/${projectId}/time`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ minutes, date, topic }),
  });
}

export async function taskHash(projectId: string, text: string): Promise<string> {
  const items = await bridge<ListItem[]>("list_tasks", { project_id: projectId });
  const match = items
    .flatMap((i) => (i.kind === "task" ? [i, ...i.subtasks] : []))
    .find((t) => t.text === text);
  if (!match) throw new Error(`no task "${text}"`);
  return match.hash;
}

/** Commit subjects of the data repo, newest first. */
export function gitSubjects(): string[] {
  const out = execFileSync("git", ["-C", dataDir(), "log", "--format=%s"], { encoding: "utf8" });
  return out.trim().split("\n");
}

export function today(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

interface Native {
  /** What the file dialog returns. */
  dialogPath?: string | null;
  cliInstalled?: boolean;
  installCli?: () => string;
}

/** Fakes the Tauri side; returns the calls made, as `[command, args]`. */
export function mockNative(native: Native = {}): [string, unknown][] {
  const calls: [string, unknown][] = [];
  mockIPC((cmd, args) => {
    calls.push([cmd, args]);
    switch (cmd) {
      case "get_backend_port":
        return inject("backendPort");
      case "api": {
        const { cmd: command, args: commandArgs } = args as { cmd: string; args: unknown };
        return bridge(command, commandArgs);
      }
      case "plugin:dialog|open":
        return native.dialogPath ?? null;
      case "is_cli_installed":
        return native.cliInstalled ?? false;
      case "install_cli":
        return (native.installCli ?? (() => "Installed pov"))();
      default:
        return null;
    }
  });
  return calls;
}

/** Renders the whole app with fake native calls. */
export function renderApp(native: Native = {}) {
  const calls = mockNative(native);
  const user = userEvent.setup();
  render(<App />);
  return { user, calls };
}

/** Opens a project from the list. */
export async function openProject(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(await screen.findByRole("button", { name: new RegExp(`^${name}`) }));
  await screen.findByRole("button", { name: "Back" });
  await waitFor(() => expect(screen.queryByText("Loading…")).not.toBeInTheDocument());
}

/** The section of the task list under a "Done" label, if any. */
export function doneSection() {
  const label = screen.queryByText("Done", { selector: "p" });
  return label ? within(label.nextElementSibling as HTMLElement) : null;
}

/** The checkbox of a task or subtask, found by its text. */
export function checkbox(text: string): HTMLElement {
  return screen.getByText(text).previousElementSibling as HTMLElement;
}

/** The hover "+" / "×" button of a pending task, found by its text. */
export function nextButton(text: string): HTMLElement {
  return screen.getByText(text).nextElementSibling as HTMLElement;
}

/** Text positions in the page, to check display order. */
export function inOrder(...texts: string[]): boolean {
  const body = document.body.textContent ?? "";
  const positions = texts.map((t) => body.indexOf(t));
  return positions.every((p, i) => p >= 0 && (i === 0 || p > positions[i - 1]));
}

export async function getProject(id: string): Promise<Project> {
  const projects = await bridge<Project[]>("list_projects");
  return projects.find((p) => p.id === id) as Project;
}

/** Picks an option in a Select, found by the index of its trigger in the dialog. */
export async function choose(
  user: ReturnType<typeof userEvent.setup>,
  trigger: number,
  option: string,
) {
  await user.click(screen.getAllByRole("combobox")[trigger]);
  await user.click(await screen.findByRole("option", { name: option }));
}

/** Turns on the "Archived" filter of the Projects tab. */
export async function showArchived(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: "Menu" }));
  await user.click(screen.getAllByRole("button", { name: "Archived" })[0]);
  await user.click(screen.getByRole("button", { name: "Menu" }));
}

export async function toggleTask(projectId: string, hash: string) {
  await bridge("toggle_task", { project_id: projectId, task_hash: hash });
}

export function daysAgo(n: number): string {
  const d = new Date();
  d.setDate(d.getDate() - n);
  const pad = (v: number) => String(v).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
