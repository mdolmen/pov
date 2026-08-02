import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { apiFetch } from "@/lib/backend";
import { formatMinutes } from "@/lib/utils";

const STEP_MINUTES = 15;

interface Props {
  open: boolean;
  projectId: string;
  onClose: () => void;
  onRecorded: () => void;
}

function todayKey(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

export function AddTimeModal({ open: isOpen, projectId, onClose, onRecorded }: Props) {
  const [date, setDate] = useState(todayKey);
  const [minutes, setMinutes] = useState(60);
  const [topic, setTopic] = useState("");
  const [topics, setTopics] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    let cancelled = false;
    (async () => {
      const r = await apiFetch(`/projects/${projectId}/time/topics`);
      if (!r.ok || cancelled) return;
      setTopics(await r.json());
    })();
    return () => {
      cancelled = true;
    };
  }, [isOpen, projectId]);

  async function submit() {
    setLoading(true);
    try {
      const r = await apiFetch(`/projects/${projectId}/time`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ date, minutes, topic: topic.trim() || null }),
      });
      if (!r.ok) throw new Error(await r.text());
      onRecorded();
      handleClose();
    } finally {
      setLoading(false);
    }
  }

  function handleClose() {
    setDate(todayKey());
    setMinutes(60);
    setTopic("");
    onClose();
  }

  return (
    <Dialog open={isOpen} onOpenChange={(o) => !o && handleClose()}>
      <DialogContent className="sm:max-w-sm bg-[#f7f6f2] border border-stone-200 shadow-xl rounded-xl">
        <DialogHeader>
          <DialogTitle className="text-sm font-semibold text-stone-700 tracking-wide uppercase">
            Record time
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3 pt-1">
          <div className="flex items-center gap-2">
            <Input
              type="date"
              value={date}
              max={todayKey()}
              onChange={(e) => setDate(e.target.value)}
              className="flex-1 bg-white border-stone-200 text-sm text-stone-800 focus-visible:ring-stone-300"
            />
            <span className="w-14 text-right text-sm text-stone-800 tabular-nums">
              {formatMinutes(minutes)}
            </span>
            <div className="flex">
              <button
                onClick={() => setMinutes((m) => Math.max(STEP_MINUTES, m - STEP_MINUTES))}
                disabled={minutes <= STEP_MINUTES}
                className="w-8 h-9 rounded-l-lg border border-stone-200 bg-white text-stone-600 hover:border-stone-300 disabled:opacity-40 disabled:hover:border-stone-200 cursor-pointer text-base leading-none"
                aria-label="Less time"
              >
                −
              </button>
              <button
                onClick={() => setMinutes((m) => m + STEP_MINUTES)}
                className="w-8 h-9 -ml-px rounded-r-lg border border-stone-200 bg-white text-stone-600 hover:border-stone-300 cursor-pointer text-base leading-none"
                aria-label="More time"
              >
                +
              </button>
            </div>
          </div>

          <Input
            placeholder="Topic (optional)"
            value={topic}
            onChange={(e) => setTopic(e.target.value)}
            className="bg-white border-stone-200 text-sm text-stone-800 placeholder:text-stone-400 focus-visible:ring-stone-300"
          />

          {topics.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {topics.map((t) => (
                <button
                  key={t}
                  onClick={() => setTopic(t)}
                  className="px-2 py-1 rounded-md border text-[11px] transition-colors cursor-pointer"
                  style={{
                    borderColor: topic === t ? "#3b6fb0" : "#e7e5e4",
                    background: topic === t ? "#e8f0fa" : "#ffffff",
                    color: topic === t ? "#2b5484" : "#57534e",
                  }}
                >
                  {t}
                </button>
              ))}
            </div>
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
              disabled={loading || !date}
              onClick={submit}
              className="bg-stone-800 hover:bg-stone-700 text-white text-xs"
            >
              {loading ? "Saving…" : "Record"}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
