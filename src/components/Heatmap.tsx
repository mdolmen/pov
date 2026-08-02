import { useMemo } from "react";

interface Props {
  /** Value per day, keyed by YYYY-MM-DD. Missing days render as empty. */
  data: Record<string, number>;
  /** Number of full calendar months to display, ending with the current month. */
  months: number;
  /** Five colors: empty cell, then four ramp steps. */
  shades: readonly string[];
  /** Tooltip text for a day; receives the raw value and the YYYY-MM-DD key. */
  tooltip: (value: number, date: string) => string;
  loading?: boolean;
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

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
  return [at(0.25), at(0.5), at(0.75), at(0.9)];
}

function shadeFor(value: number, thresholds: number[], shades: readonly string[]): string {
  if (value <= 0) return shades[0];
  if (value <= thresholds[0]) return shades[1];
  if (value <= thresholds[1]) return shades[2];
  if (value <= thresholds[2]) return shades[3];
  return shades[4];
}

export function Heatmap({ data, months, shades, tooltip, loading = false }: Props) {
  // Calendar-aligned window: first day of (current month - (months - 1)) → today.
  const { windowStart, windowEnd } = useMemo(() => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const start = new Date(today.getFullYear(), today.getMonth() - (months - 1), 1);
    return { windowStart: start, windowEnd: today };
  }, [months]);

  const { weeks, monthLabels, thresholds } = useMemo(() => {
    // Build a [weeks][7 days] grid. Columns are weeks (Mon-Sun); rows are
    // weekdays (Mon=0..Sun=6). The window starts on the first of a month;
    // we roll back to the previous Monday so each column is a clean week.
    const start = new Date(windowStart);
    const startWeekday = (start.getDay() + 6) % 7; // Mon=0..Sun=6
    start.setDate(start.getDate() - startWeekday);

    type Cell = { date: Date; key: string; value: number; inWindow: boolean };
    const weeks: Cell[][] = [];
    const cursor = new Date(start);
    let currentWeek: Cell[] = [];
    while (cursor <= windowEnd) {
      const key = dateKey(cursor);
      const inWindow = cursor >= windowStart && cursor <= windowEnd;
      currentWeek.push({ date: new Date(cursor), key, value: data[key] ?? 0, inWindow });
      if (currentWeek.length === 7) {
        weeks.push(currentWeek);
        currentWeek = [];
      }
      cursor.setDate(cursor.getDate() + 1);
    }
    if (currentWeek.length) {
      while (currentWeek.length < 7) {
        currentWeek.push({ date: new Date(NaN), key: "", value: 0, inWindow: false });
      }
      weeks.push(currentWeek);
    }

    // Month labels: place above the column whose first in-window day is the
    // first of that month, so labels align to the start of each month rather
    // than to whatever week happens to span it.
    const monthLabels: { col: number; text: string }[] = [];
    weeks.forEach((week, col) => {
      const firstOfMonth = week.find((d) => d.inWindow && d.date.getDate() === 1);
      if (firstOfMonth) {
        monthLabels.push({ col, text: MONTHS[firstOfMonth.date.getMonth()] });
      }
    });
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
            <svg className="animate-spin" width="10" height="10" viewBox="0 0 24 24" fill="none">
              <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" opacity="0.25" />
              <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
            </svg>
            <span>updating</span>
          </>
        )}
      </div>
      <div className="overflow-x-auto">
        <svg width={totalWidth} height={totalHeight} role="img" aria-label="Heatmap">
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
              return (
                <rect
                  key={day.key}
                  x={col * (cell + gap)}
                  y={14 + row * (cell + gap)}
                  width={cell}
                  height={cell}
                  rx={2}
                  ry={2}
                  fill={shadeFor(day.value, thresholds, shades)}
                >
                  <title>{tooltip(day.value, day.key)}</title>
                </rect>
              );
            })
          )}
        </svg>
      </div>
    </div>
  );
}

/** Days covered by a `months`-long calendar-aligned window ending today. */
export function windowDays(months: number): number {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const start = new Date(today.getFullYear(), today.getMonth() - (months - 1), 1);
  return Math.floor((today.getTime() - start.getTime()) / 86_400_000) + 1;
}
