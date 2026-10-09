import { useEffect, useState } from "react";
import { ActivityHeatmap, SHADES } from "@/components/ActivityHeatmap";
import { quantileThresholds, shadeFor } from "@/components/Heatmap";
import { listActivity } from "@/lib/backend";
import type { Tab } from "@/types";

const STRIP_DAYS = 30;

function readExpanded(tab: Tab): boolean {
  try {
    return localStorage.getItem(`pov.heatmap.${tab}.visible`) === "1";
  } catch {
    return false;
  }
}

function writeExpanded(tab: Tab, expanded: boolean) {
  try {
    localStorage.setItem(`pov.heatmap.${tab}.visible`, expanded ? "1" : "0");
  } catch {
    // localStorage might not exist (e.g. private mode); silently ignore.
  }
}

function dateKey(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** The last 30 days of the tab, one cell per day, oldest first. */
function Strip({ type, refreshKey }: { type: "project" | "learning"; refreshKey: number }) {
  const [data, setData] = useState<Record<string, number>>({});

  useEffect(() => {
    let cancelled = false;
    listActivity(type, STRIP_DAYS)
      .then((rows) => {
        if (!cancelled) setData(Object.fromEntries(rows.map((r) => [r.date, r.count])));
      })
      .catch(() => {
        // Keep the current strip; the next refresh will retry.
      });
    return () => {
      cancelled = true;
    };
  }, [type, refreshKey]);

  const thresholds = quantileThresholds(Object.values(data));
  const days = Array.from({ length: STRIP_DAYS }, (_, i) => {
    const d = new Date();
    d.setDate(d.getDate() - (STRIP_DAYS - 1 - i));
    return dateKey(d);
  });

  return (
    <span className="flex flex-1 gap-0.5">
      {days.map((day) => (
        <i
          key={day}
          className="flex-1 h-3.5 rounded-[2px]"
          style={{ background: shadeFor(data[day] ?? 0, thresholds, SHADES) }}
          title={`${data[day] ?? 0} on ${day}`}
        />
      ))}
    </span>
  );
}

interface Props {
  tab: Tab;
  refreshKey: number;
}

/** The tab's last 30 days; click to show the full heatmap. Open or closed is kept per tab. */
export function ActivityFooter({ tab, refreshKey }: Props) {
  const [expanded, setExpanded] = useState(() => readExpanded(tab));
  const type = tab === "projects" ? "project" : "learning";

  useEffect(() => {
    setExpanded(readExpanded(tab));
  }, [tab]);

  function toggle() {
    setExpanded(!expanded);
    writeExpanded(tab, !expanded);
  }

  return (
    <div className="shrink-0 border-t border-line px-3 pt-2.5 pb-3 text-[12px] text-ink-muted">
      {expanded ? (
        <>
          <div className="flex justify-between items-center mb-1 px-2">
            <span>Activity</span>
            <button onClick={toggle} className="hover:text-ink-2 transition-colors cursor-pointer">
              Hide
            </button>
          </div>
          <ActivityHeatmap key={tab} type={type} months={6} fit refreshKey={refreshKey} />
        </>
      ) : (
        <button
          onClick={toggle}
          aria-label="Show activity"
          className="w-full flex items-center gap-3 px-2 cursor-pointer hover:text-ink-2"
        >
          <span className="shrink-0">Last 30 days</span>
          <Strip type={type} refreshKey={refreshKey} />
        </button>
      )}
    </div>
  );
}
