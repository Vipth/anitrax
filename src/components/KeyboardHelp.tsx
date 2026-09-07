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

const GROUPS: { title: string; items: Shortcut[] }[] = [
  {
    title: "Global",
    items: [
      { keys: ["?"], label: "Show this help" },
      { keys: ["1"], label: "Go to Library" },
      { keys: ["2"], label: "Go to Discover" },
      { keys: ["3"], label: "Go to Settings" },
      { keys: ["/"], label: "Focus search / filter" },
      { keys: ["Esc"], label: "Close dialog or panel" },
    ],
  },
  {
    title: "Library",
    items: [
      { keys: ["j", "↓"], label: "Next title" },
      { keys: ["k", "↑"], label: "Previous title" },
      { keys: ["h", "←"], label: "Title to the left" },
      { keys: ["l", "→"], label: "Title to the right" },
      { keys: ["Enter"], label: "Open selected title" },
      { keys: ["e"], label: "Edit selected entry" },
      { keys: ["+"], label: "+1 episode" },
      { keys: ["-"], label: "−1 episode" },
      { keys: ["["], label: "Previous status tab" },
      { keys: ["]"], label: "Next status tab" },
      { keys: ["r"], label: "Sync now" },
    ],
  },
];

function Key({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex min-w-[1.5rem] items-center justify-center rounded-md border border-border bg-surface px-1.5 py-0.5 text-[11px] font-medium text-muted-foreground shadow-sm">
      {children}
    </kbd>
  );
}

export function KeyboardHelp() {
  const { helpOpen, setHelpOpen } = useUi();

  return (
    <Dialog open={helpOpen} onOpenChange={setHelpOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Keyboard shortcuts</DialogTitle>
        </DialogHeader>
        <div className="grid gap-x-8 gap-y-5 p-5 sm:grid-cols-2">
          {GROUPS.map((group) => (
            <div key={group.title}>
              <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground/70">
                {group.title}
              </h3>
              <dl className="space-y-1.5">
                {group.items.map((s) => (
                  <div
                    key={s.label}
                    className="flex items-center justify-between gap-4"
                  >
                    <dt className="text-sm text-muted-foreground">{s.label}</dt>
                    <dd className="flex shrink-0 gap-1">
                      {s.keys.map((k) => (
                        <Key key={k}>{k}</Key>
                      ))}
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
