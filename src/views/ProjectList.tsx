import { useState } from "react";
import { ProjectCard } from "@/components/ProjectCard";
import { AddProjectModal } from "@/components/AddProjectModal";
import { useProjects } from "@/hooks/useProjects";
import type { Filters, Project, Tab } from "@/types";

interface Props {
  onSelectProject: (project: Project) => void;
  addOpen: boolean;
  onAddClose: () => void;
  filters: Filters;
}

export function ProjectList({ onSelectProject, addOpen, onAddClose, filters }: Props) {
  const { projects, loading, refresh } = useProjects();
  const [activeTab, setActiveTab] = useState<Tab>("projects");

  const tabType = activeTab === "projects" ? "project" : "learning";
  const tabFilters = filters[activeTab];

  const open = projects.filter((p) => p.type === tabType && p.status === "open");
  const archived = projects.filter((p) => p.type === tabType && p.status === "archived");

  return (
    <>
      <div className="flex flex-col h-full">
        {/* Tabs */}
        <div
          className="flex gap-4 px-4 pt-3 shrink-0"
          style={{ borderBottom: "1px solid rgba(0,0,0,0.06)" }}
        >
          {(["projects", "learning"] as Tab[]).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className="pb-2.5 text-[11px] font-semibold tracking-widest uppercase transition-colors"
              style={{
                color: activeTab === tab ? "#292524" : "#a8a29e",
                borderBottom: activeTab === tab ? "2px solid #292524" : "2px solid transparent",
              }}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* List */}
        <div className="flex-1 overflow-y-auto px-3 py-3 space-y-5">
          {loading ? (
            <div className="flex items-center justify-center h-32">
              <span className="text-xs text-stone-400">Loading…</span>
            </div>
          ) : (
            <>
              {tabFilters.open && open.length > 0 && (
                <section>
                  <p className="text-[10px] font-semibold tracking-widest uppercase text-stone-400 mb-2 px-0.5">
                    Open
                  </p>
                  <div className="flex flex-col gap-1.5">
                    {open.map((p) => (
                      <ProjectCard
                        key={p.id}
                        project={p}
                        onClick={() => onSelectProject(p)}
                      />
                    ))}
                  </div>
                </section>
              )}

              {tabFilters.archived && archived.length > 0 && (
                <section>
                  <p className="text-[10px] font-semibold tracking-widest uppercase text-stone-400 mb-2 px-0.5">
                    Archived
                  </p>
                  <div className="flex flex-col gap-1.5">
                    {archived.map((p) => (
                      <ProjectCard
                        key={p.id}
                        project={p}
                        onClick={() => onSelectProject(p)}
                      />
                    ))}
                  </div>
                </section>
              )}

              {projects.length === 0 && (
                <div className="flex flex-col items-center justify-center h-40 gap-2">
                  <p className="text-xs text-stone-400">No projects yet.</p>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      <AddProjectModal
        open={addOpen}
        onClose={onAddClose}
        onCreated={refresh}
      />
    </>
  );
}
