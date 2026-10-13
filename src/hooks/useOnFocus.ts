import { useEffect } from "react";

/** Runs `fn` when the window gains focus: files may have changed while it was in the background. */
export function useOnFocus(fn: () => void) {
  useEffect(() => {
    window.addEventListener("focus", fn);
    return () => window.removeEventListener("focus", fn);
  }, [fn]);
}
