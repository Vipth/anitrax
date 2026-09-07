import { CheckCircle2, Info, X, XCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import { useToasts } from "@/stores/toast";

const ICON = {
  info: Info,
  success: CheckCircle2,
  error: XCircle,
};

export function Toaster() {
  const { toasts, dismiss } = useToasts();
  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-[100] flex w-80 flex-col gap-2">
      {toasts.map((t) => {
        const Icon = ICON[t.tone];
        return (
          <div
            key={t.id}
            className={cn(
              "pointer-events-auto flex items-start gap-2.5 rounded-lg border bg-surface-raised p-3 shadow-lg",
              t.tone === "error"
                ? "border-danger/40"
                : t.tone === "success"
                  ? "border-success/40"
                  : "border-border",
            )}
          >
            <Icon
              className={cn(
                "mt-0.5 size-4 shrink-0",
                t.tone === "error"
                  ? "text-danger"
                  : t.tone === "success"
                    ? "text-success"
                    : "text-muted-foreground",
              )}
            />
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium">{t.title}</p>
              {t.description && (
                <p className="mt-0.5 text-xs text-muted-foreground">
                  {t.description}
                </p>
              )}
            </div>
            <button
              onClick={() => dismiss(t.id)}
              className="text-muted-foreground hover:text-foreground"
            >
              <X className="size-3.5" />
            </button>
          </div>
        );
      })}
    </div>
  );
}
