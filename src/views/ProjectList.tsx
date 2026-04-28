import { useState } from "react";
import { ProjectCard } from "@/components/ProjectCard";
import { AddProjectModal } from "@/components/AddProjectModal";
import { useProjects } from "@/hooks/useProjects";
import type { Project } from "@/types";

interface Props {
  onSelectProject: (project: Project) => void;
  addOpen: boolean;
  onAddClose: () => void;
}

export function ProjectList({ onSelectProject, addOpen, onAddClose }: Props) {
  const { projects, loading, refresh } = useProjects();
  const [archivedExpanded, setArchivedExpanded] = useState(false);

  const open = projects.filter((p) => p.status === "open");
  const archived = projects.filter((p) => p.status === "archived");

  return (
    <>
      <div className="flex-1 overflow-y-auto px-3 py-3 space-y-5">
        {loading ? (
          <div className="flex items-center justify-center h-32">
            <span className="text-xs text-stone-400">Loading…</span>
          </div>
        ) : (
          <>
            {open.length > 0 && (
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

            {archived.length > 0 && (
              <section>
                <button
                  onClick={() => setArchivedExpanded((v) => !v)}
                  className="flex items-center gap-1.5 mb-2 px-0.5 group"
                >
                  <svg
                    width="8"
                    height="8"
                    viewBox="0 0 8 8"
                    fill="none"
                    className="text-stone-300 group-hover:text-stone-400 transition-colors shrink-0"
                    style={{
                      transform: archivedExpanded ? "rotate(90deg)" : "rotate(0deg)",
                      transition: "transform 150ms",
                    }}
                  >
                    <path d="M2 1l4 3-4 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                  <span className="text-[10px] font-semibold tracking-widest uppercase text-stone-400 group-hover:text-stone-500 transition-colors">
                    Archived ({archived.length})
                  </span>
                </button>
                {archivedExpanded && (
                  <div className="flex flex-col gap-1.5">
                    {archived.map((p) => (
                      <ProjectCard
                        key={p.id}
                        project={p}
                        onClick={() => onSelectProject(p)}
                      />
                    ))}
                  </div>
                )}
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

      <AddProjectModal
        open={addOpen}
        onClose={onAddClose}
        onCreated={refresh}
      />
    </>
  );
}
