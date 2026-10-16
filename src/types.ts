export type ActivityLevel = "this_week" | "this_month" | "older" | "none";
export type Status = "open" | "archived";
export type SubStatus = "paused" | "done" | "canceled" | null;
export type Tab = "projects" | "learning";

export interface HeadingItem {
  kind: "heading";
  text: string;
  level: number;
}

export interface Subtask {
  hash: string;
  text: string;
  checked: boolean;
  line_number: number;
}

export interface Task {
  kind: "task";
  hash: string;
  text: string;
  checked: boolean;
  line_number: number;
  subtasks: Subtask[];
  is_done: boolean;
  is_selected: boolean;
  in_inbox: boolean;
  meta: TaskMeta | null;
  /** The tool the task is delegated to. */
  delegated_to: string | null;
}

/** A link a tool attached to a task: a URL or a file path. */
export interface Link {
  label: string;
  target: string;
}

/** A task's metadata; `id` is the handle tools refer to the task by. */
export interface TaskMeta {
  id: number;
  note: string | null;
  links: Link[];
  has_source: boolean;
}

export type ListItem = HeadingItem | Task;

/** Lines removed by `delete_task`, to put back with `restore_task`. */
export interface DeletedTask {
  line_number: number;
  text: string;
  /** The task's metadata, opaque to the frontend. */
  meta?: unknown;
}

export interface TimeDay {
  date: string; // YYYY-MM-DD
  minutes: number;
}

export interface TimeEntry {
  id: number;
  date: string;
  minutes: number;
  topic: string | null;
}

export interface ActivityDay {
  date: string; // YYYY-MM-DD
  count: number;
}

export interface Project {
  id: string;
  name: string;
  file_path: string;
  status: Status;
  sub_status: SubStatus;
  type: string;
  has_hardlink: boolean;
  task_count: number;
  selected_count: number;
  inbox_count: number;
  activity: ActivityLevel;
  paused_until: string | null;
}

export interface NextUpEntry {
  project_id: string;
  project_name: string;
  project_type: "project" | "learning";
  /** The nearest `##` or deeper heading; `null` for next tasks before any milestone. */
  milestone: string | null;
  /** The first next task, when `milestone` is `null`. */
  task_text: string | null;
  task_hash: string | null;
  next_count: number;
  /** Over the tasks under that heading, or the task's subtasks. */
  done: number;
  total: number;
}
