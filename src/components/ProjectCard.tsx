import type { Project } from "@/types";

const OPEN_STRIP: Record<string, string> = {
  this_week: "#16a34a",
  this_month: "#86efac",
  older: "#e2e1dd",
  none: "#e2e1dd",
};

const ARCHIVED_STRIP: Record<string, string> = {
  done: "#16a34a",
  canceled: "#dc2626",
  paused: "#e2e1dd",
};

function activityStrip(project: Project): string {
  if (project.status === "archived") {
    return ARCHIVED_STRIP[project.sub_status ?? "paused"] ?? "#e2e1dd";
  }
  return OPEN_STRIP[project.activity] ?? "#e2e1dd";
}

interface Props {
  project: Project;
  onClick: () => void;
}

export function ProjectCard({ project, onClick }: Props) {
  const strip = activityStrip(project);

  return (
    <button
      onClick={onClick}
      className="group w-full flex items-stretch bg-white rounded-lg overflow-hidden text-left transition-shadow duration-150 hover:shadow-sm"
      style={{ border: "1px solid rgba(0,0,0,0.07)" }}
    >
      <div className="w-[3px] shrink-0" style={{ backgroundColor: strip }} />

      <div className="flex-1 px-3 py-2.5 min-w-0">
        <div className="flex items-baseline justify-between gap-2">
          <span
            className="text-sm font-medium text-stone-800 truncate"
            style={{ fontFeatureSettings: '"ss01"' }}
          >
            {project.name}
          </span>
          {project.selected_count > 0 && (
            <span
              className="shrink-0 text-xs font-bold tabular-nums text-stone-800"
              style={{ fontFamily: "var(--font-mono, monospace)" }}
            >
              {project.selected_count}
            </span>
          )}
        </div>
        <p className="text-[10px] text-stone-400 mt-0.5 tabular-nums">
          {project.task_count} {project.task_count === 1 ? "task" : "tasks"}
          {project.status === "archived" && project.sub_status && (
            <span className="ml-2 text-stone-300">· {project.sub_status}</span>
          )}
        </p>
      </div>
    </button>
  );
}
