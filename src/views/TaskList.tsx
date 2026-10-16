import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { InfoIcon } from "lucide-react";
import { AddTimeModal } from "@/components/AddTimeModal";
import { InlineMarkdown } from "@/components/InlineMarkdown";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "@/components/ui/menu";
import { Popover, PopoverContent, PopoverTitle, PopoverTrigger } from "@/components/ui/popover";
import { ProjectSettingsModal } from "@/components/ProjectSettingsModal";
import { TimeHeatmap } from "@/components/TimeHeatmap";
import { useTasks } from "@/hooks/useTasks";
import type { DeletedTask, HeadingItem, ListItem, Project, Subtask, Task, TaskMeta } from "@/types";

/** How long a deleted task can be brought back. */
const UNDO_MS = 5000;

function readTimeVisible(projectId: string): boolean {
  try {
    const v = localStorage.getItem(`pov.time.${projectId}.visible`);
    return v === null ? true : v === "1";
  } catch {
    return true;
  }
}

function writeTimeVisible(projectId: string, visible: boolean) {
  try {
    localStorage.setItem(`pov.time.${projectId}.visible`, visible ? "1" : "0");
  } catch {
    // localStorage might not exist (e.g. private mode); silently ignore.
  }
}

function readDoneHidden(projectId: string): boolean {
  try {
    return localStorage.getItem(`pov.done.${projectId}.hidden`) === "1";
  } catch {
    return false;
  }
}

function writeDoneHidden(projectId: string, hidden: boolean) {
  try {
    localStorage.setItem(`pov.done.${projectId}.hidden`, hidden ? "1" : "0");
  } catch {
    // localStorage might not exist (e.g. private mode); silently ignore.
  }
}

/** Where to scroll: `m:<milestone>` or `t:<task hash>`; a new `nonce` scrolls again. */
export interface Focus {
  key: string;
  nonce: number;
}

interface Props {
  project: Project;
  onClose: () => void;
  onProjectUpdated: (p: Project) => void;
  /** A task was toggled, (un)selected or triaged. */
  onChanged?: () => void;
  focus?: Focus | null;
}

function CheckboxIcon({ checked }: { checked: boolean }) {
  return (
    <div
      className="w-[15px] h-[15px] rounded-[4px] border-[1.5px] flex items-center justify-center shrink-0 transition-colors mt-0.5"
      style={{
        background: checked ? "var(--pov-next)" : "transparent",
        borderColor: checked ? "var(--pov-next)" : "var(--pov-check)",
      }}
    >
      {checked && (
        <svg width="8" height="6" viewBox="0 0 8 6" fill="none">
          <path d="M1 3l2 2 4-4" stroke="white" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      )}
    </div>
  );
}

function SubtaskRow({
  subtask,
  onToggle,
  disabled,
}: {
  subtask: Subtask;
  onToggle: () => void;
  disabled: boolean;
}) {
  return (
    <div className="flex items-start gap-2 pl-[25px]">
      <button
        onClick={(e) => { e.stopPropagation(); onToggle(); }}
        disabled={disabled}
        className={disabled ? "cursor-default" : "cursor-pointer"}
      >
        <CheckboxIcon checked={subtask.checked} />
      </button>
      <span
        className="flex-1 text-[12.5px] leading-5 min-w-0"
        style={{
          color: subtask.checked ? "var(--pov-ink-muted)" : "var(--pov-ink-2)",
          textDecoration: subtask.checked ? "line-through" : "none",
        }}
      >
        <InlineMarkdown text={subtask.text} />
      </span>
    </div>
  );
}

const pill =
  "shrink-0 text-[11px] font-semibold px-2 py-0.5 rounded-full whitespace-nowrap cursor-pointer transition-opacity ";
const onHover = "opacity-0 group-hover:opacity-100 focus-visible:opacity-100 ";

/** Stops a double-click in a popover from reaching the task row, through the portal. */
const stopDoubleClick = (e: React.MouseEvent) => e.stopPropagation();

/** Opens a link a tool attached: a URL in the browser, a path with its default app. */
async function openLink(target: string) {
  try {
    await (/^https?:\/\//i.test(target) ? openUrl(target) : openPath(target));
  } catch {
    // not in Tauri context (browser dev), or nothing to open it with
  }
}

/** The info icon of a task with metadata: its note and links. */
function InfoButton({ meta }: { meta: TaskMeta }) {
  return (
    <Popover>
      <PopoverTrigger
        onClick={(e) => e.stopPropagation()}
        aria-label="Task details"
        title="Task details"
        className="w-5 h-5 flex items-center justify-center rounded-full shrink-0 cursor-pointer text-ink-muted hover:text-ink hover:bg-hover"
      >
        <InfoIcon size={13} />
      </PopoverTrigger>
      <PopoverContent onDoubleClick={stopDoubleClick}>
        <PopoverTitle>Details</PopoverTitle>
        {meta.note && <p className="whitespace-pre-wrap text-ink-2">{meta.note}</p>}
        {meta.links.length > 0 && (
          <ul className="flex flex-col gap-1 mt-2">
            {meta.links.map((link, i) => (
              <li key={i}>
                <button
                  onClick={() => openLink(link.target)}
                  title={link.target}
                  className="text-left text-delegated hover:underline cursor-pointer"
                >
                  {link.label}
                </button>
              </li>
            ))}
          </ul>
        )}
        {!meta.note && meta.links.length === 0 && (
          <p className="text-ink-muted">No note or links yet.</p>
        )}
      </PopoverContent>
    </Popover>
  );
}

/** The "Delegate" pill: a menu of the tools when there are several, then a note to confirm. */
function DelegateButton({
  tools,
  onDelegate,
}: {
  tools: string[];
  onDelegate: (tool: string, note: string | null) => void;
}) {
  const anchor = useRef<HTMLButtonElement>(null);
  const [tool, setTool] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const className = pill + "text-ink-muted " + onHover + "hover:text-delegated hover:bg-delegated-soft";

  function pick(name: string) {
    setNote("");
    setTool(name);
  }

  return (
    <>
      {tools.length === 1 ? (
        <button
          ref={anchor}
          onClick={(e) => { e.stopPropagation(); pick(tools[0]); }}
          className={className}
        >
          Delegate
        </button>
      ) : (
        <Menu>
          <MenuTrigger ref={anchor} onClick={(e) => e.stopPropagation()} className={className}>
            Delegate
          </MenuTrigger>
          <MenuContent onDoubleClick={stopDoubleClick}>
            {tools.map((name) => (
              <MenuItem key={name} onClick={() => pick(name)}>
                {name}
              </MenuItem>
            ))}
          </MenuContent>
        </Menu>
      )}
      <Popover open={tool !== null} onOpenChange={(open) => { if (!open) setTool(null); }}>
        <PopoverContent anchor={anchor} onDoubleClick={stopDoubleClick}>
          <PopoverTitle>Delegate to {tool}</PopoverTitle>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (tool) onDelegate(tool, note.trim() || null);
              setTool(null);
            }}
          >
            <textarea
              aria-label="Note"
              placeholder="Note for the tool (optional)"
              value={note}
              onChange={(e) => setNote(e.target.value)}
              rows={3}
              className="w-full resize-none rounded-lg bg-pane px-2 py-1.5 text-[13px] ring-1 ring-line outline-none focus:ring-delegated"
            />
            <div className="flex justify-end mt-2">
              <button
                type="submit"
                className="text-[12px] font-semibold px-3 py-1 rounded-full text-white bg-delegated cursor-pointer"
              >
                Delegate
              </button>
            </div>
          </form>
        </PopoverContent>
      </Popover>
    </>
  );
}

function TaskRow({
  task,
  tools,
  onToggle,
  onSelect,
  onUnselect,
  onKeep,
  onDelete,
  onDelegate,
}: {
  task: Task;
  /** The project's tools. */
  tools: string[];
  onToggle: (hash: string) => void;
  onSelect: (hash: string) => void;
  onUnselect: (hash: string) => void;
  onKeep: (hash: string) => void;
  onDelete: (hash: string) => void;
  onDelegate: (hash: string, tool: string, note: string | null) => void;
}) {
  const done = task.is_done;
  // No actions until the tool sends it back.
  const delegated = task.delegated_to !== null && !done;
  const next = task.is_selected && !done && !delegated;
  const inbox = task.in_inbox && !done && !delegated;
  const canDelegate = !done && !delegated && tools.length > 0;
  const flip = () => (task.is_selected ? onUnselect(task.hash) : onSelect(task.hash));
  const band = delegated
    ? "var(--pov-delegated)"
    : next
      ? "var(--pov-next)"
      : inbox
        ? "var(--pov-inbox)"
        : null;

  return (
    <div
      data-task
      data-focus={`t:${task.hash}`}
      className="group px-3 py-2.5 border-t border-line first:border-t-0"
      style={{ boxShadow: band ? `inset 3px 0 0 ${band}` : undefined }}
      onDoubleClick={(e) => {
        e.stopPropagation();
        if (!done && !delegated) flip();
      }}
    >
      <div className="flex items-start gap-2.5">
        <button
          onClick={(e) => { e.stopPropagation(); onToggle(task.hash); }}
          disabled={delegated}
          className={delegated ? "cursor-default" : "cursor-pointer"}
        >
          <CheckboxIcon checked={done} />
        </button>
        <span
          className="flex-1 text-[13.5px] leading-5 min-w-0"
          style={{
            color: done ? "var(--pov-ink-muted)" : "var(--pov-ink)",
            textDecoration: done ? "line-through" : "none",
          }}
        >
          <InlineMarkdown text={task.text} />
        </span>
        {task.meta && <InfoButton meta={task.meta} />}
        {canDelegate && (
          <DelegateButton tools={tools} onDelegate={(tool, note) => onDelegate(task.hash, tool, note)} />
        )}
        {delegated && (
          <span
            title={`Delegated to ${task.delegated_to}`}
            className="shrink-0 text-[11px] font-semibold px-2 py-0.5 rounded-full whitespace-nowrap text-delegated bg-delegated-soft"
          >
            {task.delegated_to}
          </span>
        )}
        {!done && !delegated && (
          <button
            onClick={(e) => { e.stopPropagation(); flip(); }}
            className={
              pill +
              (next ? "text-next" : "text-ink-muted " + onHover + "hover:text-next hover:bg-next-soft")
            }
          >
            {next ? "Next" : "Mark next"}
          </button>
        )}
        {inbox && (
          <button
            onClick={(e) => { e.stopPropagation(); onKeep(task.hash); }}
            title="Keep in the backlog"
            className={pill + "text-ink-muted " + onHover + "hover:text-ink hover:bg-hover"}
          >
            Keep
          </button>
        )}
        {inbox && (
          <button
            onClick={(e) => { e.stopPropagation(); onDelete(task.hash); }}
            aria-label="Delete task"
            title="Delete task"
            className={
              "ml-1 w-5 h-5 flex items-center justify-center rounded-full shrink-0 cursor-pointer text-destructive hover:bg-hover transition-opacity " +
              onHover
            }
          >
            <svg width="9" height="9" viewBox="0 0 11 11" fill="none">
              <path d="M2 2l7 7M9 2L2 9" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
            </svg>
          </button>
        )}
      </div>
      {task.subtasks.length > 0 && (
        <div className="flex flex-col gap-1 mt-1.5">
          {task.subtasks.map((s) => (
            <SubtaskRow
              key={s.hash}
              subtask={s}
              onToggle={() => onToggle(s.hash)}
              disabled={delegated}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface Section {
  heading: HeadingItem | null;
  /** Progress of the `##` milestone the section belongs to; `shown` counts tasks not hidden. */
  progress: { done: number; total: number; shown: number } | null;
  tasks: Task[];
}

/**
 * Splits items into sections at each heading (level 1 is the file title and starts none),
 * keeping the tasks `isShown` accepts.
 */
function sectionsOf(items: ListItem[], isShown: (t: Task) => boolean): Section[] {
  const sections: Section[] = [{ heading: null, progress: null, tasks: [] }];
  let milestone: Section["progress"] = null;
  for (const item of items) {
    if (item.kind === "heading") {
      if (item.level === 2) milestone = { done: 0, total: 0, shown: 0 };
      if (item.level === 1) milestone = null;
      sections.push({ heading: item.level === 1 ? null : item, progress: milestone, tasks: [] });
      continue;
    }
    const shown = isShown(item);
    if (shown) sections[sections.length - 1].tasks.push(item);
    if (milestone) {
      milestone.total++;
      if (item.is_done) milestone.done++;
      if (shown) milestone.shown++;
    }
  }
  return sections;
}

function SectionCaption({ section }: { section: Section }) {
  const { heading, progress } = section;
  if (!heading) return null;
  if (heading.level === 2) {
    return (
      <p
        data-focus={`m:${heading.text}`}
        className="flex justify-between items-baseline text-[12px] font-semibold text-ink-2 mt-[18px] mb-1.5 mx-1"
      >
        <span><InlineMarkdown text={heading.text} /></span>
        {progress && (
          <span className="font-normal text-ink-muted tabular-nums">
            {progress.done} of {progress.total} done
          </span>
        )}
      </p>
    );
  }
  return (
    <p className="text-[11.5px] text-ink-muted mt-2.5 mb-1.5 mx-1">
      <InlineMarkdown text={heading.text} />
    </p>
  );
}

export function TaskList({ project, onClose, onProjectUpdated, onChanged, focus }: Props) {
  const { items, tasks, loading, toggle, select, unselect, keep, delegate, remove, undoRemove } = useTasks(
    project.id,
    onChanged,
  );
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [addTimeOpen, setAddTimeOpen] = useState(false);
  const [timeRefreshKey, setTimeRefreshKey] = useState(0);
  const [timeVisible, setTimeVisible] = useState<boolean>(() => readTimeVisible(project.id));
  const [doneHidden, setDoneHidden] = useState<boolean>(() => readDoneHidden(project.id));
  // Not saved: a quick look at what's left to triage, or at what's next.
  const [only, setOnly] = useState<"inbox" | "next" | "delegated" | null>(null);
  // The last deleted task, offered for undo for a few seconds.
  const [undo, setUndo] = useState<{ hash: string; text: string; deleted: DeletedTask } | null>(null);
  // Why the last delegation failed, shown for a few seconds.
  const [error, setError] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const tracksTime = project.type === "learning";

  async function deleteTask(hash: string) {
    const text = tasks.find((t) => t.hash === hash)?.text ?? "";
    const deleted = await remove(hash);
    if (deleted) setUndo({ hash, text, deleted });
  }

  async function undoDelete() {
    if (!undo) return;
    setUndo(null);
    await undoRemove(undo.hash, undo.deleted);
  }

  async function delegateTask(hash: string, tool: string, note: string | null) {
    try {
      await delegate(hash, tool, note);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    if (!error) return;
    const timer = setTimeout(() => setError(null), UNDO_MS);
    return () => clearTimeout(timer);
  }, [error]);

  // A new delete restarts the timer; another project drops the offer.
  useEffect(() => {
    if (!undo) return;
    const timer = setTimeout(() => setUndo(null), UNDO_MS);
    return () => clearTimeout(timer);
  }, [undo]);
  useEffect(() => setUndo(null), [project.id]);

  function toggleTime() {
    const next = !timeVisible;
    setTimeVisible(next);
    writeTimeVisible(project.id, next);
  }

  function toggleDoneHidden() {
    setDoneHidden(!doneHidden);
    writeDoneHidden(project.id, !doneHidden);
  }

  // Scroll to the milestone or task picked under Next up, once the tasks are shown.
  useEffect(() => {
    if (!focus || loading) return;
    const targets = scrollRef.current?.querySelectorAll<HTMLElement>("[data-focus]") ?? [];
    [...targets].find((el) => el.dataset.focus === focus.key)?.scrollIntoView({ block: "start" });
  }, [focus?.key, focus?.nonce, loading]);

  // A milestone caption shows when its milestone shows tasks, even if they're all under `###`.
  const inboxCount = tasks.filter((t) => t.in_inbox && !t.is_done).length;
  const nextCount = tasks.filter((t) => t.is_selected && !t.is_done).length;
  const delegatedCount = tasks.filter((t) => t.delegated_to !== null && !t.is_done).length;
  // Triaging the last inbox task, finishing the last next one, or the last delegated task coming
  // back, leaves the filter on with nothing to show: turn it off.
  useEffect(() => {
    if (
      (only === "inbox" && inboxCount === 0) ||
      (only === "next" && nextCount === 0) ||
      (only === "delegated" && delegatedCount === 0)
    ) {
      setOnly(null);
    }
  }, [only, inboxCount, nextCount, delegatedCount]);

  const sections = sectionsOf(items, (t) =>
    only === "inbox"
      ? t.in_inbox && !t.is_done
      : only === "next"
        ? t.is_selected && !t.is_done
        : only === "delegated"
          ? t.delegated_to !== null && !t.is_done
          : !(doneHidden && t.is_done),
  ).filter(
    (s) => s.tasks.length > 0 || (s.heading?.level === 2 && s.progress!.shown > 0),
  );
  const doneCount = tasks.filter((t) => t.is_done).length;

  async function openInEditor() {
    try {
      await invoke("open_in_editor", { path: project.file_path });
    } catch {
      // not in Tauri context (browser dev)
    }
  }

  const iconButton =
    "w-[26px] h-[26px] flex items-center justify-center text-ink-muted hover:text-ink hover:bg-hover transition-colors rounded-md shrink-0 cursor-pointer";

  return (
    <div className="flex flex-col h-full">
      <header className="flex items-center gap-1 h-[60px] pl-6 pr-3 shrink-0 border-b border-line">
        <span className="flex-1 text-[19px] font-bold tracking-tight text-ink truncate">
          {project.name}
        </span>

        {nextCount > 0 && (
          <button
            onClick={() => setOnly(only === "next" ? null : "next")}
            aria-pressed={only === "next"}
            title={only === "next" ? "Show all tasks" : `Show only the ${nextCount} next tasks`}
            className={
              "mr-1.5 text-[12px] font-medium px-2.5 py-1 rounded-full whitespace-nowrap cursor-pointer " +
              (only === "next" ? "text-next bg-next-soft" : "text-ink-2 ring-1 ring-line hover:text-ink hover:bg-hover")
            }
          >
            Next
          </button>
        )}

        {inboxCount > 0 && (
          <button
            onClick={() => setOnly(only === "inbox" ? null : "inbox")}
            aria-pressed={only === "inbox"}
            title={only === "inbox" ? "Show all tasks" : `Show only the ${inboxCount} inbox tasks`}
            className={
              "mr-1.5 text-[12px] font-medium px-2.5 py-1 rounded-full whitespace-nowrap cursor-pointer " +
              (only === "inbox" ? "text-inbox bg-inbox-soft" : "text-ink-2 ring-1 ring-line hover:text-ink hover:bg-hover")
            }
          >
            Inbox
          </button>
        )}

        {delegatedCount > 0 && (
          <button
            onClick={() => setOnly(only === "delegated" ? null : "delegated")}
            aria-pressed={only === "delegated"}
            title={only === "delegated" ? "Show all tasks" : `Show only the ${delegatedCount} delegated tasks`}
            className={
              "mr-1.5 text-[12px] font-medium px-2.5 py-1 rounded-full whitespace-nowrap cursor-pointer " +
              (only === "delegated"
                ? "text-delegated bg-delegated-soft"
                : "text-ink-2 ring-1 ring-line hover:text-ink hover:bg-hover")
            }
          >
            Delegated
          </button>
        )}

        {doneCount > 0 && (
          <button
            onClick={toggleDoneHidden}
            aria-pressed={doneHidden}
            title={`${doneHidden ? "Show" : "Hide"} the ${doneCount} done tasks`}
            className={
              "mr-1.5 text-[12px] font-medium px-2.5 py-1 rounded-full whitespace-nowrap cursor-pointer " +
              (doneHidden ? "text-next bg-next-soft" : "text-ink-2 ring-1 ring-line hover:text-ink hover:bg-hover")
            }
          >
            {doneHidden ? `${doneCount} done hidden` : "Hide done"}
          </button>
        )}

        <button
          onClick={openInEditor}
          className={iconButton}
          aria-label="Open in default text editor"
          title="Open in default text editor"
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

        <button
          onClick={() => setSettingsOpen(true)}
          className={iconButton}
          aria-label="Project settings"
          title="Project settings"
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="3" stroke="currentColor" strokeWidth="1.8" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>

        <button
          onClick={onClose}
          className={iconButton}
          aria-label="Close project"
          title="Close (Esc)"
        >
          <svg width="11" height="11" viewBox="0 0 11 11" fill="none">
            <path d="M2 2l7 7M9 2L2 9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </button>
      </header>

      <ProjectSettingsModal
        open={settingsOpen}
        project={project}
        onClose={() => setSettingsOpen(false)}
        onUpdated={onProjectUpdated}
        onDeleted={() => { setSettingsOpen(false); onClose(); }}
      />

      <div className="relative flex-1 min-h-0 flex flex-col">
        <div ref={scrollRef} className="flex-1 min-h-0 overflow-y-auto px-6 pt-2 pb-6">
          {loading ? (
            <div className="flex items-center justify-center h-32">
              <span className="text-xs text-ink-muted">Loading…</span>
            </div>
          ) : (
            <>
              {sections.map((section, i) => (
                <div key={i} className={section.heading ? "" : "mt-2.5"}>
                  <SectionCaption section={section} />
                  {section.tasks.length > 0 && (
                    <div className="bg-surface rounded-xl ring-1 ring-line overflow-hidden">
                      {section.tasks.map((t) => (
                        <TaskRow
                          key={t.hash}
                          task={t}
                          tools={project.tools}
                          onToggle={toggle}
                          onSelect={select}
                          onUnselect={unselect}
                          onKeep={keep}
                          onDelete={deleteTask}
                          onDelegate={delegateTask}
                        />
                      ))}
                    </div>
                  )}
                </div>
              ))}

              {tasks.length > 0 && sections.length === 0 && (
                <p className="my-[18px] mx-1 text-[13px] text-ink-muted">
                  All tasks are done.{" "}
                  <button onClick={toggleDoneHidden} className="text-next cursor-pointer">
                    Show done
                  </button>
                </p>
              )}

              {tasks.length === 0 && (
                <div className="flex items-center justify-center h-32">
                  <span className="text-xs text-ink-muted">No tasks.</span>
                </div>
              )}
            </>
          )}
        </div>

        {(error || undo) && (
          <div className="absolute bottom-4 left-1/2 -translate-x-1/2 max-w-[80%] flex flex-col items-center gap-2">
            {error && (
              <div
                role="alert"
                onClick={() => setError(null)}
                className="max-w-full px-4 py-1.5 rounded-full bg-ink text-pane text-[12.5px] shadow-lg truncate cursor-pointer"
              >
                {error}
              </div>
            )}
            {undo && (
              <div
                role="status"
                className="max-w-full flex items-center gap-3 pl-4 pr-2 py-1.5 rounded-full bg-ink text-pane text-[12.5px] shadow-lg whitespace-nowrap"
              >
                <span className="truncate">Deleted “{undo.text}”</span>
                <button
                  onClick={undoDelete}
                  className="font-semibold text-inbox-soft px-2 py-0.5 rounded-full hover:bg-white/10 cursor-pointer"
                >
                  Undo
                </button>
              </div>
            )}
          </div>
        )}
      </div>

      {tracksTime && (
        <div className="shrink-0 border-t border-line">
          {timeVisible ? (
            <div className="px-6 pt-2 pb-3">
              <div className="flex justify-between items-center mb-1">
                <button
                  onClick={toggleTime}
                  className="text-[10px] tracking-wide uppercase text-ink-muted hover:text-ink-2 transition-colors cursor-pointer"
                >
                  Hide
                </button>
                <button
                  onClick={() => setAddTimeOpen(true)}
                  className="w-5 h-5 flex items-center justify-center text-ink-muted hover:text-ink-2 transition-colors rounded cursor-pointer text-base leading-none pb-0.5"
                  aria-label="Record time"
                  title="Record time"
                >
                  +
                </button>
              </div>
              <TimeHeatmap projectId={project.id} refreshKey={timeRefreshKey} />
            </div>
          ) : (
            <div className="flex justify-end px-6 py-1.5">
              <button
                onClick={toggleTime}
                className="text-[10px] tracking-wide uppercase text-ink-muted hover:text-ink-2 transition-colors cursor-pointer"
              >
                Show time
              </button>
            </div>
          )}

          <AddTimeModal
            open={addTimeOpen}
            projectId={project.id}
            onClose={() => setAddTimeOpen(false)}
            onRecorded={() => setTimeRefreshKey((k) => k + 1)}
          />
        </div>
      )}
    </div>
  );
}
