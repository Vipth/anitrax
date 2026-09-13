import { create } from "zustand";

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface Toast {
  id: number;
  title: string;
  description?: string;
  tone: "info" | "success" | "error";
  action?: ToastAction;
}

interface ToastState {
  toasts: Toast[];
  push: (t: Omit<Toast, "id">) => void;
  dismiss: (id: number) => void;
}

// Actionable toasts stay up long enough to actually notice and click.
const AUTO_DISMISS_MS = 5000;
const ACTION_DISMISS_MS = 20_000;

export const useToasts = create<ToastState>((set) => ({
  toasts: [],
  push: (t) => {
    const id = Date.now() + Math.random();
    set((s) => ({ toasts: [...s.toasts, { ...t, id }] }));
    setTimeout(
      () => {
        set((s) => ({ toasts: s.toasts.filter((x) => x.id !== id) }));
      },
      t.action ? ACTION_DISMISS_MS : AUTO_DISMISS_MS,
    );
  },
  dismiss: (id) => set((s) => ({ toasts: s.toasts.filter((x) => x.id !== id) })),
}));

export const toast = {
  info: (title: string, description?: string) =>
    useToasts.getState().push({ title, description, tone: "info" }),
  success: (title: string, description?: string) =>
    useToasts.getState().push({ title, description, tone: "success" }),
  error: (title: string, description?: string) =>
    useToasts.getState().push({ title, description, tone: "error" }),
  /** An info toast with a single action button (e.g. "Bump progress"). */
  prompt: (title: string, description: string | undefined, action: ToastAction) =>
    useToasts.getState().push({ title, description, tone: "info", action }),
};
