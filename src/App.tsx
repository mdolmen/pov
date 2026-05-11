import { useState } from "react";
import { ProjectList } from "@/views/ProjectList";
import { TaskList } from "@/views/TaskList";
import { SettingsView } from "@/views/SettingsView";
import { Sidebar } from "@/components/Sidebar";
import type { Filters, Project } from "@/types";

const DEFAULT_FILTERS: Filters = {
  projects: { open: true, archived: false },
  learning: { open: true, archived: false },
};

export default function App() {
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [filters, setFilters] = useState<Filters>(DEFAULT_FILTERS);
  const [activityRefreshKey, setActivityRefreshKey] = useState(0);

  function toggleFilter(section: "projects" | "learning", key: "open" | "archived") {
    setFilters((f) => ({
      ...f,
      [section]: { ...f[section], [key]: !f[section][key] },
    }));
  }

  if (selectedProject) {
    return (
      <div
        className="flex flex-col h-screen select-none overflow-hidden"
        style={{ background: "#f7f6f2" }}
      >
        <TaskList
          project={selectedProject}
          onBack={() => setSelectedProject(null)}
          onProjectUpdated={setSelectedProject}
        />
      </div>
    );
  }

  if (settingsOpen) {
    return <SettingsView onBack={() => setSettingsOpen(false)} />;
  }

  return (
    <div
      className="flex flex-col h-screen select-none overflow-hidden"
      style={{ background: "#f7f6f2" }}
    >
      <header
        className="flex items-center justify-between px-4 py-3 shrink-0"
        style={{ borderBottom: "1px solid rgba(0,0,0,0.06)" }}
      >
        <button
          onClick={() => setSidebarOpen((v) => !v)}
          className="w-6 h-6 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors rounded"
          aria-label="Menu"
        >
          <svg width="13" height="10" viewBox="0 0 13 10" fill="none">
            <rect width="13" height="1.5" rx="0.75" fill="currentColor" />
            <rect y="4.25" width="13" height="1.5" rx="0.75" fill="currentColor" />
            <rect y="8.5" width="13" height="1.5" rx="0.75" fill="currentColor" />
          </svg>
        </button>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setActivityRefreshKey((k) => k + 1)}
            className="w-6 h-6 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors rounded"
            aria-label="Refresh activity"
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path d="M10.5 6a4.5 4.5 0 1 1-1.01-2.845" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
              <path d="M10.5 2v2.5H8" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </button>
          <button
            onClick={() => setAddOpen(true)}
            className="w-6 h-6 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors rounded text-lg leading-none pb-0.5"
            aria-label="Add project"
          >
            +
          </button>
        </div>
      </header>

      <div className="flex-1 relative overflow-hidden">
        {sidebarOpen && (
          <div
            className="absolute inset-0 z-10 flex"
            onClick={() => setSidebarOpen(false)}
          >
            <div className="h-full w-48 shrink-0" onClick={(e) => e.stopPropagation()}>
              <Sidebar
                filters={filters}
                onToggle={toggleFilter}
                onSettings={() => { setSidebarOpen(false); setSettingsOpen(true); }}
              />
            </div>
            <div className="flex-1" style={{ background: "rgba(0,0,0,0.08)" }} />
          </div>
        )}

        <ProjectList
          onSelectProject={setSelectedProject}
          addOpen={addOpen}
          onAddClose={() => setAddOpen(false)}
          filters={filters}
          activityRefreshKey={activityRefreshKey}
        />
      </div>
    </div>
  );
}
