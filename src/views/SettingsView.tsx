import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listTools, removeTool, saveTool } from "@/lib/backend";
import type { ToolEntry } from "@/types";

const EMPTY_TOOL: ToolEntry = { name: "", command: "", pidfile: "" };

const field =
  "w-full rounded border border-stone-200 bg-white px-2 py-1 text-[12px] text-stone-800 outline-none focus:border-stone-400 disabled:text-stone-400";

/** The tools tasks can be delegated to: list, add, edit, remove. */
function ToolsSection() {
  const [tools, setTools] = useState<ToolEntry[]>([]);
  const [form, setForm] = useState<ToolEntry>(EMPTY_TOOL);
  // The tool being edited; its name can't change.
  const [editing, setEditing] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listTools().then(setTools, () => {});
  }, []);

  function reset() {
    setForm(EMPTY_TOOL);
    setEditing(null);
    setError(null);
  }

  async function save(e: React.FormEvent) {
    e.preventDefault();
    try {
      setTools(await saveTool(form));
      reset();
    } catch (err) {
      setError(String(err));
    }
  }

  async function remove(name: string) {
    try {
      setTools(await removeTool(name));
      if (editing === name) reset();
    } catch (err) {
      setError(String(err));
    }
  }

  const set = (key: keyof ToolEntry) => (e: React.ChangeEvent<HTMLInputElement>) =>
    setForm({ ...form, [key]: e.target.value });

  return (
    <section className="mt-8">
      <p className="text-[9px] font-semibold tracking-widest uppercase text-stone-400 mb-3">Tools</p>
      <p className="text-[11px] text-stone-400 mb-3">
        Tasks can be delegated to these; link them to a project in its settings.
      </p>
      {tools.length > 0 && (
        <ul aria-label="Tools" className="flex flex-col gap-2 mb-4">
          {tools.map((t) => (
            <li key={t.name} className="flex items-start gap-3">
              <div className="flex-1 min-w-0">
                <p className="text-[13px] text-stone-700">{t.name}</p>
                <p className="text-[11px] font-mono text-stone-400 break-all select-text">{t.command}</p>
                <p className="text-[11px] font-mono text-stone-400 break-all select-text">{t.pidfile}</p>
              </div>
              <button
                onClick={() => { setForm(t); setEditing(t.name); setError(null); }}
                aria-label={`Edit ${t.name}`}
                className="text-[11px] text-stone-500 hover:text-stone-700 cursor-pointer"
              >
                Edit
              </button>
              <button
                onClick={() => remove(t.name)}
                aria-label={`Remove ${t.name}`}
                className="text-[11px] text-red-600 hover:text-red-700 cursor-pointer"
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}
      <form onSubmit={save} className="flex flex-col gap-2">
        <input aria-label="Tool name" placeholder="Name" value={form.name} onChange={set("name")} disabled={editing !== null} className={field} />
        <input aria-label="Command" placeholder="Command (absolute path)" value={form.command} onChange={set("command")} className={field + " font-mono"} />
        <input aria-label="Pidfile" placeholder="Pidfile (absolute path)" value={form.pidfile} onChange={set("pidfile")} className={field + " font-mono"} />
        {error && <p role="alert" className="text-[11px] text-red-600">{error}</p>}
        <div className="flex justify-end gap-2">
          {editing && (
            <button type="button" onClick={reset} className="px-3 py-1.5 rounded text-[12px] text-stone-500 hover:text-stone-700 cursor-pointer">
              Cancel
            </button>
          )}
          <button
            type="submit"
            disabled={!form.name.trim() || !form.command.trim() || !form.pidfile.trim()}
            className="px-3 py-1.5 rounded text-[12px] font-medium bg-stone-800 text-stone-50 disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer"
          >
            {editing ? "Save tool" : "Add tool"}
          </button>
        </div>
      </form>
    </section>
  );
}

interface Props {
  onBack: () => void;
}

export function SettingsView({ onBack }: Props) {
  const [installed, setInstalled] = useState<boolean | null>(null);
  const [installing, setInstalling] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>("is_cli_installed").then(setInstalled);
  }, []);

  async function handleInstall() {
    setInstalling(true);
    setMessage(null);
    try {
      const msg = await invoke<string>("install_cli");
      setMessage(msg);
      setInstalled(true);
    } catch (e) {
      setMessage(String(e));
    } finally {
      setInstalling(false);
    }
  }

  return (
    <div className="flex flex-col h-screen select-none overflow-hidden" style={{ background: "#f7f6f2" }}>
      <header
        className="flex items-center gap-3 px-4 py-3 shrink-0"
        style={{ borderBottom: "1px solid rgba(0,0,0,0.06)" }}
      >
        <button
          onClick={onBack}
          className="w-6 h-6 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors rounded"
          aria-label="Back"
        >
          <svg width="7" height="12" viewBox="0 0 7 12" fill="none">
            <path d="M6 1L1 6L6 11" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
        <span className="text-[13px] font-medium text-stone-700">Settings</span>
      </header>

      <div className="flex-1 overflow-y-auto p-6">
        <section>
          <p className="text-[9px] font-semibold tracking-widest uppercase text-stone-400 mb-3">
            Command Line
          </p>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-[13px] text-stone-700">Install CLI</p>
              <p className="text-[11px] text-stone-400 mt-0.5">Installs <code className="font-mono">pov</code> to ~/.local/bin</p>
            </div>
            <button
              onClick={handleInstall}
              disabled={installed !== false || installing}
              className="px-3 py-1.5 rounded text-[12px] font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              style={{
                background: installed ? "#e7e5e0" : "#292524",
                color: installed ? "#a8a29e" : "#fafaf9",
              }}
            >
              {installed ? "CLI installed" : installing ? "Installing…" : "Install CLI"}
            </button>
          </div>
          {message && (
            <p className="mt-3 text-[11px] font-mono whitespace-pre-wrap text-stone-500 bg-stone-100 rounded px-3 py-2">
              {message}
            </p>
          )}
        </section>

        <ToolsSection />
      </div>
    </div>
  );
}
