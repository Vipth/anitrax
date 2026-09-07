import { create } from "zustand";

export interface Toast {
  id: number;
  title: string;
  description?: string;
  tone: "info" | "success" | "error";
}

interface ToastState {
  toasts: Toast[];
  push: (t: Omit<Toast, "id">) => void;
  dismiss: (id: number) => void;
}

export const useToasts = create<ToastState>((set) => ({
  toasts: [],
  push: (t) => {
    const id = Date.now() + Math.random();
    set((s) => ({ toasts: [...s.toasts, { ...t, id }] }));
    setTimeout(() => {
      set((s) => ({ toasts: s.toasts.filter((x) => x.id !== id) }));
    }, 5000);
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
};
