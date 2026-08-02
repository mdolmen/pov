import { useEffect, useMemo, useState } from "react";
import { Heatmap, windowDays } from "@/components/Heatmap";
import { apiFetch } from "@/lib/backend";
import { formatMinutes } from "@/lib/utils";

interface Day {
  date: string; // YYYY-MM-DD
  minutes: number;
}

interface Props {
  projectId: string;
  /** Number of full calendar months to display, ending with the current month. */
  months?: number;
  /** Increment to trigger a re-fetch. */
  refreshKey?: number;
}

// Same stone base as the activity heatmap, with a blue ramp so time spent
// reads as a distinct measure from activity.
const SHADES = ["#ebe9e2", "#c9dcf0", "#8bb6e0", "#4f8bc9", "#2b5484"];

export function TimeHeatmap({ projectId, months = 10, refreshKey = 0 }: Props) {
  const [data, setData] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(true);

  const queryDays = useMemo(() => windowDays(months), [months]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    (async () => {
      const r = await apiFetch(`/projects/${projectId}/time?days=${queryDays}`);
      if (!r.ok) {
        if (!cancelled) setLoading(false);
        return;
      }
      const rows: Day[] = await r.json();
      if (cancelled) return;
      const map: Record<string, number> = {};
      for (const row of rows) map[row.date] = row.minutes;
      setData(map);
      setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
  }, [projectId, queryDays, refreshKey]);

  return (
    <Heatmap
      data={data}
      months={months}
      shades={SHADES}
      loading={loading}
      tooltip={(minutes, date) =>
        minutes > 0 ? `${formatMinutes(minutes)} on ${date}` : `No time on ${date}`
      }
    />
  );
}
