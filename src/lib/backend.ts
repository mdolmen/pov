import { invoke } from "@tauri-apps/api/core";
import type {
  ActivityDay,
  ListItem,
  NextUpEntry,
  Project,
  Status,
  SubStatus,
  Task,
  TimeDay,
  TimeEntry,
} from "@/types";

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

export const keepTask = (projectId: string, taskHash: string) =>
  call<null>("keep_task", { project_id: projectId, task_hash: taskHash });

export const cleanupSelected = () => call<null>("cleanup_selected");

export const listNextUp = () => call<NextUpEntry[]>("next_up");

export interface NewProject {
  name: string;
  file_path: string;
  type: "project" | "learning";
  status: Status;
  sub_status: SubStatus;
}

/** Fields to change; `null` clears `sub_status`, `""` clears `paused_until`. */
export interface ProjectChanges {
  name?: string;
  status?: Status;
  sub_status?: SubStatus;
  paused_until?: string;
}

export const listProjects = () => call<Project[]>("list_projects");

export const addProject = (project: NewProject) => call<Project>("add_project", { ...project });

export const updateProject = (projectId: string, changes: ProjectChanges) =>
  call<Project>("update_project", { project_id: projectId, ...changes });

export const removeProject = (projectId: string) =>
  call<null>("remove_project", { project_id: projectId });

export const listTime = (projectId: string, days: number) =>
  call<TimeDay[]>("list_time", { project_id: projectId, days });

export const addTime = (
  projectId: string,
  entry: { minutes: number; date: string; topic: string | null },
) => call<TimeEntry>("add_time", { project_id: projectId, ...entry });

export const listTopics = (projectId: string) =>
  call<string[]>("list_topics", { project_id: projectId });

export const listActivity = (type: "project" | "learning", days: number) =>
  call<ActivityDay[]>("activity", { type, days });

export const listProjectActivity = (projectId: string, type: string, days: number) =>
  call<ActivityDay[]>("activity", { type, days, project_id: projectId });
