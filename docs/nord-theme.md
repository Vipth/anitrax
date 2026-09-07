# Nord — a dark UI theme

[Nord](https://www.nordtheme.com/) is an arctic, north-bluish palette by Arctic
Ice Studio (16 fixed colours in four groups: Polar Night, Snow Storm, Frost,
Aurora). This is Nord mapped onto our semantic token set so it drops into
Tailwind v4 / v3 / plain CSS unchanged.

Dark only — Nord is a dark theme by convention. Colours are used faithfully:
dimmed Polar Night for surfaces, Snow Storm for text, the Frost cyan (`nord8`)
as primary, and Aurora green/yellow/red for status.

---

## Tokens

Values are `H S% L%` (no `hsl()` wrapper). Hex is the approximate rendered colour;
the "Nord" column names the source swatch.

| Token | Role | HSL | Hex | Nord |
|---|---|---|---|---|
| `--background` | app canvas | `220 16% 20%` | `#2A2E38` | ~`nord0` |
| `--surface` | sunken areas, inputs | `221 16% 25%` | `#353B49` | nord0–1 |
| `--surface-raised` | cards, sheets, popovers | `222 16% 28%` | `#3B4252` | `nord1` |
| `--border` | hairlines, dividers | `220 15% 34%` | `#49505F` | `nord2`/`nord3` |
| `--border-strong` | focus outlines, scrollbars | `220 14% 42%` | `#5B6478` | `nord3`+ |
| `--foreground` | primary text | `218 27% 92%` | `#ECEFF4` | `nord6` |
| `--muted` | secondary text | `220 14% 66%` | `#9BA3B4` | between `nord4`/`nord3` |
| `--muted-foreground` | tertiary text on surfaces | `219 18% 76%` | `#B4BCCB` | ~`nord4` |
| `--primary` | buttons, links, active state | `193 43% 67%` | `#88C0D0` | `nord8` |
| `--primary-foreground` | text on `--primary` | `220 28% 15%` | `#1B2233` | `nord0`− |
| `--accent` | subtle highlight fills | `217 24% 32%` | `#3E4A63` | ~`nord10` dark |
| `--accent-foreground` | text on `--accent` | `193 43% 84%` | `#C1DEE6` | `nord8`+ |
| `--success` | positive status | `92 28% 65%` | `#A3BE8C` | `nord14` |
| `--warning` | caution status | `40 71% 73%` | `#EBCB8B` | `nord13` |
| `--danger` | destructive / error | `354 42% 56%` | `#BF616A` | `nord11` |
| `--ring` | focus ring | `193 43% 67%` | `#88C0D0` | `nord8` |
| `--radius` | base corner radius | `0.75rem` | — | — |

---

## Drop-in CSS

```css
.nord {
  --background: 220 16% 20%;
  --surface: 221 16% 25%;
  --surface-raised: 222 16% 28%;
  --border: 220 15% 34%;
  --border-strong: 220 14% 42%;
  --foreground: 218 27% 92%;
  --muted: 220 14% 66%;
  --muted-foreground: 219 18% 76%;
  --primary: 193 43% 67%;
  --primary-foreground: 220 28% 15%;
  --accent: 217 24% 32%;
  --accent-foreground: 193 43% 84%;
  --success: 92 28% 65%;
  --warning: 40 71% 73%;
  --danger: 354 42% 56%;
  --ring: 193 43% 67%;
  --radius: 0.75rem;
}
```

Activate with `class="nord"` on `<html>`. It's a dark theme, so add it to the
`dark:` variant selector:

```css
/* Tailwind v4 */
@custom-variant dark (&:where(.dark, .dark *, .nord, .nord *));
```

For the raw Nord swatches (nord0–nord15) or a light variant, see
<https://www.nordtheme.com/docs/colors-and-palettes>.

---

## Tailwind / plain CSS / runtime switching

Identical wiring to any token-based theme — see `docs/cappuccino-theme.md` for
the `@theme inline` block, the `tailwind.config.js` colours, and the pre-paint
bootstrap script. Only the token values above change. Add `"nord"` to the theme
list and treat it like any other class-based theme:

```ts
root.classList.remove("dark", ...ALL_CUSTOM_THEME_CLASSES);
if (theme === "nord") { root.classList.add("nord"); return; }
```

---

## Token semantics

| Group | Use it for |
|---|---|
| `background` | the page itself, nothing else |
| `surface` | recessed things: inputs, wells, code blocks, table headers |
| `surface-raised` | floating things: cards, modals, dropdowns, toasts |
| `border` / `border-strong` | `border` everywhere normal; `border-strong` only where a line must be seen |
| `foreground` / `muted` / `muted-foreground` | body text / supporting text / supporting text on a raised surface |
| `primary` | the one action to take on a screen; active nav; links |
| `accent` | non-interactive tinted fills. Pair with `accent-foreground` |
| `success` / `warning` / `danger` | status only; `danger` doubles as the destructive-button colour |
| `ring` | `:focus-visible` outlines, always 2px |

---

## Accessibility

Approximate contrast ratios (WCAG 2.1):

| Pair | Ratio |
|---|---|
| `foreground` on `background` | ~11 : 1 |
| `muted` on `background` | ~4.9 : 1 |
| `primary-foreground` on `primary` | ~8 : 1 |
| `foreground` on `surface-raised` | ~9 : 1 |

Body and UI text clear **AA**. `muted` on `background` sits just above 4.5 : 1 —
fine for text, but put true fine print at `foreground`. Nord's Frost primary and
Aurora status colours are all fairly close in lightness, so as always pair every
status with an icon or label rather than relying on hue alone.

---

## Design notes

- **This is Nord dimmed the way editors ship it** — `nord0` as the canvas,
  `nord1` as raised UI (Nord goes lighter for elevation, unlike a
  cappuccino-style palette). `surface` sits just under `nord0` for inputs.
- **`primary` is `nord8`, the signature Frost cyan** — the colour most people
  picture when they hear "Nord". Buttons put a near-`nord0` dark text on it.
- **Status colours are straight Aurora** — `nord14` green, `nord13` yellow,
  `nord11` red — desaturated and muted, so they sit calmly rather than shouting.
- **Everything is bluish.** Even the greys carry ~220° hue; that cool cast is
  the whole point of Nord.
