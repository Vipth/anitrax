# Cappuccino — a warm UI theme

A cozy coffee-house palette in a light and a dark roast. Steamed-milk grounds,
espresso ink, caramel accents. Built as a set of **semantic design tokens** (HSL
channel triplets) so it drops into Tailwind v4, Tailwind v3, or plain CSS without
touching component code.

- **Cappuccino** — light. Cream background, dark-roast text.
- **Cappuccino Dark** — the same roast after hours. Espresso-black background,
  cream text, a brighter caramel crema so accents still lift off the dark.

Both are standalone palettes — not a neutral theme with a brown tint. They share
hues (warm browns 20–35°, one herbal green, one honey, one terracotta) so an app
can switch between them and feel like the same place at a different time of day.

---

## Tokens

Values are `H S% L%` (no `hsl()` wrapper) so they compose with alpha:
`hsl(var(--primary) / 0.1)`. Hex is the approximate rendered colour.

### Cappuccino (light)

| Token | Role | HSL | Hex |
|---|---|---|---|
| `--background` | app canvas | `30 32% 91%` | `#EFE8E1` |
| `--surface` | sunken areas, inputs | `30 30% 88%` | `#EAE0D7` |
| `--surface-raised` | cards, sheets, popovers | `34 44% 96%` | `#F9F5F0` |
| `--border` | hairlines, dividers | `27 24% 79%` | `#D6C8BD` |
| `--border-strong` | focus outlines, scrollbars | `25 22% 66%` | `#BBA595` |
| `--foreground` | primary text | `22 32% 19%` | `#402C21` |
| `--muted` | secondary text | `24 15% 39%` | `#726055` |
| `--muted-foreground` | tertiary text on surfaces | `24 18% 31%` | `#5D4C41` |
| `--primary` | buttons, links, active state | `22 54% 41%` | `#A15A30` |
| `--primary-foreground` | text on `--primary` | `34 46% 96%` | `#FAF5F0` |
| `--accent` | subtle highlight fills | `28 42% 84%` | `#E7D5C5` |
| `--accent-foreground` | text on `--accent` | `22 52% 29%` | `#704024` |
| `--success` | positive status | `130 30% 35%` | `#3E7447` |
| `--warning` | caution status | `34 76% 43%` | `#C1791A` |
| `--danger` | destructive / error | `8 58% 46%` | `#B94331` |
| `--ring` | focus ring | `22 54% 41%` | `#A15A30` |
| `--radius` | base corner radius | `0.75rem` | — |

### Cappuccino Dark

| Token | Role | HSL | Hex |
|---|---|---|---|
| `--background` | app canvas | `22 16% 10%` | `#1E1815` |
| `--surface` | sunken areas, inputs | `24 15% 13%` | `#26201C` |
| `--surface-raised` | cards, sheets, popovers | `24 14% 16%` | `#2F2823` |
| `--border` | hairlines, dividers | `24 12% 24%` | `#453C36` |
| `--border-strong` | focus outlines, scrollbars | `24 11% 32%` | `#5B5049` |
| `--foreground` | primary text | `32 28% 90%` | `#EDE6DE` |
| `--muted` | secondary text | `28 12% 62%` | `#AA9D92` |
| `--muted-foreground` | tertiary text on surfaces | `30 15% 70%` | `#BEB3A7` |
| `--primary` | buttons, links, active state | `28 62% 62%` | `#DA9A62` |
| `--primary-foreground` | text on `--primary` | `24 30% 12%` | `#281D15` |
| `--accent` | subtle highlight fills | `26 28% 24%` | `#4E3B2C` |
| `--accent-foreground` | text on `--accent` | `30 55% 82%` | `#EAD1B8` |
| `--success` | positive status | `140 34% 56%` | `#69B582` |
| `--warning` | caution status | `36 78% 62%` | `#EAAD53` |
| `--danger` | destructive / error | `8 66% 64%` | `#E07767` |
| `--ring` | focus ring | `28 62% 62%` | `#DA9A62` |

---

## Drop-in CSS

### As switchable themes (class-based)

Put both blocks in your global stylesheet. Add `class="cappuccino"` or
`class="cappuccino-dark"` to `<html>` to activate.

```css
.cappuccino {
  --background: 30 32% 91%;
  --surface: 30 30% 88%;
  --surface-raised: 34 44% 96%;
  --border: 27 24% 79%;
  --border-strong: 25 22% 66%;
  --foreground: 22 32% 19%;
  --muted: 24 15% 39%;
  --muted-foreground: 24 18% 31%;
  --primary: 22 54% 41%;
  --primary-foreground: 34 46% 96%;
  --accent: 28 42% 84%;
  --accent-foreground: 22 52% 29%;
  --success: 130 30% 35%;
  --warning: 34 76% 43%;
  --danger: 8 58% 46%;
  --ring: 22 54% 41%;
  --radius: 0.75rem;
}

.cappuccino-dark {
  --background: 22 16% 10%;
  --surface: 24 15% 13%;
  --surface-raised: 24 14% 16%;
  --border: 24 12% 24%;
  --border-strong: 24 11% 32%;
  --foreground: 32 28% 90%;
  --muted: 28 12% 62%;
  --muted-foreground: 30 15% 70%;
  --primary: 28 62% 62%;
  --primary-foreground: 24 30% 12%;
  --accent: 26 28% 24%;
  --accent-foreground: 30 55% 82%;
  --success: 140 34% 56%;
  --warning: 36 78% 62%;
  --danger: 8 66% 64%;
  --ring: 28 62% 62%;
  --radius: 0.75rem;
}
```

### As the only theme (auto light/dark)

If Cappuccino *is* the product's identity, skip the classes and let the OS decide:

```css
:root {
  /* …paste the .cappuccino tokens here… */
}
@media (prefers-color-scheme: dark) {
  :root {
    /* …paste the .cappuccino-dark tokens here… */
  }
}
```

---

## Tailwind v4

Tailwind v4 has no config file — map the tokens to colour utilities in CSS:

```css
@import "tailwindcss";

/* Make `dark:` utilities respond to the dark roast too */
@custom-variant dark (&:where(.dark, .dark *, .cappuccino-dark, .cappuccino-dark *));

/* …the .cappuccino / .cappuccino-dark blocks from above… */

@theme inline {
  --color-background: hsl(var(--background));
  --color-surface: hsl(var(--surface));
  --color-surface-raised: hsl(var(--surface-raised));
  --color-border: hsl(var(--border));
  --color-border-strong: hsl(var(--border-strong));
  --color-foreground: hsl(var(--foreground));
  --color-muted: hsl(var(--muted));
  --color-muted-foreground: hsl(var(--muted-foreground));
  --color-primary: hsl(var(--primary));
  --color-primary-foreground: hsl(var(--primary-foreground));
  --color-accent: hsl(var(--accent));
  --color-accent-foreground: hsl(var(--accent-foreground));
  --color-success: hsl(var(--success));
  --color-warning: hsl(var(--warning));
  --color-danger: hsl(var(--danger));
  --color-ring: hsl(var(--ring));

  --radius-lg: var(--radius);
  --radius-md: calc(var(--radius) - 0.25rem);
  --radius-sm: calc(var(--radius) - 0.4rem);
}
```

Now `bg-surface`, `text-foreground`, `border-border`, `ring-ring`,
`bg-primary/10`, etc. all work and re-resolve per theme.

## Tailwind v3

```js
// tailwind.config.js
const withOpacity = (v) => `hsl(var(${v}) / <alpha-value>)`;

module.exports = {
  darkMode: ["class", '[class~="cappuccino-dark"]'],
  theme: {
    extend: {
      colors: {
        background: withOpacity("--background"),
        surface: withOpacity("--surface"),
        "surface-raised": withOpacity("--surface-raised"),
        border: withOpacity("--border"),
        "border-strong": withOpacity("--border-strong"),
        foreground: withOpacity("--foreground"),
        muted: withOpacity("--muted"),
        "muted-foreground": withOpacity("--muted-foreground"),
        primary: withOpacity("--primary"),
        "primary-foreground": withOpacity("--primary-foreground"),
        accent: withOpacity("--accent"),
        "accent-foreground": withOpacity("--accent-foreground"),
        success: withOpacity("--success"),
        warning: withOpacity("--warning"),
        danger: withOpacity("--danger"),
        ring: withOpacity("--ring"),
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 0.25rem)",
        sm: "calc(var(--radius) - 0.4rem)",
      },
    },
  },
};
```

## Plain CSS

```css
body {
  background: hsl(var(--background));
  color: hsl(var(--foreground));
}
.card {
  background: hsl(var(--surface-raised));
  border: 1px solid hsl(var(--border));
  border-radius: var(--radius);
}
.button-primary {
  background: hsl(var(--primary));
  color: hsl(var(--primary-foreground));
}
.button-primary:focus-visible {
  outline: 2px solid hsl(var(--ring));
}
.tag-tentative {
  background: hsl(var(--primary) / 0.12);
  color: hsl(var(--primary));
}
```

---

## Switching themes at runtime

Store the choice, apply a class to `<html>`, and apply it again *before first
paint* so there's no flash.

```ts
type Theme =
  | "light"
  | "dark"
  | "cappuccino"
  | "cappuccino-dark"
  | "system";

export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.classList.remove("dark", "cappuccino", "cappuccino-dark");
  if (theme === "cappuccino" || theme === "cappuccino-dark") {
    root.classList.add(theme);
    return;
  }
  const dark =
    theme === "dark" ||
    (theme === "system" &&
      matchMedia("(prefers-color-scheme: dark)").matches);
  if (dark) root.classList.add("dark");
}
```

Pre-paint bootstrap (inline in `<head>`, before your stylesheet):

```html
<script>
  try {
    var t = localStorage.getItem("theme") || "system";
    var el = document.documentElement;
    el.classList.remove("dark", "cappuccino", "cappuccino-dark");
    if (t === "cappuccino" || t === "cappuccino-dark") el.classList.add(t);
    else if (t === "dark" || (t === "system" &&
      matchMedia("(prefers-color-scheme: dark)").matches)) el.classList.add("dark");
  } catch (e) {}
</script>
```

---

## Token semantics

Design against roles, never raw colours — that's what makes one palette swap for
another with zero component edits.

| Group | Use it for |
|---|---|
| `background` | the page itself, nothing else |
| `surface` | things that sit *in* the page and read as recessed: text inputs, wells, code blocks, table headers |
| `surface-raised` | things that sit *above* the page: cards, modals, dropdowns, toasts |
| `border` / `border-strong` | `border` for every normal divider; `border-strong` only where a line must be seen (focus, resize handles, scrollbar thumb) |
| `foreground` | body text and headings |
| `muted` | supporting text — captions, metadata, placeholder-adjacent labels |
| `muted-foreground` | the same idea but a notch stronger, for text sitting on `surface`/`surface-raised` where `muted` would be too faint |
| `primary` | the one action you want taken on a screen; active nav; links |
| `accent` | tinted backgrounds that aren't interactive — a selected row, a highlighted date, an info callout. Pair with `accent-foreground` |
| `success` / `warning` / `danger` | status only. `danger` doubles as the destructive-button colour |
| `ring` | `:focus-visible` outlines, always 2px |

Common alpha derivations: `hsl(var(--primary) / 0.1)` for a primary-tinted chip,
`hsl(var(--foreground) / 0.06)` for a hover wash, `hsl(var(--danger) / 0.15)` for
an error banner fill.

---

## Accessibility

Contrast ratios (approximate, WCAG 2.1):

| Pair | Light | Dark |
|---|---|---|
| `foreground` on `background` | ~10.5 : 1 | ~13 : 1 |
| `muted` on `background` | ~4.9 : 1 | ~5.2 : 1 |
| `primary-foreground` on `primary` | ~5.4 : 1 | ~7.4 : 1 |
| `foreground` on `surface-raised` | ~11 : 1 | ~11.5 : 1 |

Body text and UI text clear **AA** (and mostly **AAA**) in both roasts. `muted` on
`background` is above 4.5 : 1 — fine for text, but keep true fine print at
`foreground`. The caramel `primary` is a hue many forms of colour-blindness still
read as distinct from the browns around it; still, don't rely on it alone —
pair every status colour with an icon or label.

---

## Design notes

- **One warm hue family.** Everything structural lives in 20–35°. The green,
  honey and terracotta are the *only* excursions and they're all still warm-leaning
  and desaturated, so status colours read as part of the set rather than stickers.
- **The dark roast lightens the accent, not just the surfaces.** `primary` goes
  from a deep `41%` lightness to `62%` — a mid-brown button that works on cream
  would turn to mud on espresso, so the dark palette reaches for the crema instead.
- **Nothing is pure black or white.** Darkest text is `#402C21`, lightest surface
  `#F9F5F0`. The near-absence of true neutrals is what sells "coffee" over "beige".
- **`surface` is darker than `background` in the light theme** (unusual — most
  light themes raise surfaces toward white). Cappuccino sinks inputs and wells
  into the crema and floats only cards up to `surface-raised`. It reads warmer.
