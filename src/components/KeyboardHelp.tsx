import * as React from "react";
import { Globe, Keyboard, Library } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useUi } from "@/stores/ui";

interface Shortcut {
  keys: string[];
  label: string;
}

const GROUPS: {
  title: string;
  icon: React.ComponentType<{ className?: string }>;
  items: Shortcut[];
}[] = [
  {
    title: "Global",
    icon: Globe,
    items: [
      { keys: ["?"], label: "Toggle this dialog" },
      { keys: ["1"], label: "Go to Library" },
      { keys: ["2"], label: "Go to Discover" },
      { keys: ["3"], label: "Go to Settings" },
      { keys: ["/"], label: "Focus search / filter" },
      { keys: ["Esc"], label: "Close dialog or panel" },
    ],
  },
  {
    title: "Library",
    icon: Library,
    items: [
      { keys: ["j", "↓"], label: "Next title" },
      { keys: ["k", "↑"], label: "Previous title" },
      { keys: ["h", "←"], label: "Title to the left" },
      { keys: ["l", "→"], label: "Title to the right" },
      { keys: ["Enter"], label: "Open selected title" },
      { keys: ["e"], label: "Edit selected entry" },
      { keys: ["+"], label: "Advance one episode" },
      { keys: ["−"], label: "Back one episode" },
      { keys: ["[", "]"], label: "Switch status tab" },
      { keys: ["r"], label: "Sync now" },
    ],
  },
];

function Keycap({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-grid h-[22px] min-w-[22px] place-items-center rounded-[6px] border border-border-strong/50 bg-surface px-1.5 font-sans text-[11px] font-medium leading-none text-muted-foreground shadow-[inset_0_-1px_0_hsl(var(--border-strong)/0.6)]">
      {children}
    </kbd>
  );
}

function Keys({ keys }: { keys: string[] }) {
  return (
    <span className="flex items-center gap-1">
      {keys.map((k, i) => (
        <React.Fragment key={k}>
          {i > 0 && <span className="text-[10px] text-muted-foreground/50">/</span>}
          <Keycap>{k}</Keycap>
        </React.Fragment>
      ))}
    </span>
  );
}

export function KeyboardHelp() {
  const { helpOpen, setHelpOpen } = useUi();

  return (
    <Dialog open={helpOpen} onOpenChange={setHelpOpen}>
      <DialogContent className="max-w-2xl" showClose={false}>
        <DialogHeader className="flex items-center justify-between">
          <DialogTitle className="flex items-center gap-2.5">
            <span className="grid size-6 place-items-center rounded-md bg-primary/15 text-primary">
              <Keyboard className="size-3.5" />
            </span>
            Keyboard shortcuts
          </DialogTitle>
          <span className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
            close with <Keycap>Esc</Keycap>
          </span>
        </DialogHeader>

        <div className="grid gap-3 p-4 sm:grid-cols-2">
          {GROUPS.map((group) => (
            <div
              key={group.title}
              className="rounded-lg border border-border/70 bg-surface/60 p-3"
            >
              <div className="mb-2 flex items-center gap-1.5 px-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground/70">
                <group.icon className="size-3" />
                {group.title}
              </div>
              <dl>
                {group.items.map((s, i) => (
                  <div
                    key={s.label}
                    className={
                      "flex items-center justify-between gap-4 rounded-md px-1 py-[7px] " +
                      (i > 0 ? "border-t border-border/40" : "")
                    }
                  >
                    <dt className="text-[13px] text-foreground/90">{s.label}</dt>
                    <dd className="shrink-0">
                      <Keys keys={s.keys} />
                    </dd>
                  </div>
                ))}
              </dl>
            </div>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
