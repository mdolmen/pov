import { useState } from "react";
import { ProjectTile } from "@/components/ProjectTile";
import { byActivity, useProjectDays } from "@/hooks/useProjectDays";
import type { Project, Tab } from "@/types";

interface Props {
  projects: Project[];
  loading: boolean;
  activeTab: Tab;
  onTabChange: (tab: Tab) => void;
  selectedId: string | null;
  onSelect: (project: Project) => void;
  activityRefreshKey: number;
}

function Chevron({ expanded }: { expanded: boolean }) {
  return (
    <svg
      width="8"
      height="8"
      viewBox="0 0 8 8"
      fill="none"
      className="text-ink-muted shrink-0"
      style={{ transform: expanded ? "rotate(90deg)" : "rotate(0deg)", transition: "transform 150ms" }}
    >
      <path d="M2 1l4 3-4 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function ProjectTiles({
  projects,
  loading,
  activeTab,
  onTabChange,
  selectedId,
  onSelect,
  activityRefreshKey,
}: Props) {
  const [archivedExpanded, setArchivedExpanded] = useState(false);

  const tabType = activeTab === "projects" ? "project" : "learning";

  const inTab = projects.filter((p) => p.type === tabType);
  const daysOf = useProjectDays(inTab, activityRefreshKey);

  // Most next tasks first, then most active over the tile's 8 days, then by name.
  const order = (a: Project, b: Project) =>
    b.selected_count - a.selected_count ||
    byActivity(daysOf(a.id), daysOf(b.id)) ||
    a.name.localeCompare(b.name);

  const open = inTab.filter((p) => p.status === "open").sort(order);
  const archived = inTab.filter((p) => p.status === "archived").sort(order);

  const tiles = (list: Project[]) => (
    <div className="grid grid-cols-2 gap-2">
      {list.map((p) => (
        <ProjectTile
          key={p.id}
          project={p}
          selected={p.id === selectedId}
          days={daysOf(p.id)}
          onClick={() => onSelect(p)}
        />
      ))}
    </div>
  );

  return (
    <div role="group" aria-label="Project list">
      <div className="flex justify-between items-center mt-5 mb-2 mx-1 text-[12px] font-semibold text-ink-2">
        <span>Open {activeTab}</span>
        <span className="inline-flex bg-[var(--pov-hover)] rounded-[7px] p-0.5">
          {(["projects", "learning"] as Tab[]).map((tab) => (
            <button
              key={tab}
              onClick={() => onTabChange(tab)}
              className={
                "px-2.5 py-0.5 rounded-[5px] text-[12px] font-medium cursor-pointer " +
                (activeTab === tab ? "bg-surface text-ink shadow-sm" : "text-ink-2")
              }
            >
              {tab === "projects" ? "Projects" : "Learning"}
            </button>
          ))}
        </span>
      </div>

      {tiles(open)}

      {archived.length > 0 && (
        <section className="mt-4">
          <button
            onClick={() => setArchivedExpanded((v) => !v)}
            className="flex items-center gap-1.5 mb-2 mx-1 cursor-pointer"
          >
            <Chevron expanded={archivedExpanded} />
            <span className="text-[12px] font-semibold text-ink-2">
              Archived ({archived.length})
            </span>
          </button>
          {archivedExpanded && tiles(archived)}
        </section>
      )}

      {!loading && projects.length === 0 && (
        <div className="flex items-center justify-center h-24">
          <p className="text-xs text-ink-muted">No projects yet.</p>
        </div>
      )}
    </div>
  );
}
