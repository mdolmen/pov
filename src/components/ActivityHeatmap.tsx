import { useEffect, useMemo, useState } from "react";
import { Heatmap, windowDays } from "@/components/Heatmap";
import { apiFetch } from "@/lib/backend";

interface Day {
  date: string; // YYYY-MM-DD
  count: number;
}

interface Props {
  type: "project" | "learning";
  /** Number of full calendar months to display, ending with the current month. */
  months?: number;
  /** Increment to trigger a re-fetch. */
  refreshKey?: number;
}

// Stone-100 base, then green ramp inspired by GitHub's contribution graph but
// muted to fit the warm app palette.
const SHADES = ["#ebe9e2", "#c6e3c2", "#86c98a", "#4ea862", "#2a7a3f"];

export function ActivityHeatmap({ type, months = 4, refreshKey = 0 }: Props) {
  const [data, setData] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(true);

  const queryDays = useMemo(() => windowDays(months), [months]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    (async () => {
      const r = await apiFetch(`/activity?type=${type}&days=${queryDays}`);
      if (!r.ok) {
        if (!cancelled) setLoading(false);
        return;
      }
      const rows: Day[] = await r.json();
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
      tooltip={(count, date) =>
        `${count} ${count === 1 ? "activity" : "activities"} on ${date}`
      }
    />
  );
}
