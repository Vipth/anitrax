import { useEffect } from "react";
import {
  createRootRoute,
  Outlet,
  useNavigate,
  useRouterState,
} from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { CloudOff } from "lucide-react";
import { Sidebar } from "@/components/layout/Sidebar";
import { Toaster } from "@/components/ui/toaster";
import { KeyboardHelp } from "@/components/KeyboardHelp";
import { useBackendEvents, useBudget } from "@/lib/hooks";
import { useHotkeys } from "@/lib/hotkeys";
import { toast } from "@/stores/toast";
import { errorMessage } from "@/lib/types";
import { usePrefs, applyTheme } from "@/stores/prefs";
import { useUi } from "@/stores/ui";

export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  useBackendEvents();
  const theme = usePrefs((s) => s.theme);
  const navigate = useNavigate();
  const toggleHelp = useUi((s) => s.toggleHelp);
  const setHelpOpen = useUi((s) => s.setHelpOpen);
  const isNavigating = useRouterState({ select: (s) => s.status === "pending" });

  useHotkeys(
    {
      "?": (e) => {
        e.preventDefault();
        toggleHelp();
      },
      "1": (e) => {
        e.preventDefault();
        navigate({ to: "/" });
      },
      "2": (e) => {
        e.preventDefault();
        navigate({ to: "/discover" });
      },
      "3": (e) => {
        e.preventDefault();
        navigate({ to: "/settings" });
      },
      Escape: () => setHelpOpen(false),
    },
    { allowInInput: ["Escape"] },
  );

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
      listen<unknown>("auth-error", (e) =>
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
        <OutageBanner />
        <Outlet />
      </main>
      <Toaster />
      <KeyboardHelp />
    </div>
  );
}

function OutageBanner() {
  const { data } = useBudget();
  if (!data?.serviceDown) return null;
  return (
    <div className="flex items-center gap-2 border-b border-warning/40 bg-warning/10 px-6 py-2 text-xs text-warning">
      <CloudOff className="size-3.5 shrink-0" />
      <span>
        AniList&apos;s API is currently down on their end — not your setup. Your
        cached library still works, and any edits you make will sync
        automatically once it&apos;s back.
      </span>
    </div>
  );
}
