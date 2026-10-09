import { useEffect, useState } from "react";
import { ProjectTiles } from "@/views/ProjectTiles";
import { TaskList, type Focus } from "@/views/TaskList";
import { SettingsView } from "@/views/SettingsView";
import { ActivityFooter } from "@/components/ActivityFooter";
import { AddProjectModal } from "@/components/AddProjectModal";
import { NextUp, entryKey } from "@/components/NextUp";
import { useNextUp } from "@/hooks/useNextUp";
import { useProjects } from "@/hooks/useProjects";
import { cleanupSelected } from "@/lib/backend";
import type { NextUpEntry, Project, Tab } from "@/types";

export default function App() {
  const { projects, loading, refresh: refreshProjects } = useProjects();
  const { entries: nextUp, refresh: refreshNextUp } = useNextUp();
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [focus, setFocus] = useState<Focus | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>("projects");
  const [activityRefreshKey, setActivityRefreshKey] = useState(0);

  function openProject(project: Project) {
    setSelectedProject(project);
    setFocus(null);
  }

  function closeProject() {
    setSelectedProject(null);
    setFocus(null);
    refreshProjects(false);
  }

  function refreshCounts() {
    refreshProjects(false);
    refreshNextUp();
  }

  // Opens the entry's project, on its tab, scrolled to the milestone or task.
  function pickEntry(entry: NextUpEntry) {
    const project = projects.find((p) => p.id === entry.project_id);
    if (!project) return;
    setSelectedProject(project);
    setActiveTab(project.type === "learning" ? "learning" : "projects");
    setFocus((f) => ({ key: entryKey(entry), nonce: (f?.nonce ?? 0) + 1 }));
  }

  // Esc closes the task pane; a dialog handles its own Esc.
  useEffect(() => {
    if (!selectedProject) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape" && !document.querySelector('[role="dialog"]')) closeProject();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [selectedProject]);

  if (settingsOpen) {
    return <SettingsView onBack={() => setSettingsOpen(false)} />;
  }

  const iconButton =
    "w-[26px] h-[26px] flex items-center justify-center text-ink-muted hover:text-ink hover:bg-hover transition-colors rounded-md cursor-pointer";

  return (
    <div className="flex h-screen select-none overflow-hidden text-ink">
      <section
        aria-label="Projects"
        className="w-[400px] shrink-0 flex flex-col bg-ground border-r border-divider"
      >
        <header className="flex items-start gap-0.5 pl-5 pr-3 pt-4 pb-2.5 shrink-0">
          <div className="flex-1">
            <p className="text-[12px] text-ink-muted">
              {new Date().toLocaleDateString("en-GB", { weekday: "long", day: "numeric", month: "long" })}
            </p>
            <h1 className="mt-0.5 text-[26px] font-bold tracking-tight leading-tight">
              Next up <span className="text-ink-muted font-medium">{nextUp.length}</span>
            </h1>
          </div>
          <button
            onClick={async () => {
              await cleanupSelected();
              refreshCounts();
              setActivityRefreshKey((k) => k + 1);
            }}
            className={iconButton}
            aria-label="Refresh activity"
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path d="M10.5 6a4.5 4.5 0 1 1-1.01-2.845" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
              <path d="M10.5 2v2.5H8" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </button>
          <button onClick={() => setAddOpen(true)} className={iconButton} aria-label="Add project">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path d="M6 1.5v9M1.5 6h9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </button>
          <button
            onClick={() => setSettingsOpen(true)}
            className={iconButton}
            aria-label="Settings"
          >
            <svg width="13" height="10" viewBox="0 0 13 10" fill="none">
              <rect width="13" height="1.5" rx="0.75" fill="currentColor" />
              <rect y="4.25" width="13" height="1.5" rx="0.75" fill="currentColor" />
              <rect y="8.5" width="13" height="1.5" rx="0.75" fill="currentColor" />
            </svg>
          </button>
        </header>

        <div className="flex-1 min-h-0 overflow-y-auto px-3 pt-0.5 pb-4">
          <NextUp
            entries={nextUp}
            selected={selectedProject && focus ? `${selectedProject.id}|${focus.key}` : null}
            onPick={pickEntry}
          />
          <ProjectTiles
            projects={projects}
            loading={loading}
            activeTab={activeTab}
            onTabChange={setActiveTab}
            selectedId={selectedProject?.id ?? null}
            onSelect={openProject}
            activityRefreshKey={activityRefreshKey}
          />
        </div>

        <ActivityFooter tab={activeTab} refreshKey={activityRefreshKey} />
      </section>

      <section
        aria-label={selectedProject ? `Tasks of ${selectedProject.name}` : "Tasks"}
        className="flex-1 min-w-0 flex flex-col bg-pane"
      >
        {selectedProject ? (
          <TaskList
            key={selectedProject.id}
            project={selectedProject}
            onClose={closeProject}
            onProjectUpdated={(p) => { setSelectedProject(p); refreshCounts(); }}
            onChanged={refreshCounts}
            focus={focus}
          />
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center gap-2.5 p-6 text-center">
            <svg width="40" height="40" viewBox="0 0 40 40" fill="none" stroke="var(--pov-check)" strokeWidth="2" strokeLinecap="round">
              <rect x="5" y="7" width="7" height="7" rx="2" />
              <path d="M17 10.5h18" />
              <rect x="5" y="17" width="7" height="7" rx="2" />
              <path d="M17 20.5h14" />
              <rect x="5" y="27" width="7" height="7" rx="2" />
              <path d="M17 30.5h16" />
            </svg>
            <p className="text-sm text-ink-2">Choose a project to show its tasks</p>
          </div>
        )}
      </section>

      <AddProjectModal open={addOpen} onClose={() => setAddOpen(false)} onCreated={refreshCounts} />
    </div>
  );
}
