# Jade — a dark UI theme

Polished jade-stone dark: a near-black ground with a green cast, a bright
cut-jade primary, and mint-white ink. Dark only, by design — jade is a jewel,
and jewels are lit against dark.

Status colours deliberately step off the green (a gold warning, a coral danger)
so nothing important gets lost in the foliage. Built as the same semantic token
set as the other themes, so it drops into Tailwind v4 / v3 / plain CSS unchanged.

---

## Tokens

Values are `H S% L%` (no `hsl()` wrapper) so they compose with alpha:
`hsl(var(--primary) / 0.12)`. Hex is the approximate rendered colour.

| Token | Role | HSL | Hex |
|---|---|---|---|
| `--background` | app canvas | `162 18% 8%` | `#111816` |
| `--surface` | sunken areas, inputs | `162 15% 11%` | `#17211E` |
| `--surface-raised` | cards, sheets, popovers | `160 14% 14%` | `#1F2925` |
| `--border` | hairlines, dividers | `160 12% 22%` | `#313F3A` |
| `--border-strong` | focus outlines, scrollbars | `158 12% 31%` | `#465952` |
| `--foreground` | primary text | `150 14% 93%` | `#EBF0ED` |
| `--muted` | secondary text | `155 9% 60%` | `#90A29B` |
| `--muted-foreground` | tertiary text on surfaces | `152 12% 69%` | `#A6B9B1` |
| `--primary` | buttons, links, active state | `154 56% 51%` | `#3CC88B` |
| `--primary-foreground` | text on `--primary` | `160 40% 9%` | `#0E201A` |
| `--accent` | subtle highlight fills | `160 28% 20%` | `#254138` |
| `--accent-foreground` | text on `--accent` | `152 48% 82%` | `#BBE7D3` |
| `--success` | positive status | `128 46% 55%` | `#57C165` |
| `--warning` | caution status | `40 88% 62%` | `#F3BB49` |
| `--danger` | destructive / error | `356 68% 64%` | `#E2656D` |
| `--ring` | focus ring | `154 56% 51%` | `#3CC88B` |
| `--radius` | base corner radius | `0.75rem` | — |

---

## Drop-in CSS

### As a switchable theme

```css
.jade {
  --background: 162 18% 8%;
  --surface: 162 15% 11%;
  --surface-raised: 160 14% 14%;
  --border: 160 12% 22%;
  --border-strong: 158 12% 31%;
  --foreground: 150 14% 93%;
  --muted: 155 9% 60%;
  --muted-foreground: 152 12% 69%;
  --primary: 154 56% 51%;
  --primary-foreground: 160 40% 9%;
  --accent: 160 28% 20%;
  --accent-foreground: 152 48% 82%;
  --success: 128 46% 55%;
  --warning: 40 88% 62%;
  --danger: 356 68% 64%;
  --ring: 154 56% 51%;
  --radius: 0.75rem;
}
```

Activate with `class="jade"` on `<html>`. Because it's a dark theme, add it to
your `dark:` variant selector so dark-only utilities respond:

```css
/* Tailwind v4 */
@custom-variant dark (&:where(.dark, .dark *, .jade, .jade *));
```

### As the only theme

```css
:root {
  color-scheme: dark;
  /* …paste the .jade tokens here… */
}
```

---

## Tailwind / plain CSS

Identical wiring to any token-based theme — map the vars once and design against
`bg-surface`, `text-foreground`, `border-border`, `bg-primary/10`, etc. See
`docs/cappuccino-theme.md` → "Tailwind v4 / v3 / Plain CSS" for the exact
`@theme inline` block and `tailwind.config.js` colours; nothing changes but the
token values above.

Runtime switching + the pre-paint bootstrap script are the same too — just add
`"jade"` to the theme list and treat it like any other class-based theme:

```ts
root.classList.remove("dark", ...ALL_CUSTOM_THEME_CLASSES);
if (theme === "jade") { root.classList.add("jade"); return; }
```

---

## Token semantics

Design against roles, never raw colours (that's what lets one palette swap for
another with zero component edits):

| Group | Use it for |
|---|---|
| `background` | the page itself, nothing else |
| `surface` | recessed things: inputs, wells, code blocks, table headers |
| `surface-raised` | floating things: cards, modals, dropdowns, toasts |
| `border` / `border-strong` | `border` everywhere normal; `border-strong` only where a line must be seen |
| `foreground` / `muted` / `muted-foreground` | body text / supporting text / supporting text on a raised surface |
| `primary` | the one action to take on a screen; active nav; links |
| `accent` | non-interactive tinted fills — selected row, highlighted date, info callout. Pair with `accent-foreground` |
| `success` / `warning` / `danger` | status only; `danger` doubles as the destructive-button colour |
| `ring` | `:focus-visible` outlines, always 2px |

---

## Accessibility

Approximate contrast ratios (WCAG 2.1):

| Pair | Ratio |
|---|---|
| `foreground` on `background` | ~14 : 1 |
| `muted` on `background` | ~5.6 : 1 |
| `primary-foreground` on `primary` | ~6.5 : 1 |
| `foreground` on `surface-raised` | ~12 : 1 |

Body and UI text clear **AA** comfortably (mostly **AAA**). Because so much of the
palette is green, never lean on the jade `primary` alone to carry meaning — pair
every status with an icon or label, and keep destructive actions on `danger`
(coral) so red/green colour-blind users still read them as different.

---

## Design notes

- **The green is in the neutrals, not just the accent.** Background through
  border sit at 158–162°, ~12–18% saturation — enough that the greys read as
  "jade" rather than "slate", not so much that they buzz.
- **`primary` is a cut-jade highlight, not the stone colour.** Raw jade
  (`#00A86B`, ~33% lightness) would sink into the dark surfaces; the theme
  reaches for a brighter, slightly desaturated `51%` so buttons and links lift.
- **Warning and danger are the only warm notes** and they're chosen to be
  maximally distinct from the green — gold and coral both pop hard against jade,
  which is exactly what you want a status colour to do.
- **Nothing is pure black.** Darkest surface is `#111816`; there's always a
  little green in it.
