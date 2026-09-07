import { useEffect } from "react";
import {
  createRootRoute,
  Outlet,
  useRouterState,
} from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { Sidebar } from "@/components/layout/Sidebar";
import { Toaster } from "@/components/ui/toaster";
import { useBackendEvents } from "@/lib/hooks";
import { toast } from "@/stores/toast";
import { errorMessage } from "@/lib/types";
import { usePrefs, applyTheme } from "@/stores/prefs";

export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  useBackendEvents();
  const theme = usePrefs((s) => s.theme);
  const isNavigating = useRouterState({ select: (s) => s.status === "pending" });

  useEffect(() => {
    applyTheme(theme);
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => theme === "system" && applyTheme("system");
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [theme]);

  useEffect(() => {
    const unlisten = [
      listen<string>("auth-changed", () =>
        toast.success("Account connected", "Your list is syncing now."),
      ),
      listen<string>("auth-error", (e) =>
        toast.error("Sign-in failed", errorMessage(e.payload)),
      ),
    ];
    return () => {
      unlisten.forEach((p) => p.then((f) => f()));
    };
  }, []);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      <Sidebar />
      <main className="relative flex-1 overflow-y-auto">
        {isNavigating && (
          <div className="absolute inset-x-0 top-0 z-50 h-0.5 animate-pulse bg-primary" />
        )}
        <Outlet />
      </main>
      <Toaster />
    </div>
  );
}
