import type { Filters } from "@/types";

const LEARNING_KEYS = ["maths", "papers", "books", "videos"] as const;
const LEARNING_LABELS: Record<string, string> = {
  maths: "Maths",
  papers: "Papers",
  books: "Books",
  videos: "Videos",
};

interface Props {
  filters: Filters;
  onToggle: (key: keyof Filters) => void;
}

function FilterItem({
  label,
  active,
  onToggle,
}: {
  label: string;
  active: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      onClick={onToggle}
      className="flex items-center gap-2 px-1 py-1 w-full text-left rounded hover:bg-stone-200/60 transition-colors group"
    >
      <span
        className="w-2 h-2 rounded-full shrink-0 transition-colors"
        style={{ background: active ? "#57534e" : "transparent", border: active ? "none" : "1.5px solid #d6d3d1" }}
      />
      <span
        className="text-[13px] transition-colors"
        style={{ color: active ? "#292524" : "#a8a29e" }}
      >
        {label}
      </span>
    </button>
  );
}

export function Sidebar({ filters, onToggle }: Props) {
  return (
    <div
      className="flex flex-col h-full overflow-y-auto py-4"
      style={{ background: "#f0efe9" }}
    >
      <nav className="flex flex-col gap-5 px-3">
        <section>
          <p className="text-[9px] font-semibold tracking-widest uppercase text-stone-400 mb-1.5 px-1">
            Projects
          </p>
          <div className="flex flex-col">
            <FilterItem label="Open" active={filters.open} onToggle={() => onToggle("open")} />
            <FilterItem label="Archived" active={filters.archived} onToggle={() => onToggle("archived")} />
          </div>
        </section>

        <section>
          <p className="text-[9px] font-semibold tracking-widest uppercase text-stone-400 mb-1.5 px-1">
            Learning
          </p>
          <div className="flex flex-col">
            {LEARNING_KEYS.map((key) => (
              <FilterItem
                key={key}
                label={LEARNING_LABELS[key]}
                active={filters[key]}
                onToggle={() => onToggle(key)}
              />
            ))}
          </div>
        </section>
      </nav>
    </div>
  );
}
