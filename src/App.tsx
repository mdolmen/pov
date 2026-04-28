import { useState } from "react";
import { ProjectList } from "@/views/ProjectList";
import { Sidebar } from "@/components/Sidebar";
import type { Filters, Project } from "@/types";

const DEFAULT_FILTERS: Filters = {
  open: true,
  archived: false,
  maths: true,
  papers: true,
  books: true,
  videos: true,
};

export default function App() {
  const [_selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [filters, setFilters] = useState<Filters>(DEFAULT_FILTERS);

  function toggleFilter(key: keyof Filters) {
    setFilters((f) => ({ ...f, [key]: !f[key] }));
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
        <button
          onClick={() => setAddOpen(true)}
          className="w-6 h-6 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors rounded text-lg leading-none pb-0.5"
          aria-label="Add project"
        >
          +
        </button>
      </header>

      <div className="flex-1 relative overflow-hidden">
        {sidebarOpen && (
          <div
            className="absolute inset-0 z-10 flex"
            onClick={() => setSidebarOpen(false)}
          >
            <div
              className="h-full w-48 shrink-0"
              onClick={(e) => e.stopPropagation()}
            >
              <Sidebar filters={filters} onToggle={toggleFilter} />
            </div>
            <div className="flex-1" style={{ background: "rgba(0,0,0,0.08)" }} />
          </div>
        )}

        <ProjectList
          onSelectProject={setSelectedProject}
          addOpen={addOpen}
          onAddClose={() => setAddOpen(false)}
          filters={filters}
        />
      </div>
    </div>
  );
}
