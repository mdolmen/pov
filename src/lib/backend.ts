import { invoke } from "@tauri-apps/api/core";
import type {
  ActivityDay,
  DeletedTask,
  ListItem,
  NextUpEntry,
  Project,
  Status,
  SubStatus,
  Task,
  TimeDay,
  TimeEntry,
  ToolEntry,
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

export const inboxTask = (projectId: string, taskHash: string) =>
  call<null>("inbox_task", { project_id: projectId, task_hash: taskHash });

export const deleteTask = (projectId: string, taskHash: string) =>
  call<DeletedTask>("delete_task", { project_id: projectId, task_hash: taskHash });

export const restoreTask = (projectId: string, deleted: DeletedTask) =>
  call<null>("restore_task", { project_id: projectId, ...deleted });

/**
 * Sends a task to agent `tool`, starting it when it doesn't answer, and delegates it once the
 * agent took it (the app's own command).
 */
export const delegateTask = (projectId: string, taskHash: string, tool: string, note: string | null) =>
  invoke<null>("delegate", { projectId, taskHash, tool, note });

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

export const listTools = () => call<ToolEntry[]>("list_tools");

/** Defines or replaces a tool; returns all the tools. */
export const saveTool = (tool: ToolEntry) => call<ToolEntry[]>("save_tool", { ...tool });

/** Removes a tool and unlinks it from every project; returns the tools left. */
export const removeTool = (name: string) => call<ToolEntry[]>("remove_tool", { name });

export const setProjectTools = (projectId: string, tools: string[]) =>
  call<Project>("set_project_tools", { project_id: projectId, tools });

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
