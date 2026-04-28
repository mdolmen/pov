export type ActivityLevel = "this_week" | "this_month" | "older" | "none";
export type Status = "open" | "archived";
export type SubStatus = "paused" | "done" | "canceled" | null;

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
