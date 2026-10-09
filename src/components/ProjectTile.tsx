import { useEffect, useState } from "react";
import { listProjectActivity } from "@/lib/backend";
import type { Project } from "@/types";

const WEEKS = 8;

function dateKey(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Ramp step of a week's activity count. */
function weekLevel(count: number): number {
  if (count <= 0) return 0;
  if (count <= 4) return 1;
  if (count <= 9) return 2;
  if (count <= 15) return 3;
  return 4;
}

/** Activity of the last 8 weeks, oldest first, one cell per week ending today. */
function WeekRow({ project, refreshKey }: { project: Project; refreshKey: number }) {
  const [weeks, setWeeks] = useState<number[]>(() => Array(WEEKS).fill(0));

  useEffect(() => {
    let cancelled = false;
    listProjectActivity(project.id, project.type, WEEKS * 7)
      .then((days) => {
        if (cancelled) return;
        const counts = new Map(days.map((d) => [d.date, d.count]));
        const sums: number[] = [];
        for (let w = WEEKS - 1; w >= 0; w--) {
          let sum = 0;
          for (let i = 0; i < 7; i++) {
            const d = new Date();
            d.setDate(d.getDate() - w * 7 - i);
            sum += counts.get(dateKey(d)) ?? 0;
          }
          sums.push(sum);
        }
        setWeeks(sums);
      })
      .catch(() => {
        // Keep the current row; the next refresh will retry.
      });
    return () => {
      cancelled = true;
    };
  }, [project.id, project.type, refreshKey]);

  return (
    <span className="flex gap-0.5 shrink-0" title="Last 8 weeks" data-weeks={weeks.join(",")}>
      {weeks.map((count, i) => (
        <i
          key={i}
          className="w-1.5 h-1.5 rounded-[1.5px]"
          style={{ background: `var(--pov-heat-${weekLevel(count)})` }}
        />
      ))}
    </span>
  );
}

interface Props {
  project: Project;
  selected: boolean;
  activityRefreshKey: number;
  onClick: () => void;
}

export function ProjectTile({ project, selected, activityRefreshKey, onClick }: Props) {
  const quiet = project.activity === "older" || project.activity === "none";

  return (
    <button
      onClick={onClick}
      className={
        "min-w-0 flex flex-col gap-1 rounded-[10px] px-3 py-2.5 text-left cursor-pointer transition-shadow " +
        (selected ? "ring-2 ring-next bg-surface " : "ring-1 ring-line hover:ring-check ") +
        (quiet && !selected ? "bg-transparent" : "bg-surface")
      }
    >
      <span className="flex items-center justify-between gap-2">
        <span
          className={
            "text-[13.5px] truncate " + (quiet ? "font-medium text-ink-2" : "font-semibold text-ink")
          }
        >
          {project.name}
        </span>
        <WeekRow project={project} refreshKey={activityRefreshKey} />
      </span>
      <span className="text-[12px] text-ink-muted tabular-nums">
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
    </button>
  );
}
