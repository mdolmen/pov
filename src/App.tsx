import { useState } from "react";
import { ProjectList } from "@/views/ProjectList";
import type { Project } from "@/types";

export default function App() {
  const [_selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [addOpen, setAddOpen] = useState(false);

  return (
    <div
      className="flex flex-col h-screen select-none"
      style={{ background: "#f7f6f2" }}
    >
      <header
        className="flex items-center justify-between px-4 py-3 shrink-0"
        style={{ borderBottom: "1px solid rgba(0,0,0,0.06)" }}
      >
        <span className="text-[10px] font-semibold tracking-widest uppercase text-stone-400">
          pov
        </span>
        <div className="flex items-center gap-1">
          {/* sidebar toggle — Phase 5 */}
          <button
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
        </div>
      </header>

      <ProjectList
        onSelectProject={setSelectedProject}
        addOpen={addOpen}
        onAddClose={() => setAddOpen(false)}
      />
    </div>
  );
}
