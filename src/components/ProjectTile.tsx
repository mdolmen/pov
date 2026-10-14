import { dayLevel } from "@/hooks/useProjectDays";
import type { Project } from "@/types";

/**
 * Activity of the last 8 days, one cell per day: two bars of four, oldest at the top of the
 * left bar, today at the bottom of the right one.
 */
function DayBars({ days }: { days: number[] }) {
  return (
    <span
      className="grid grid-rows-4 grid-flow-col gap-0.5 shrink-0"
      title="Last 8 days"
      data-days={days.join(",")}
    >
      {days.map((count, i) => (
        <i
          key={i}
          className="w-1.5 h-1.5 rounded-[1.5px]"
          style={{ background: `var(--pov-heat-${dayLevel(count)})` }}
        />
      ))}
    </span>
  );
}

interface Props {
  project: Project;
  selected: boolean;
  /** Activity counts of the last 8 days, oldest first. */
  days: number[];
  onClick: () => void;
}

export function ProjectTile({ project, selected, days, onClick }: Props) {
  const quiet = project.activity === "older" || project.activity === "none";

  return (
    <button
      onClick={onClick}
      className={
        "relative min-w-0 flex items-center gap-2 rounded-[10px] px-3 py-2.5 text-left cursor-pointer transition-shadow " +
        (selected ? "ring-2 ring-next bg-surface " : "ring-1 ring-line hover:ring-check ") +
        (quiet && !selected ? "bg-transparent" : "bg-surface")
      }
    >
      <span className="flex-1 min-w-0 flex flex-col gap-1">
        <span
          className={
            "text-[13.5px] truncate " + (quiet ? "font-medium text-ink-2" : "font-semibold text-ink")
          }
        >
          {project.name}
        </span>
        <span className="text-[12px] text-ink-muted tabular-nums truncate">
          {project.selected_count > 0 && (
            <>
              <b className="font-semibold text-next">{project.selected_count} next</b> of{" "}
            </>
          )}
          {project.task_count} open
          {project.status === "archived" && project.sub_status && (
            <span className="ml-1">· {project.sub_status}</span>
          )}
        </span>
      </span>
      <DayBars days={days} />
      {project.inbox_count > 0 && (
        <span
          aria-label={`${project.inbox_count} in inbox`}
          title={`${project.inbox_count} in inbox`}
          className="absolute -top-1.5 -right-1.5 min-w-4 h-4 px-1 rounded-full bg-inbox text-white text-[10px] font-semibold leading-4 text-center tabular-nums ring-2 ring-ground"
        >
          {project.inbox_count}
        </span>
      )}
    </button>
  );
}
