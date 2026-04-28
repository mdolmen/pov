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

      <AddProjectModal
        open={addOpen}
        onClose={onAddClose}
        onCreated={refresh}
      />
    </>
  );
}
