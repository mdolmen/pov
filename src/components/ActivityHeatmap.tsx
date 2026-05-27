import { useEffect, useMemo, useState } from "react";
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

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
// Stone-100 base, then green ramp inspired by GitHub's contribution graph but
// muted to fit the warm app palette.
const SHADES = ["#ebe9e2", "#c6e3c2", "#86c98a", "#4ea862", "#2a7a3f"];

function dateKey(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function quantileThresholds(values: number[]): number[] {
  // Returns 4 thresholds defining 5 buckets (including the 0 bucket).
  // Computed from the non-zero values so a few quiet days don't flatten the scale.
  const nonZero = values.filter((v) => v > 0).sort((a, b) => a - b);
  if (nonZero.length === 0) return [1, 2, 3, 4];
  const at = (q: number) => nonZero[Math.min(nonZero.length - 1, Math.floor(q * nonZero.length))];
  return [at(0.25), at(0.5), at(0.75), at(0.9)].map((v, i, arr) => Math.max(v, arr[Math.max(0, i - 1)] + (i === 0 ? 0 : 0)));
}

function shadeFor(count: number, thresholds: number[]): string {
  if (count <= 0) return SHADES[0];
  if (count <= thresholds[0]) return SHADES[1];
  if (count <= thresholds[1]) return SHADES[2];
  if (count <= thresholds[2]) return SHADES[3];
  return SHADES[4];
}

export function ActivityHeatmap({ type, months = 4, refreshKey = 0 }: Props) {
  const [data, setData] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(true);

  // Calendar-aligned window: first day of (current month - (months - 1)) → today.
  // Computed once per render-input change.
  const { windowStart, windowEnd, queryDays } = useMemo(() => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const start = new Date(today.getFullYear(), today.getMonth() - (months - 1), 1);
    const days = Math.floor((today.getTime() - start.getTime()) / 86_400_000) + 1;
    return { windowStart: start, windowEnd: today, queryDays: days };
  }, [months]);

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

  const { weeks, monthLabels, thresholds } = useMemo(() => {
    // Build a [weeks][7 days] grid. Columns are weeks (Mon-Sun); rows are
    // weekdays (Mon=0..Sun=6). The window starts on the first of a month;
    // we roll back to the previous Monday so each column is a clean week.
    const start = new Date(windowStart);
    const startWeekday = (start.getDay() + 6) % 7; // Mon=0..Sun=6
    start.setDate(start.getDate() - startWeekday);

    type Cell = { date: Date; key: string; count: number; inWindow: boolean };
    const weeks: Cell[][] = [];
    const cursor = new Date(start);
    let currentWeek: Cell[] = [];
    while (cursor <= windowEnd) {
      const key = dateKey(cursor);
      const inWindow = cursor >= windowStart && cursor <= windowEnd;
      currentWeek.push({ date: new Date(cursor), key, count: data[key] ?? 0, inWindow });
      if (currentWeek.length === 7) {
        weeks.push(currentWeek);
        currentWeek = [];
      }
      cursor.setDate(cursor.getDate() + 1);
    }
    if (currentWeek.length) {
      while (currentWeek.length < 7) {
        currentWeek.push({ date: new Date(NaN), key: "", count: 0, inWindow: false });
      }
      weeks.push(currentWeek);
    }

    // Month labels: place above the column whose first in-window day is the
    // first of that month. This keeps Feb/Mar/Apr/May visually aligned to
    // the start of each month rather than to whatever week happens to span it.
    const monthLabels: { col: number; text: string }[] = [];
    weeks.forEach((week, col) => {
      const firstOfMonth = week.find((d) => d.inWindow && d.date.getDate() === 1);
      if (firstOfMonth) {
        monthLabels.push({ col, text: MONTHS[firstOfMonth.date.getMonth()] });
      }
    });
    // If the window opens mid-month (it shouldn't, since we align to day 1,
    // but be defensive) and no label landed on col 0, prepend one.
    if (monthLabels.length === 0 || monthLabels[0].col > 0) {
      const first = weeks[0]?.find((d) => d.inWindow);
      if (first) {
        monthLabels.unshift({ col: 0, text: MONTHS[first.date.getMonth()] });
      }
    }

    const thresholds = quantileThresholds(Object.values(data));
    return { weeks, monthLabels, thresholds };
  }, [data, windowStart, windowEnd]);

  const cell = 11; // px
  const gap = 3; // px
  // Right padding so the last month label (anchored at the start of its
  // column) isn't clipped by the SVG edge.
  const rightPad = 8;
  const totalWidth = weeks.length * (cell + gap) + rightPad;
  const totalHeight = 7 * (cell + gap) + 14; // 14px reserved for month labels

  return (
    <div>
      <div className="flex justify-end items-center gap-1.5 text-[10px] text-stone-400 h-3 mb-1">
        {loading && (
          <>
            <svg
              className="animate-spin"
              width="10"
              height="10"
              viewBox="0 0 24 24"
              fill="none"
            >
              <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" opacity="0.25" />
              <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
            </svg>
            <span>updating</span>
          </>
        )}
      </div>
      <div className="overflow-x-auto">
      <svg width={totalWidth} height={totalHeight} role="img" aria-label="Activity heatmap">
        {monthLabels.map((m) => (
          <text
            key={`${m.col}-${m.text}`}
            x={m.col * (cell + gap)}
            y={10}
            fontSize="9"
            fill="#a8a29e"
            style={{ fontFeatureSettings: '"ss01"' }}
          >
            {m.text}
          </text>
        ))}
        {weeks.map((week, col) =>
          week.map((day, row) => {
            if (isNaN(day.date.getTime()) || !day.inWindow) return null;
            const fill = shadeFor(day.count, thresholds);
            const x = col * (cell + gap);
            const y = 14 + row * (cell + gap);
            const tooltip = `${day.count} ${day.count === 1 ? "activity" : "activities"} on ${day.key}`;
            return (
              <rect
                key={day.key}
                x={x}
                y={y}
                width={cell}
                height={cell}
                rx={2}
                ry={2}
                fill={fill}
              >
                <title>{tooltip}</title>
              </rect>
            );
          })
        )}
      </svg>
      </div>
    </div>
  );
}
