import { Coffee, Monitor, Moon, Sun } from "lucide-react";
import { Segmented } from "@/components/ui/primitives";
import { usePrefs, applyTheme, type Theme } from "@/stores/prefs";

export function ThemeToggle() {
  const theme = usePrefs((s) => s.theme);
  const setTheme = usePrefs((s) => s.setTheme);

  return (
    <Segmented<Theme>
      value={theme}
      onChange={(t) => {
        setTheme(t);
        applyTheme(t);
      }}
      options={[
        { value: "light", label: <Sun className="size-3.5" /> },
        { value: "dark", label: <Moon className="size-3.5" /> },
        { value: "cappuccino", label: <Coffee className="size-3.5" /> },
        { value: "system", label: <Monitor className="size-3.5" /> },
      ]}
    />
  );
}
