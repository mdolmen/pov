import { useEffect, useState } from "react";
import { listProjectActivity } from "@/lib/backend";
import type { Project } from "@/types";

/** Days in a tile's bars: two bars of four. */
export const DAYS = 8;

const NO_DAYS: number[] = Array(DAYS).fill(0);

function dateKey(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Ramp step of a day's activity count: the shade of its cell. */
export function dayLevel(count: number): number {
  if (count <= 0) return 0;
  if (count <= 2) return 1;
  if (count <= 5) return 2;
  if (count <= 9) return 3;
  return 4;
}

/** Most active first: by the summed shades of the 8 cells, then by the commits they count. */
export function byActivity(a: number[], b: number[]): number {
  const shades = (days: number[]) => days.reduce((sum, c) => sum + dayLevel(c), 0);
  const total = (days: number[]) => days.reduce((sum, c) => sum + c, 0);
  return shades(b) - shades(a) || total(b) - total(a);
}

/**
 * Activity counts of the last 8 days for each project, oldest first, today last. Refetched
 * when the projects or `refreshKey` change.
 */
export function useProjectDays(projects: Project[], refreshKey: number) {
  const [days, setDays] = useState<Map<string, number[]>>(new Map());
  const ids = projects.map((p) => `${p.id}:${p.type}`).join(",");

  useEffect(() => {
    let cancelled = false;
    const pairs = ids ? ids.split(",").map((s) => s.split(":")) : [];
    Promise.all(
      pairs.map(async ([id, type]) => {
        try {
          const counts = new Map(
            (await listProjectActivity(id, type, DAYS)).map((d) => [d.date, d.count]),
          );
          const cells: number[] = [];
          for (let ago = DAYS - 1; ago >= 0; ago--) {
            const d = new Date();
            d.setDate(d.getDate() - ago);
            cells.push(counts.get(dateKey(d)) ?? 0);
          }
          return [id, cells] as const;
        } catch {
          return null;
        }
      }),
    ).then((results) => {
      if (cancelled) return;
      setDays((prev) => {
        const next = new Map(prev);
        for (const r of results) if (r) next.set(r[0], r[1]);
        return next;
      });
    });
    return () => {
      cancelled = true;
    };
  }, [ids, refreshKey]);

  return (id: string) => days.get(id) ?? NO_DAYS;
}
