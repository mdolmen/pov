import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { apiFetch } from "@/lib/backend";
import type { Project } from "@/types";

type ProjectStatus = "open" | "paused" | "done" | "canceled";

const STATUS_MAP: Record<ProjectStatus, { status: "open" | "archived"; sub_status: "paused" | "done" | "canceled" | null }> = {
  open:     { status: "open",     sub_status: null },
  paused:   { status: "archived", sub_status: "paused" },
  done:     { status: "archived", sub_status: "done" },
  canceled: { status: "archived", sub_status: "canceled" },
};

function projectToStatus(p: Project): ProjectStatus {
  if (p.status === "open") return "open";
  return (p.sub_status ?? "paused") as ProjectStatus;
}

interface Props {
  open: boolean;
  project: Project;
  onClose: () => void;
  onUpdated: (p: Project) => void;
  onDeleted: () => void;
}

export function ProjectSettingsModal({ open, project, onClose, onUpdated, onDeleted }: Props) {
  const [name, setName] = useState(project.name);
  const [status, setStatus] = useState<ProjectStatus>(projectToStatus(project));
  const [pausedUntil, setPausedUntil] = useState<string>(project.paused_until ?? "");
  const [confirming, setConfirming] = useState(false);
  const [saving, setSaving] = useState(false);

  // Reset form whenever the modal opens for a different project.
  useEffect(() => {
    if (open) {
      setName(project.name);
      setStatus(projectToStatus(project));
      setPausedUntil(project.paused_until ?? "");
      setConfirming(false);
    }
  }, [open, project]);

  async function save() {
    setSaving(true);
    try {
      const { status: dbStatus, sub_status } = STATUS_MAP[status];
      const body: Record<string, string | null> = {
        name: name.trim(),
        status: dbStatus,
        sub_status,
        // empty string clears the date on the backend
        paused_until: status === "paused" ? pausedUntil : "",
      };
      const r = await apiFetch(`/projects/${project.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (!r.ok) throw new Error(await r.text());
      const updated: Project = await r.json();
      onUpdated(updated);
      onClose();
    } finally {
      setSaving(false);
    }
  }

  async function remove() {
    setSaving(true);
    try {
      const r = await apiFetch(`/projects/${project.id}`, { method: "DELETE" });
      if (!r.ok && r.status !== 204) throw new Error(await r.text());
      onDeleted();
    } finally {
      setSaving(false);
    }
  }

  const dirty =
    name.trim() !== project.name ||
    status !== projectToStatus(project) ||
    (status === "paused" && pausedUntil !== (project.paused_until ?? ""));

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-sm bg-[#f7f6f2] border border-stone-200 shadow-xl rounded-xl">
        <DialogHeader>
          <DialogTitle className="text-sm font-semibold text-stone-700 tracking-wide uppercase">
            Project settings
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3 pt-1">
          <label className="flex flex-col gap-1.5">
            <span className="text-[11px] font-medium tracking-wide text-stone-500 uppercase">Name</span>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="bg-white border-stone-200 text-sm text-stone-800 focus-visible:ring-stone-300"
            />
          </label>

          <label className="flex flex-col gap-1.5">
            <span className="text-[11px] font-medium tracking-wide text-stone-500 uppercase">Status</span>
            <Select value={status} onValueChange={(v) => setStatus(v as ProjectStatus)}>
              <SelectTrigger className="bg-white border-stone-200 text-sm text-stone-700">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="open">Open</SelectItem>
                <SelectItem value="paused">Paused</SelectItem>
                <SelectItem value="done">Done</SelectItem>
                <SelectItem value="canceled">Canceled</SelectItem>
              </SelectContent>
            </Select>
          </label>

          <div className="flex flex-col gap-1.5">
            <span className="text-[11px] font-medium tracking-wide text-stone-500 uppercase">File</span>
            <p
              className="text-xs text-stone-600 font-mono break-all select-text"
              title={project.file_path}
            >
              {project.file_path}
            </p>
          </div>

          {status === "paused" && (
            <label className="flex flex-col gap-1.5">
              <span className="text-[11px] font-medium tracking-wide text-stone-500 uppercase">
                Paused until <span className="text-stone-400 normal-case tracking-normal">(optional)</span>
              </span>
              <Input
                type="date"
                value={pausedUntil}
                onChange={(e) => setPausedUntil(e.target.value)}
                className="bg-white border-stone-200 text-sm text-stone-800 focus-visible:ring-stone-300"
              />
            </label>
          )}

          <div className="flex items-center justify-between pt-2 mt-1 border-t border-stone-200">
            {confirming ? (
              <div className="flex items-center gap-2 w-full">
                <span className="text-xs text-stone-600 flex-1">Delete this project?</span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setConfirming(false)}
                  className="text-stone-500 hover:text-stone-700 hover:bg-stone-100"
                >
                  Cancel
                </Button>
                <Button
                  size="sm"
                  onClick={remove}
                  disabled={saving}
                  className="bg-red-600 hover:bg-red-500 text-white text-xs"
                >
                  Delete
                </Button>
              </div>
            ) : (
              <>
                <button
                  onClick={() => setConfirming(true)}
                  className="text-xs text-red-600 hover:text-red-700 cursor-pointer"
                >
                  Delete project
                </button>
                <div className="flex gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={onClose}
                    className="text-stone-500 hover:text-stone-700 hover:bg-stone-100"
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    onClick={save}
                    disabled={!dirty || !name.trim() || saving}
                    className="bg-stone-800 hover:bg-stone-700 text-white text-xs"
                  >
                    {saving ? "Saving…" : "Save"}
                  </Button>
                </div>
              </>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
