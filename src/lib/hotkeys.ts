import { useEffect, useRef } from "react";

/** True when a keystroke is destined for a text field and shouldn't trigger a shortcut. */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if (target.getAttribute("role") === "textbox") return true;
  const ce = target.getAttribute("contenteditable");
  return target.isContentEditable || ce === "" || ce === "true";
}

type Handler = (e: KeyboardEvent) => void;
export type HotkeyMap = Record<string, Handler>;

interface Options {
  enabled?: boolean;
  /** Keys that should still fire while a text field is focused (e.g. "Escape", "/"). */
  allowInInput?: string[];
}

/**
 * Register window-level keyboard shortcuts.
 *
 * Keys are matched case-insensitively. Prefix with `mod+` for Ctrl/Cmd and
 * `shift+` for Shift (Shift is only added for named keys — `?` already implies it).
 * The handler map can change every render; only `enabled` re-binds the listener.
 */
export function useHotkeys(map: HotkeyMap, opts: Options = {}) {
  const mapRef = useRef(map);
  mapRef.current = map;
  const { enabled = true } = opts;
  const allowRef = useRef(opts.allowInInput ?? []);
  allowRef.current = opts.allowInInput ?? [];

  useEffect(() => {
    if (!enabled) return;

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.altKey || e.isComposing) return;
      const mod = e.ctrlKey || e.metaKey;

      const named = e.key.length > 1;
      const parts: string[] = [];
      if (mod) parts.push("mod");
      if (e.shiftKey && named) parts.push("shift");
      parts.push(named ? e.key : e.key.toLowerCase());
      const combo = parts.join("+");
      const bare = named ? e.key : e.key.toLowerCase();

      // Only fall back to the bare key when no Ctrl/Cmd is held, so we never
      // hijack things like Ctrl+R (reload) or Cmd+F.
      const handler =
        mapRef.current[combo] ?? (mod ? undefined : mapRef.current[bare]);
      if (!handler) return;

      if (isTypingTarget(e.target)) {
        const allowed =
          allowRef.current.includes(combo) || allowRef.current.includes(bare);
        if (!allowed) return;
      }

      handler(e);
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [enabled]);
}
