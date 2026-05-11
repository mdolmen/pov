import type { Filters, SectionFilters } from "@/types";

interface Props {
  filters: Filters;
  onToggle: (section: "projects" | "learning", key: "open" | "archived") => void;
  onSettings: () => void;
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
      className="flex items-center gap-2 px-1 py-1 w-full text-left rounded hover:bg-stone-200/60 transition-colors"
    >
      <span
        className="w-2 h-2 rounded-full shrink-0 transition-colors"
        style={{
          background: active ? "#57534e" : "transparent",
          border: active ? "none" : "1.5px solid #d6d3d1",
        }}
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

function SidebarSection({
  title,
  filters,
  onToggle,
}: {
  title: string;
  filters: SectionFilters;
  onToggle: (key: "open" | "archived") => void;
}) {
  return (
    <section>
      <p className="text-[9px] font-semibold tracking-widest uppercase text-stone-400 mb-1.5 px-1">
        {title}
      </p>
      <div className="flex flex-col">
        <FilterItem label="Open" active={filters.open} onToggle={() => onToggle("open")} />
        <FilterItem label="Archived" active={filters.archived} onToggle={() => onToggle("archived")} />
      </div>
    </section>
  );
}

export function Sidebar({ filters, onToggle, onSettings }: Props) {
  return (
    <div
      className="flex flex-col h-full py-4"
      style={{ background: "#f0efe9" }}
    >
      <nav className="flex flex-col gap-5 px-3 flex-1 overflow-y-auto">
        <SidebarSection
          title="Projects"
          filters={filters.projects}
          onToggle={(key) => onToggle("projects", key)}
        />
        <SidebarSection
          title="Learning"
          filters={filters.learning}
          onToggle={(key) => onToggle("learning", key)}
        />
      </nav>
      <div className="px-3 pt-3 shrink-0" style={{ borderTop: "1px solid rgba(0,0,0,0.06)" }}>
        <button
          onClick={onSettings}
          className="flex items-center gap-2 px-1 py-1 w-full text-left rounded hover:bg-stone-200/60 transition-colors"
        >
          <svg width="13" height="13" viewBox="0 0 13 13" fill="none" className="text-stone-400 shrink-0">
            <path
              d="M6.5 8.125a1.625 1.625 0 1 0 0-3.25 1.625 1.625 0 0 0 0 3.25Z"
              stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" strokeLinejoin="round"
            />
            <path
              d="M10.53 8.125a.894.894 0 0 0 .179.987l.032.033a1.083 1.083 0 1 1-1.532 1.532l-.033-.033a.894.894 0 0 0-.987-.178.894.894 0 0 0-.542.818v.091a1.083 1.083 0 1 1-2.167 0v-.048a.894.894 0 0 0-.585-.818.894.894 0 0 0-.987.178l-.033.033a1.083 1.083 0 1 1-1.532-1.532l.033-.033a.894.894 0 0 0 .178-.987.894.894 0 0 0-.818-.541h-.091a1.083 1.083 0 1 1 0-2.167h.048a.894.894 0 0 0 .818-.585.894.894 0 0 0-.178-.987l-.033-.033a1.083 1.083 0 1 1 1.532-1.532l.033.033a.894.894 0 0 0 .987.178h.043a.894.894 0 0 0 .542-.818v-.091a1.083 1.083 0 0 1 2.167 0v.048a.894.894 0 0 0 .541.818.894.894 0 0 0 .987-.178l.033-.033a1.083 1.083 0 1 1 1.532 1.532l-.033.033a.894.894 0 0 0-.178.987v.043a.894.894 0 0 0 .818.542h.091a1.083 1.083 0 0 1 0 2.166h-.048a.894.894 0 0 0-.818.542Z"
              stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" strokeLinejoin="round"
            />
          </svg>
          <span className="text-[13px] text-stone-500">Settings</span>
        </button>
      </div>
    </div>
  );
}
