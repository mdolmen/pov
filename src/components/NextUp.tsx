import { InlineMarkdown } from "@/components/InlineMarkdown";
import type { NextUpEntry } from "@/types";

/** Where an entry scrolls to in the task pane: `m:<milestone>` or `t:<task hash>`. */
export function entryKey(entry: NextUpEntry): string {
  return entry.milestone !== null ? `m:${entry.milestone}` : `t:${entry.task_hash}`;
}

function ProgressRing({ done, total }: { done: number; total: number }) {
  const c = 2 * Math.PI * 7.5;
  const f = total ? done / total : 0;
  return (
    <svg className="row-span-2" width="20" height="20" viewBox="0 0 20 20" aria-hidden="true">
      <circle cx="10" cy="10" r="7.5" fill="none" strokeWidth="2.5" stroke="var(--pov-track)" />
      {f > 0 && (
        <circle
          cx="10"
          cy="10"
          r="7.5"
          fill="none"
          strokeWidth="2.5"
          stroke="var(--pov-live)"
          strokeLinecap="round"
          strokeDasharray={`${f * c} ${c}`}
          transform="rotate(-90 10 10)"
        />
      )}
    </svg>
  );
}

interface Props {
  entries: NextUpEntry[];
  /** `project_id` + entry key of the picked entry, if its project is open. */
  selected: string | null;
  onPick: (entry: NextUpEntry) => void;
}

export function NextUp({ entries, selected, onPick }: Props) {
  if (entries.length === 0) {
    return (
      <div className="bg-surface rounded-xl ring-1 ring-line px-3.5 py-4 text-[13px] text-ink-2">
        <b className="block text-ink mb-0.5">Nothing marked next</b>
        Open a project and mark the tasks you'll handle next; their milestones collect here.
      </div>
    );
  }

  return (
    <div className="bg-surface rounded-xl ring-1 ring-line overflow-hidden" role="group" aria-label="Next up">
      {entries.map((e) => {
        const key = `${e.project_id}|${entryKey(e)}`;
        const isMilestone = e.milestone !== null;
        return (
          <button
            key={key}
            onClick={() => onPick(e)}
            className={
              "w-full grid grid-cols-[20px_1fr_auto] grid-rows-[auto_auto] gap-x-2.5 gap-y-px items-center px-3 py-2.5 text-left cursor-pointer border-t border-line first:border-t-0 " +
              (selected === key ? "bg-next-soft" : "hover:bg-hover")
            }
          >
            <ProgressRing done={e.done} total={e.total} />
            <span className="text-[13.5px] font-semibold truncate">{e.project_name}</span>
            {isMilestone ? (
              <span
                className={
                  "justify-self-end text-[11px] font-semibold text-next px-2 py-px rounded-full " +
                  (selected === key ? "bg-surface" : "bg-next-soft")
                }
              >
                {e.next_count} next
              </span>
            ) : (
              <span />
            )}
            <span className="text-[12.5px] text-ink-muted truncate">
              <InlineMarkdown text={(isMilestone ? e.milestone : e.task_text) ?? ""} />
            </span>
            {e.total > 0 ? (
              <span className="justify-self-end text-[12px] text-ink-muted tabular-nums">
                {e.done} of {e.total} done
              </span>
            ) : (
              <span />
            )}
          </button>
        );
      })}
    </div>
  );
}
