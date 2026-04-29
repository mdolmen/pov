export type ActivityLevel = "this_week" | "this_month" | "older" | "none";
export type Status = "open" | "archived";
export type SubStatus = "paused" | "done" | "canceled" | null;
export type Tab = "projects" | "learning";

export interface SectionFilters {
  open: boolean;
  archived: boolean;
}

export interface Filters {
  projects: SectionFilters;
  learning: SectionFilters;
}

export interface Subtask {
  hash: string;
  text: string;
  checked: boolean;
  line_number: number;
}

export interface Task {
  hash: string;
  text: string;
  checked: boolean;
  line_number: number;
  subtasks: Subtask[];
  is_done: boolean;
  is_selected: boolean;
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
  activity: ActivityLevel;
}
