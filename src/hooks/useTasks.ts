import { useState, useEffect, useCallback } from "react";
import { apiFetch } from "@/lib/backend";
import type { Task } from "@/types";

export function useTasks(projectId: string) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchTasks = useCallback(async () => {
    const r = await apiFetch(`/projects/${projectId}/tasks`);
    if (r.ok) setTasks(await r.json());
    setLoading(false);
  }, [projectId]);

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  async function toggle(hash: string) {
    setTasks((ts) =>
      ts.map((t) => {
        if (t.hash === hash) {
          const checked = !t.checked;
          return { ...t, checked, is_done: t.subtasks.length === 0 ? checked : t.is_done };
        }
        const si = t.subtasks.findIndex((s) => s.hash === hash);
        if (si >= 0) {
          const subtasks = t.subtasks.map((s, i) =>
            i === si ? { ...s, checked: !s.checked } : s
          );
          return { ...t, subtasks, is_done: subtasks.every((s) => s.checked) };
        }
        return t;
      })
    );
    const r = await apiFetch(`/projects/${projectId}/tasks/${hash}`, { method: "PATCH" });
    if (r.ok) {
      const updated: Task = await r.json();
      setTasks((ts) =>
        ts.map((t) =>
          t.hash === updated.hash || t.subtasks.some((s) => s.hash === hash) ? updated : t
        )
      );
    } else {
      fetchTasks();
    }
  }

  async function select(hash: string) {
    setTasks((ts) => ts.map((t) => (t.hash === hash ? { ...t, is_selected: true } : t)));
    await apiFetch(`/projects/${projectId}/tasks/${hash}/select`, { method: "POST" });
  }

  async function unselect(hash: string) {
    setTasks((ts) => ts.map((t) => (t.hash === hash ? { ...t, is_selected: false } : t)));
    await apiFetch(`/projects/${projectId}/tasks/${hash}/select`, { method: "DELETE" });
  }

  return { tasks, loading, toggle, select, unselect };
}
