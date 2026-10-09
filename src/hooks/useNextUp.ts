import { useCallback, useEffect, useState } from "react";
import { listNextUp } from "@/lib/backend";
import type { NextUpEntry } from "@/types";

export function useNextUp() {
  const [entries, setEntries] = useState<NextUpEntry[]>([]);

  const refresh = useCallback(async () => {
    try {
      setEntries(await listNextUp());
    } catch {
      // Keep the current entries; the next poll will retry.
    }
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 3000);
    return () => clearInterval(id);
  }, [refresh]);

  return { entries, refresh };
}
