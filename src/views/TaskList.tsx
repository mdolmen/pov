import { invoke } from "@tauri-apps/api/core";
import { useTasks } from "@/hooks/useTasks";
import type { HeadingItem, ListItem, Project, Subtask, Task } from "@/types";

interface Props {
  project: Project;
  onBack: () => void;
}

function CheckboxIcon({ checked }: { checked: boolean }) {
  return (
    <div
      className="w-3.5 h-3.5 rounded-[3px] border flex items-center justify-center shrink-0 transition-colors mt-px"
      style={{
        background: checked ? "#44403c" : "transparent",
        borderColor: checked ? "#44403c" : "#d6d3d1",
      }}
    >
      {checked && (
        <svg width="8" height="6" viewBox="0 0 8 6" fill="none">
          <path d="M1 3l2 2 4-4" stroke="white" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      )}
    </div>
  );
}

const SELECTED_BAND = "#7c3aed";

function SubtaskRow({ subtask, onToggle }: { subtask: Subtask; onToggle: () => void }) {
  return (
    <div className="flex items-start gap-2.5 pl-9 pr-3 pb-1.5">
      <button
        onClick={(e) => { e.stopPropagation(); onToggle(); }}
        className="mt-0.5 cursor-pointer"
      >
        <CheckboxIcon checked={subtask.checked} />
      </button>
      <span
        className="flex-1 text-[12px] leading-5 min-w-0"
        style={{
          color: subtask.checked ? "#a8a29e" : "#57534e",
          textDecoration: subtask.checked ? "line-through" : "none",
        }}
      >
        {subtask.text}
      </span>
    </div>
  );
}

function TaskCard({
  task,
  onToggle,
  onSelect,
  onUnselect,
}: {
  task: Task;
  onToggle: (hash: string) => void;
  onSelect: (hash: string) => void;
  onUnselect: (hash: string) => void;
}) {
  const done = task.is_done;

  return (
    <div
      className={
        "group bg-white rounded-lg overflow-hidden flex ring-1 ring-black/[0.07] transition-shadow" +
        (done ? "" : " hover:ring-blue-400")
      }
      onDoubleClick={(e) => {
        e.stopPropagation();
        if (!done) task.is_selected ? onUnselect(task.hash) : onSelect(task.hash);
      }}
    >
      <div
        className="w-[3px] shrink-0"
        style={{ backgroundColor: task.is_selected && !done ? SELECTED_BAND : "transparent" }}
      />
      <div className="flex-1 min-w-0">
        <div className="flex items-start gap-2.5 px-3 py-2.5">
          <button
            onClick={(e) => { e.stopPropagation(); onToggle(task.hash); }}
            className="mt-0.5 cursor-pointer"
          >
            <CheckboxIcon checked={done} />
          </button>
          <span
            className="flex-1 text-sm leading-5 min-w-0"
            style={{
              color: done ? "#a8a29e" : "#292524",
              textDecoration: done ? "line-through" : "none",
            }}
          >
            {task.text}
          </span>
          {!done && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                task.is_selected ? onUnselect(task.hash) : onSelect(task.hash);
              }}
              className="opacity-0 group-hover:opacity-100 transition-opacity w-5 h-5 flex items-center justify-center text-stone-400 hover:text-stone-700 shrink-0 cursor-pointer"
            >
              {task.is_selected ? (
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                  <path d="M2 2l6 6M8 2L2 8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                </svg>
              ) : (
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                  <path d="M5 1v8M1 5h8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                </svg>
              )}
            </button>
          )}
        </div>
        {task.subtasks.length > 0 && (
          <div className="pb-1">
            {task.subtasks.map((s) => (
              <SubtaskRow key={s.hash} subtask={s} onToggle={() => onToggle(s.hash)} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function HeadingLabel({ heading }: { heading: HeadingItem }) {
  return (
    <p className="text-[9px] font-semibold tracking-widest uppercase text-stone-400 mt-3 mb-1 px-1 first:mt-0">
      {heading.text}
    </p>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-[9px] font-semibold tracking-widest uppercase text-stone-400 mb-2 px-1">
      {children}
    </p>
  );
}

function Divider() {
  return <div className="border-t my-3" style={{ borderColor: "rgba(0,0,0,0.06)" }} />;
}

function renderPendingItems(
  items: ListItem[],
  onToggle: (hash: string) => void,
  onSelect: (hash: string) => void,
  onUnselect: (hash: string) => void,
) {
  const result: React.ReactNode[] = [];
  // Walk items in order, emitting headings only when they precede pending tasks
  let i = 0;
  while (i < items.length) {
    const item = items[i];
    if (item.kind === "heading") {
      // Look ahead: does this heading have any pending tasks before the next same-or-higher heading?
      let hasPending = false;
      for (let j = i + 1; j < items.length; j++) {
        const next = items[j];
        if (next.kind === "heading" && next.level <= item.level) break;
        if (next.kind === "task" && !next.is_done && !next.is_selected) { hasPending = true; break; }
      }
      if (hasPending) {
        result.push(<HeadingLabel key={`h-${i}`} heading={item} />);
      }
      i++;
      continue;
    }

    // task
    const task = item as Task;
    if (task.is_done || task.is_selected) { i++; continue; }

    result.push(
      <TaskCard key={task.hash} task={task} onToggle={onToggle} onSelect={onSelect} onUnselect={onUnselect} />
    );
    i++;
  }

  return result;
}

export function TaskList({ project, onBack }: Props) {
  const { items, tasks, loading, toggle, select, unselect } = useTasks(project.id);

  const selected = tasks.filter((t) => t.is_selected && !t.is_done);
  const done = tasks.filter((t) => t.is_done);

  async function openInEditor() {
    try {
      await invoke("open_in_editor", { path: project.file_path });
    } catch {
      // not in Tauri context (browser dev)
    }
  }

  return (
    <div className="flex flex-col h-full">
      <header
        className="flex items-center gap-3 px-4 py-3 shrink-0"
        style={{ borderBottom: "1px solid rgba(0,0,0,0.06)" }}
      >
        <button
          onClick={onBack}
          className="w-6 h-6 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors rounded shrink-0 cursor-pointer"
          aria-label="Back"
        >
          <svg width="8" height="13" viewBox="0 0 8 13" fill="none">
            <path d="M7 1L1 6.5L7 12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>

        <span
          className="flex-1 text-sm font-medium text-stone-800 truncate"
          style={{ fontFeatureSettings: '"ss01"' }}
        >
          {project.name}
        </span>

        <button
          onClick={openInEditor}
          className="w-6 h-6 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors rounded shrink-0 cursor-pointer"
          aria-label="Edit in vim"
        >
          <svg width="13" height="13" viewBox="0 0 13 13" fill="none">
            <path
              d="M9 1.5l2.5 2.5L4 11.5H1.5V9L9 1.5z"
              stroke="currentColor"
              strokeWidth="1.3"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
      </header>

      <div className="flex-1 min-h-0 overflow-y-auto px-3 py-3">
        {loading ? (
          <div className="flex items-center justify-center h-32">
            <span className="text-xs text-stone-400">Loading…</span>
          </div>
        ) : (
          <>
            <SectionLabel>TODO</SectionLabel>

            {selected.length > 0 && (
              <div className="flex flex-col gap-1.5 mb-1">
                {selected.map((t) => (
                  <TaskCard key={t.hash} task={t} onToggle={toggle} onSelect={select} onUnselect={unselect} />
                ))}
              </div>
            )}

            <Divider />

            <div className="flex flex-col gap-1.5">
              {renderPendingItems(items, toggle, select, unselect)}
            </div>

            {done.length > 0 && (
              <>
                <Divider />
                <SectionLabel>Done</SectionLabel>
                <div className="overflow-y-auto" style={{ maxHeight: "268px" }}>
                  <div className="flex flex-col gap-1.5">
                    {done.map((t) => (
                      <TaskCard key={t.hash} task={t} onToggle={toggle} onSelect={select} onUnselect={unselect} />
                    ))}
                  </div>
                </div>
              </>
            )}

            {tasks.length === 0 && (
              <div className="flex items-center justify-center h-32">
                <span className="text-xs text-stone-400">No tasks.</span>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
