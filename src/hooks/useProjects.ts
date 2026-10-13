import { useCallback, useEffect, useState } from "react";
import { listProjects } from "@/lib/backend";
import type { Project } from "@/types";
import { useOnFocus } from "./useOnFocus";

export function useProjects() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (showSpinner = true) => {
    if (showSpinner) setLoading(true);
    try {
      setProjects(await listProjects());
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(() => refresh(false), 3000);
    return () => clearInterval(id);
  }, [refresh]);

  const refreshQuietly = useCallback(() => refresh(false), [refresh]);
  useOnFocus(refreshQuietly);

  return { projects, loading, error, refresh };
}
