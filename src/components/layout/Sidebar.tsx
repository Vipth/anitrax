import { Link } from "@tanstack/react-router";
import {
  Compass,
  Keyboard,
  Library,
  Settings as SettingsIcon,
  Sparkles,
} from "lucide-react";
import { ThemeSelect } from "./ThemeSelect";
import { RequestBudgetMeter } from "@/components/RequestBudgetMeter";
import { Kbd } from "@/components/ui/kbd";
import { useUi } from "@/stores/ui";

const NAV = [
  { to: "/", label: "Library", icon: Library, exact: true },
  { to: "/discover", label: "Discover", icon: Compass, exact: false },
  { to: "/settings", label: "Settings", icon: SettingsIcon, exact: false },
] as const;

export function Sidebar() {
  const openHelp = useUi((s) => s.setHelpOpen);
  const themeMenuOpen = useUi((s) => s.themeMenuOpen);
  const setThemeMenuOpen = useUi((s) => s.setThemeMenuOpen);
  return (
    <aside className="flex h-full w-56 shrink-0 flex-col border-r border-border bg-surface">
      <div className="flex items-center gap-2 px-4 py-4">
        <div className="grid size-8 place-items-center rounded-lg bg-primary text-primary-foreground">
          <Sparkles className="size-4" />
        </div>
        <span className="text-sm font-semibold">AniTrax</span>
      </div>

      <nav className="flex-1 space-y-1 px-3 py-2">
        {NAV.map(({ to, label, icon: Icon, exact }) => (
          <Link
            key={to}
            to={to}
            activeOptions={{ exact }}
            className="flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors"
            activeProps={{ className: "bg-primary/10 text-primary" }}
            inactiveProps={{
              className:
                "text-muted-foreground hover:bg-border/40 hover:text-foreground",
            }}
          >
            <Icon className="size-4" />
            {label}
          </Link>
        ))}
      </nav>

      <div className="space-y-3 border-t border-border p-3">
        <RequestBudgetMeter />

        <button
          onClick={() => openHelp(true)}
          className="flex w-full items-center justify-between rounded-md px-2 py-1.5 text-[11px] text-muted-foreground transition-colors hover:bg-border/40 hover:text-foreground"
        >
          <span className="flex items-center gap-1.5">
            <Keyboard className="size-3.5" />
            Keyboard shortcuts
          </span>
          <Kbd className="h-[18px] min-w-[18px]">?</Kbd>
        </button>

        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] text-muted-foreground">Theme</span>
          <ThemeSelect
            variant="compact"
            open={themeMenuOpen}
            onOpenChange={setThemeMenuOpen}
          />
        </div>
      </div>
    </aside>
  );
}
