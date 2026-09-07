import { usePrefs, applyTheme, type Theme } from "@/stores/prefs";

const THEMES: { value: Theme; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "cappuccino", label: "Cappuccino" },
  { value: "cappuccino-dark", label: "Cappuccino Dark" },
];

export function ThemeSelect({ className }: { className?: string }) {
  const theme = usePrefs((s) => s.theme);
  const setTheme = usePrefs((s) => s.setTheme);

  return (
    <select
      aria-label="Theme"
      value={theme}
      onChange={(e) => {
        const t = e.target.value as Theme;
        setTheme(t);
        applyTheme(t);
      }}
      className={
        "h-8 rounded-md border border-border bg-surface px-2 text-xs outline-none focus:border-primary " +
        (className ?? "")
      }
    >
      {THEMES.map((t) => (
        <option key={t.value} value={t.value}>
          {t.label}
        </option>
      ))}
    </select>
  );
}
