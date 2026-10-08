import { invoke } from "@tauri-apps/api/core";
import type { ListItem, Task } from "@/types";

let baseUrl: string | null = null;

export async function getBaseUrl(): Promise<string> {
  if (!baseUrl) {
    const port = await invoke<number>("get_backend_port");
    baseUrl = `http://127.0.0.1:${port}`;
  }
  return baseUrl;
}

export async function apiFetch(path: string, init?: RequestInit): Promise<Response> {
  const url = await getBaseUrl();
  return fetch(`${url}${path}`, init);
}

// Commands served by Rust (`pov_core::api`), through the app's single `api` command.
function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  return invoke<T>("api", { cmd, args });
}

export const listTasks = (projectId: string) =>
  call<ListItem[]>("list_tasks", { project_id: projectId });

export const toggleTask = (projectId: string, taskHash: string) =>
  call<Task>("toggle_task", { project_id: projectId, task_hash: taskHash });

export const selectTask = (projectId: string, taskHash: string) =>
  call<null>("select_task", { project_id: projectId, task_hash: taskHash });

export const unselectTask = (projectId: string, taskHash: string) =>
  call<null>("unselect_task", { project_id: projectId, task_hash: taskHash });

export const cleanupSelected = () => call<null>("cleanup_selected");
