import { Link, useRouterState } from "@tanstack/react-router";
import {
  Compass,
  Library,
  Settings as SettingsIcon,
  Sparkles,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { ThemeToggle } from "./ThemeToggle";
import { RequestBudgetMeter } from "@/components/RequestBudgetMeter";

const NAV = [
  { to: "/", label: "Library", icon: Library, exact: true },
  { to: "/discover", label: "Discover", icon: Compass, exact: false },
  { to: "/settings", label: "Settings", icon: SettingsIcon, exact: false },
] as const;

export function Sidebar() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  return (
    <aside className="flex h-full w-56 shrink-0 flex-col border-r border-border bg-surface">
      <div className="flex items-center gap-2 px-4 py-4">
        <div className="grid size-8 place-items-center rounded-lg bg-primary text-primary-foreground">
          <Sparkles className="size-4" />
        </div>
        <span className="text-sm font-semibold">Anime Tracker</span>
      </div>

      <nav className="flex-1 space-y-1 px-3 py-2">
        {NAV.map(({ to, label, icon: Icon, exact }) => {
          const active = exact ? pathname === to : pathname.startsWith(to);
          return (
            <Link
              key={to}
              to={to}
              className={cn(
                "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                active
                  ? "bg-primary/10 text-primary"
                  : "text-muted-foreground hover:bg-border/40 hover:text-foreground",
              )}
            >
              <Icon className="size-4" />
              {label}
            </Link>
          );
        })}
      </nav>

      <div className="space-y-3 border-t border-border p-3">
        <RequestBudgetMeter />
        <div className="flex items-center justify-between">
          <span className="text-[11px] text-muted-foreground">Theme</span>
          <ThemeToggle />
        </div>
      </div>
    </aside>
  );
}
