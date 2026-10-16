import { openPath, openUrl } from "@tauri-apps/plugin-opener";

/**
 * Opens a link from a task: a web or mail URL with its handler, a file path (or `file://` URL)
 * with its default app.
 */
export async function openLink(target: string) {
  try {
    if (/^(https?|mailto):/i.test(target)) await openUrl(target);
    else await openPath(target.startsWith("file://") ? decodeURI(target.slice(7)) : target);
  } catch {
    // not in Tauri context (browser dev), or nothing to open it with
  }
}
