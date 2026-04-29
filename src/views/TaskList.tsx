import { invoke } from "@tauri-apps/api/core";
import { useTasks } from "@/hooks/useTasks";
import type { Project, Subtask, Task } from "@/types";

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

function SubtaskRow({ subtask, onToggle }: { subtask: Subtask; onToggle: () => void }) {
  return (
    <div className="flex items-start gap-2.5 px-3 pb-1">
      <div className="w-3.5 shrink-0" />
      <button
        onClick={(e) => { e.stopPropagation(); onToggle(); }}
        className="mt-0.5 cursor-pointer"
      >
        <CheckboxIcon checked={subtask.checked} />
      </button>
      <span
        className="text-[12px] leading-5"
        style={{
          color: subtask.checked ? "#a8a29e" : "#78716c",
          textDecoration: subtask.checked ? "line-through" : "none",
        }}
      >
        {subtask.text}
      </span>
    </div>
  );
}

const SELECTED_BAND = "#7c3aed";

function TaskRow({
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
      className="group bg-white rounded-lg overflow-hidden flex ring-1 ring-black/[0.07] hover:ring-[1.5px] hover:ring-blue-400 transition-shadow"
      onDoubleClick={(e) => {
        e.stopPropagation();
        if (!done) task.is_selected ? onUnselect(task.hash) : onSelect(task.hash);
      }}
    >
      {/* left band — visible only on selected tasks */}
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
          <div className="pb-2">
            {task.subtasks.map((s) => (
              <SubtaskRow key={s.hash} subtask={s} onToggle={() => onToggle(s.hash)} />
            ))}
          </div>
        )}
      </div>
    </div>
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

export function TaskList({ project, onBack }: Props) {
  const { tasks, loading, toggle, select, unselect } = useTasks(project.id);

  const selected = tasks.filter((t) => t.is_selected && !t.is_done);
  const pending = tasks.filter((t) => !t.is_selected && !t.is_done);
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
      {/* Header */}
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

      {/* Content */}
      <div className="flex-1 min-h-0 overflow-y-auto px-3 py-3">
        {loading ? (
          <div className="flex items-center justify-center h-32">
            <span className="text-xs text-stone-400">Loading…</span>
          </div>
        ) : (
          <>
            <SectionLabel>TODO</SectionLabel>
            <div className="flex flex-col gap-1.5 mb-1">
              {selected.map((t) => (
                <TaskRow key={t.hash} task={t} onToggle={toggle} onSelect={select} onUnselect={unselect} />
              ))}
            </div>
            <Divider />

            <div className="flex flex-col gap-1.5">
              {pending.map((t) => (
                <TaskRow key={t.hash} task={t} onToggle={toggle} onSelect={select} onUnselect={unselect} />
              ))}
            </div>

            {done.length > 0 && (
              <>
                <Divider />
                <SectionLabel>Done</SectionLabel>
                {/* outer div clips to ~5 cards; inner flex div holds the real layout */}
                <div className="overflow-y-auto" style={{ maxHeight: "268px" }}>
                  <div className="flex flex-col gap-1.5">
                    {done.map((t) => (
                      <TaskRow key={t.hash} task={t} onToggle={toggle} onSelect={select} onUnselect={unselect} />
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
