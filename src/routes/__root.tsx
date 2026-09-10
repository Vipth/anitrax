import { useEffect } from "react";
import {
  createRootRoute,
  Outlet,
  useNavigate,
  useRouter,
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
  const helpOpen = useUi((s) => s.helpOpen);
  const toggleHelp = useUi((s) => s.toggleHelp);
  const setHelpOpen = useUi((s) => s.setHelpOpen);
  const toggleThemeMenu = useUi((s) => s.toggleThemeMenu);
  const themeMenuOpen = useUi((s) => s.themeMenuOpen);
  const gridNavEngaged = useUi((s) => s.gridNavEngaged);
  const router = useRouter();
  const isNavigating = useRouterState({ select: (s) => s.status === "pending" });

  const SECTIONS = [
    "/",
    "/discover",
    "/seasons",
    "/library-local",
    "/rss",
    "/stats",
    "/settings",
  ];
  const goSection = (dir: -1 | 1) => {
    // A menu, listbox, or dialog owns the arrow keys while it's open.
    if (document.querySelector('[role="listbox"],[role="menu"],[role="dialog"]'))
      return;
    const i = SECTIONS.indexOf(router.state.location.pathname);
    const from = i === -1 ? (dir === 1 ? -1 : 0) : i;
    navigate({ to: SECTIONS[(from + dir + SECTIONS.length) % SECTIONS.length] });
  };

  useHotkeys(
    {
      "?": (e) => {
        e.preventDefault();
        toggleHelp();
      },
      Escape: () => setHelpOpen(false),
      // Everything below is suppressed while the help dialog is open.
      t: (e) => {
        if (helpOpen) return;
        e.preventDefault();
        toggleThemeMenu();
      },
      "1": (e) => {
        if (helpOpen) return;
        e.preventDefault();
        navigate({ to: "/" });
      },
      "2": (e) => {
        if (helpOpen) return;
        e.preventDefault();
        navigate({ to: "/discover" });
      },
      "3": (e) => {
        if (helpOpen) return;
        e.preventDefault();
        navigate({ to: "/seasons" });
      },
      "4": (e) => {
        if (helpOpen) return;
        e.preventDefault();
        navigate({ to: "/library-local" });
      },
      "5": (e) => {
        if (helpOpen) return;
        e.preventDefault();
        navigate({ to: "/rss" });
      },
      "6": (e) => {
        if (helpOpen) return;
        e.preventDefault();
        navigate({ to: "/stats" });
      },
      "7": (e) => {
        if (helpOpen) return;
        e.preventDefault();
        navigate({ to: "/settings" });
      },
      // Arrows cycle the sidebar sections — unless the theme menu is open or the
      // library grid has keyboard focus, which claim the arrows for themselves.
      ArrowUp: (e) => {
        if (helpOpen || themeMenuOpen || gridNavEngaged) return;
        e.preventDefault();
        goSection(-1);
      },
      ArrowDown: (e) => {
        if (helpOpen || themeMenuOpen || gridNavEngaged) return;
        e.preventDefault();
        goSection(1);
      },
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
