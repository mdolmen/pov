import { useEffect, useMemo, useState } from "react";
import { Heatmap, windowDays } from "@/components/Heatmap";
import { listActivity } from "@/lib/backend";

interface Props {
  type: "project" | "learning";
  /** Number of full calendar months to display, ending with the current month. */
  months?: number;
  /** Increment to trigger a re-fetch. */
  refreshKey?: number;
  /** Scale to the container's width. */
  fit?: boolean;
}

// The activity ramp in index.css: empty, then four greens.
export const SHADES = [0, 1, 2, 3, 4].map((i) => `var(--pov-heat-${i})`);

export function ActivityHeatmap({ type, months = 4, refreshKey = 0, fit }: Props) {
  const [data, setData] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(true);

  const queryDays = useMemo(() => windowDays(months), [months]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    (async () => {
      let rows;
      try {
        rows = await listActivity(type, queryDays);
      } catch {
        if (!cancelled) setLoading(false);
        return;
      }
      if (cancelled) return;
      const map: Record<string, number> = {};
      for (const row of rows) map[row.date] = row.count;
      setData(map);
      setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
  }, [type, queryDays, refreshKey]);

  return (
    <Heatmap
      data={data}
      months={months}
      shades={SHADES}
      loading={loading}
      fit={fit}
      tooltip={(count, date) =>
        `${count} ${count === 1 ? "activity" : "activities"} on ${date}`
      }
    />
  );
}
