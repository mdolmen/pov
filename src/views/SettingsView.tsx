import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
      </div>
    </div>
  );
}
