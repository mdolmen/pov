import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
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
import type { Status, SubStatus } from "@/types";

interface Props {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}

export function AddProjectModal({ open: isOpen, onClose, onCreated }: Props) {
  const [filePath, setFilePath] = useState("");
  const [name, setName] = useState("");
  const [status, setStatus] = useState<Status>("open");
  const [subStatus, setSubStatus] = useState<SubStatus>(null);
  const [loading, setLoading] = useState(false);

  async function pickFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (!selected) return;
    const path = typeof selected === "string" ? selected : (selected as { path: string }).path;
    setFilePath(path);
    if (!name) {
      const parts = path.split("/");
      const filename = parts[parts.length - 2] ?? parts[parts.length - 1];
      setName(filename.replace(/\.md$/i, ""));
    }
  }

  async function submit() {
    if (!filePath || !name) return;
    setLoading(true);
    try {
      const r = await apiFetch("/projects", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name,
          file_path: filePath,
          status,
          sub_status: subStatus,
        }),
      });
      if (!r.ok) throw new Error(await r.text());
      onCreated();
      handleClose();
    } finally {
      setLoading(false);
    }
  }

  function handleClose() {
    setFilePath("");
    setName("");
    setStatus("open");
    setSubStatus(null);
    onClose();
  }

  return (
    <Dialog open={isOpen} onOpenChange={(o) => !o && handleClose()}>
      <DialogContent className="sm:max-w-sm bg-[#f7f6f2] border border-stone-200 shadow-xl rounded-xl">
        <DialogHeader>
          <DialogTitle className="text-sm font-semibold text-stone-700 tracking-wide uppercase">
            Add project
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3 pt-1">
          <button
            onClick={pickFile}
            className="w-full text-left px-3 py-2.5 rounded-lg border border-stone-200 bg-white text-sm transition-colors hover:border-stone-300"
          >
            {filePath ? (
              <span className="text-stone-700 truncate block">{filePath}</span>
            ) : (
              <span className="text-stone-400">Choose a TODO.md file…</span>
            )}
          </button>

          <Input
            placeholder="Project name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="bg-white border-stone-200 text-sm text-stone-800 placeholder:text-stone-400 focus-visible:ring-stone-300"
          />

          <Select value={status} onValueChange={(v) => setStatus(v as Status)}>
            <SelectTrigger className="bg-white border-stone-200 text-sm text-stone-700">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="open">Open</SelectItem>
              <SelectItem value="archived">Archived</SelectItem>
            </SelectContent>
          </Select>

          {status === "archived" && (
            <Select
              value={subStatus ?? ""}
              onValueChange={(v) => setSubStatus(v as SubStatus)}
            >
              <SelectTrigger className="bg-white border-stone-200 text-sm text-stone-700">
                <SelectValue placeholder="Reason…" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="done">Done</SelectItem>
                <SelectItem value="paused">Paused</SelectItem>
                <SelectItem value="canceled">Canceled</SelectItem>
              </SelectContent>
            </Select>
          )}

          <div className="flex justify-end gap-2 pt-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClose}
              className="text-stone-500 hover:text-stone-700 hover:bg-stone-100"
            >
              Cancel
            </Button>
            <Button
              size="sm"
              disabled={!filePath || !name || loading}
              onClick={submit}
              className="bg-stone-800 hover:bg-stone-700 text-white text-xs"
            >
              {loading ? "Adding…" : "Add"}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
