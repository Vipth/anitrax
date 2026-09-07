import { Fragment } from "react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectSeparator,
  SelectTrigger,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { usePrefs, applyTheme, type Theme } from "@/stores/prefs";

/** Pinned to the top, in this order. */
const BASE_THEMES: { value: Theme; label: string }[] = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
];

/** Standalone palettes — sorted alphabetically, so new ones slot in on their own. */
const PALETTE_THEMES: { value: Theme; label: string }[] = [
  { value: "jade", label: "Jade" },
  { value: "nord", label: "Nord" },
  { value: "kanagawa", label: "Kanagawa" },
  { value: "catppuccin-latte", label: "Catppuccin Latte" },
  { value: "catppuccin-frappe", label: "Catppuccin Frappé" },
  { value: "catppuccin-macchiato", label: "Catppuccin Macchiato" },
  { value: "catppuccin-mocha", label: "Catppuccin Mocha" },
];
PALETTE_THEMES.sort((a, b) => a.label.localeCompare(b.label));

const THEMES = [...BASE_THEMES, ...PALETTE_THEMES];

/** [background, primary] preview colours for each theme's swatch. */
const SWATCH: Record<Theme, [string, string]> = {
  system: ["#f4f4f6", "#17171f"],
  light: ["#ffffff", "#7c5cff"],
  dark: ["#141420", "#a78bfa"],
  jade: ["#111816", "#3cc88b"],
  nord: ["#2e3440", "#88c0d0"],
  kanagawa: ["#1f1f28", "#7e9cd8"],
  "catppuccin-latte": ["#e6e9ef", "#d20f39"],
  "catppuccin-frappe": ["#303446", "#e78284"],
  "catppuccin-macchiato": ["#24273a", "#ed8796"],
  "catppuccin-mocha": ["#1e1e2e", "#f38ba8"],
};

function Swatch({ theme, className }: { theme: Theme; className?: string }) {
  const [bg, primary] = SWATCH[theme];
  return (
    <span
      className={cn(
        "size-4 shrink-0 rounded-[5px] shadow-sm ring-1 ring-inset ring-black/15",
        className,
      )}
      style={{
        background: `linear-gradient(135deg, ${bg} 0 52%, ${primary} 52% 100%)`,
      }}
    />
  );
}

export function ThemeSelect({
  variant = "full",
  className,
  open,
  onOpenChange,
}: {
  variant?: "full" | "compact";
  className?: string;
  /** Optional controlled open state (used so the `T` hotkey can pop it open). */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}) {
  const theme = usePrefs((s) => s.theme);
  const setTheme = usePrefs((s) => s.setTheme);
  const current = THEMES.find((t) => t.value === theme) ?? THEMES[0];

  return (
    <Select
      open={open}
      onOpenChange={onOpenChange}
      value={theme}
      onValueChange={(v) => {
        const t = v as Theme;
        setTheme(t);
        applyTheme(t);
      }}
    >
      <SelectTrigger
        aria-label="Theme"
        title={variant === "compact" ? `Theme: ${current.label}` : undefined}
        className={cn(
          variant === "compact" ? "w-auto px-2" : "min-w-[12rem]",
          className,
        )}
      >
        <span className="flex min-w-0 items-center gap-2">
          <Swatch theme={theme} />
          {variant === "full" && (
            <span className="truncate">{current.label}</span>
          )}
        </span>
      </SelectTrigger>

      <SelectContent align={variant === "compact" ? "end" : "start"}>
        {THEMES.map((t, i) => (
          <Fragment key={t.value}>
            {i === BASE_THEMES.length && <SelectSeparator />}
            <SelectItem value={t.value}>
              <span className="flex items-center gap-2.5">
                <Swatch theme={t.value} />
                {t.label}
              </span>
            </SelectItem>
          </Fragment>
        ))}
      </SelectContent>
    </Select>
  );
}
