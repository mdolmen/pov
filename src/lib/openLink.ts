import { openPath, openUrl } from "@tauri-apps/plugin-opener";

/**
 * Opens a link from a task: a web or mail URL, or a file path (or `file://` URL), with `app`
 * (an app name, as macOS `open -a` takes it) or the default app.
 */
export async function openLink(target: string, app?: string) {
  try {
    if (/^(https?|mailto):/i.test(target)) await openUrl(target, app);
    else await openPath(target.startsWith("file://") ? decodeURI(target.slice(7)) : target, app);
  } catch {
    // not in Tauri context (browser dev), or nothing to open it with
  }
}
