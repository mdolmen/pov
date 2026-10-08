import { useState, useEffect, useCallback } from "react";
import { listTasks, selectTask, toggleTask, unselectTask } from "@/lib/backend";
import type { ListItem, Task } from "@/types";

export function useTasks(projectId: string) {
  const [items, setItems] = useState<ListItem[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchTasks = useCallback(async () => {
    try {
      setItems(await listTasks(projectId));
    } catch {
      // Keep the current list; the next fetch will retry.
    }
    setLoading(false);
  }, [projectId]);

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const tasks = items.filter((i): i is Task => i.kind === "task");

  async function toggle(hash: string) {
    setItems((prev) => {
      const tIdx = prev.findIndex((i) => i.kind === "task" && (i as Task).hash === hash);
      if (tIdx >= 0) {
        const task = prev[tIdx] as Task;
        if (task.subtasks.length > 0) {
          const newDone = !task.is_done;
          const updated: Task = {
            ...task,
            checked: newDone,
            is_done: newDone,
            subtasks: task.subtasks.map((s) => ({ ...s, checked: newDone })),
          };
          return prev.map((i, idx) => idx === tIdx ? updated : i);
        }
        const checked = !task.checked;
        return prev.map((i, idx) =>
          idx === tIdx ? { ...task, checked, is_done: checked } : i
        );
      }
      // subtask toggle
      return prev.map((i) => {
        if (i.kind !== "task") return i;
        const t = i as Task;
        const si = t.subtasks.findIndex((s) => s.hash === hash);
        if (si < 0) return i;
        const subtasks = t.subtasks.map((s, idx) =>
          idx === si ? { ...s, checked: !s.checked } : s
        );
        return { ...t, subtasks, is_done: subtasks.every((s) => s.checked) };
      });
    });

    try {
      const updated = await toggleTask(projectId, hash);
      setItems((prev) =>
        prev.map((i) => {
          if (i.kind !== "task") return i;
          const t = i as Task;
          if (t.hash === updated.hash) return updated;
          if (t.subtasks.some((s) => s.hash === hash)) return updated;
          return i;
        })
      );
    } catch {
      fetchTasks();
    }
  }

  async function select(hash: string) {
    setItems((prev) =>
      prev.map((i) => i.kind === "task" && (i as Task).hash === hash ? { ...i, is_selected: true } : i)
    );
    await selectTask(projectId, hash);
  }

  async function unselect(hash: string) {
    setItems((prev) =>
      prev.map((i) => i.kind === "task" && (i as Task).hash === hash ? { ...i, is_selected: false } : i)
    );
    await unselectTask(projectId, hash);
  }

  return { items, tasks, loading, toggle, select, unselect };
}
