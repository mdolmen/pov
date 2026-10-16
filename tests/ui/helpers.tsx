// Shared helpers: fake native calls, seed projects through the real backend, read the data dir.

import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, inject } from "vitest";
import App from "@/App";
import type { ListItem, Project, ToolEntry } from "@/types";

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

/** Deletes every project and tool, so each test starts from an empty list. */
export async function resetData() {
  const projects = await bridge<Project[]>("list_projects");
  for (const p of projects) await bridge("remove_project", { project_id: p.id });
  const tools = await bridge<ToolEntry[]>("list_tools");
  for (const t of tools) await bridge("remove_tool", { name: t.name });
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

/** Puts tasks in the inbox, as an external tool would. */
export async function inboxTasks(projectId: string, ...texts: string[]) {
  for (const text of texts) {
    await bridge("inbox_task", { project_id: projectId, task_hash: await taskHash(projectId, text) });
  }
}

/** Adds a task with metadata, as a tool does through `pov mcp`; returns its hash. */
export async function addTask(projectId: string, text: string, meta: object = {}): Promise<string> {
  return bridge<string>("add_task", { project_id: projectId, text, meta });
}

/** Defines a tool, as Settings does. */
export async function saveTool(name: string) {
  await bridge("save_tool", { name, url: "http://127.0.0.1:9/tasks" });
}

/** Defines `tools` and links a project to them, as Settings and Project settings do. */
export async function linkTools(projectId: string, ...tools: string[]) {
  for (const t of tools) await saveTool(t);
  await bridge("set_project_tools", { project_id: projectId, tools });
}

/** Delegates a task to a tool, without starting it. */
export async function delegate(projectId: string, text: string, tool: string) {
  const hash = await taskHash(projectId, text);
  await bridge("delegate_task", { project_id: projectId, task_hash: hash, tool });
}

/** Sends a delegated task back to the inbox, as its tool does through `pov mcp`. */
export async function finishTask(projectId: string, text: string) {
  const items = await bridge<ListItem[]>("list_tasks", { project_id: projectId });
  const task = items.find((i) => i.kind === "task" && i.text === text);
  if (task?.kind !== "task" || !task.meta) throw new Error(`no task "${text}" with metadata`);
  await bridge("finish_task", { id: task.meta.id });
}

export async function recordTime(projectId: string, minutes: number, date: string, topic?: string) {
  await bridge("add_time", { project_id: projectId, minutes, date, topic });
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
  /** Why the agent didn't take a delegated task; the task stays as it was. */
  delegateError?: string;
}

/** Fakes the Tauri side; returns the calls made, as `[command, args]`. */
export function mockNative(native: Native = {}): [string, unknown][] {
  const calls: [string, unknown][] = [];
  mockIPC((cmd, args) => {
    calls.push([cmd, args]);
    switch (cmd) {
      case "api": {
        const { cmd: command, args: commandArgs } = args as { cmd: string; args: unknown };
        return bridge(command, commandArgs);
      }
      case "delegate": {
        if (native.delegateError) return Promise.reject(native.delegateError);
        const { projectId, taskHash, tool, note } = args as Record<string, unknown>;
        return bridge("delegate_task", { project_id: projectId, task_hash: taskHash, tool, note });
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

/** The right pane: the open project's tasks. */
export function taskPane() {
  return within(screen.getByRole("region", { name: /^Tasks/ }));
}

/** The Next up entries. */
export function nextUp() {
  return within(screen.getByRole("group", { name: "Next up" }));
}

/** The projects of the current tab. */
export function projectTiles() {
  return within(screen.getByRole("group", { name: "Project list" }));
}

/** Matches the accessible name of a project's tile: its name, then its counts. */
export function tileName(name: string): RegExp {
  return new RegExp(`^${name}(\\d+ next of )?\\d+ open`);
}

/** Opens a project from the list. */
export async function openProject(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(await projectTiles().findByRole("button", { name: tileName(name) }));
  await screen.findByRole("button", { name: "Close project" });
  await waitFor(() => expect(screen.queryByText("Loading…")).not.toBeInTheDocument());
}

/** Whether a task or subtask is struck through as done. */
export function struck(text: string): boolean {
  return taskPane().getByText(text).style.textDecoration === "line-through";
}

/** The checkbox of a task or subtask, found by its text. */
export function checkbox(text: string): HTMLElement {
  return taskPane().getByText(text).previousElementSibling as HTMLElement;
}

/** The row of a task, found by its text. */
export function taskRow(text: string): HTMLElement {
  return taskPane().getByText(text).closest("[data-task]") as HTMLElement;
}

/** The "Mark next" / "Next" button of an open task, found by its text. */
export function nextButton(text: string): HTMLElement {
  return within(taskRow(text)).getByRole("button", { name: /^(Mark next|Next)$/ });
}

/** A triage button ("Keep", "Delete task") of an inbox task, found by its text. */
export function triageButton(text: string, name: string): HTMLElement {
  return within(taskRow(text)).getByRole("button", { name });
}

/** Text positions in the task pane, to check display order. */
export function inOrder(...texts: string[]): boolean {
  const body = screen.getByRole("region", { name: /^Tasks/ }).textContent ?? "";
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

export async function toggleTask(projectId: string, hash: string) {
  await bridge("toggle_task", { project_id: projectId, task_hash: hash });
}

export function daysAgo(n: number): string {
  const d = new Date();
  d.setDate(d.getDate() - n);
  const pad = (v: number) => String(v).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
