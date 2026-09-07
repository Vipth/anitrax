# Catppuccin Mocha — a dark UI theme

[Catppuccin](https://catppuccin.com/) is a widely-ported pastel palette (MIT,
26 named colours per flavour). This is the **Mocha** flavour with the **Red**
accent, mapped onto our semantic token set so it drops into Tailwind v4 / v3 /
plain CSS unchanged. Dark only.

Red is the primary. Because Catppuccin's own error colour is also Red, `danger`
here uses **Maroon** and `warning` uses **Peach**, so status colours stay
distinct from the primary.

---

## Tokens

Values are `H S% L%` (no `hsl()` wrapper). Hex is the source Catppuccin swatch.

| Token | Role | HSL | Catppuccin | Hex |
|---|---|---|---|---|
| `--background` | app canvas | `240 21% 15%` | Base | `#1E1E2E` |
| `--surface` | sunken areas, inputs | `240 21% 12%` | Mantle | `#181825` |
| `--surface-raised` | cards, sheets, popovers | `237 16% 23%` | Surface0 | `#313244` |
| `--border` | hairlines, dividers | `234 13% 31%` | Surface1 | `#45475A` |
| `--border-strong` | focus outlines, scrollbars | `233 12% 39%` | Surface2 | `#585B70` |
| `--foreground` | primary text | `226 64% 88%` | Text | `#CDD6F4` |
| `--muted` | secondary text | `228 24% 72%` | Subtext0 | `#A6ADC8` |
| `--muted-foreground` | tertiary text on surfaces | `227 35% 80%` | Subtext1 | `#BAC2DE` |
| `--primary` | buttons, links, active state | `343 81% 75%` | Red | `#F38BA8` |
| `--primary-foreground` | text on `--primary` | `240 23% 9%` | Crust | `#11111B` |
| `--accent` | subtle highlight fills | `343 22% 24%` | (dark Red tint) | — |
| `--accent-foreground` | text on `--accent` | `343 55% 86%` | (light Red) | — |
| `--success` | positive status | `115 54% 76%` | Green | `#A6E3A1` |
| `--warning` | caution status | `23 92% 76%` | Peach | `#FAB387` |
| `--danger` | destructive / error | `350 65% 77%` | Maroon | `#EBA0AC` |
| `--ring` | focus ring | `343 81% 75%` | Red | `#F38BA8` |
| `--radius` | base corner radius | `0.75rem` | — | — |

To use a different accent (Mauve, Blue, Teal, …) swap `--primary` / `--ring` /
`--accent*` for that swatch — see <https://catppuccin.com/palette>.

---

## Drop-in CSS

```css
.catppuccin-mocha {
  --background: 240 21% 15%;
  --surface: 240 21% 12%;
  --surface-raised: 237 16% 23%;
  --border: 234 13% 31%;
  --border-strong: 233 12% 39%;
  --foreground: 226 64% 88%;
  --muted: 228 24% 72%;
  --muted-foreground: 227 35% 80%;
  --primary: 343 81% 75%;
  --primary-foreground: 240 23% 9%;
  --accent: 343 22% 24%;
  --accent-foreground: 343 55% 86%;
  --success: 115 54% 76%;
  --warning: 23 92% 76%;
  --danger: 350 65% 77%;
  --ring: 343 81% 75%;
  --radius: 0.75rem;
}
```

Activate with `class="catppuccin-mocha"` on `<html>`, and add it to the `dark:`
variant selector:

```css
/* Tailwind v4 */
@custom-variant dark (&:where(.dark, .dark *, .catppuccin-mocha, .catppuccin-mocha *));
```

---

## Tailwind / plain CSS / runtime switching

Identical wiring to any token-based theme — see `docs/cappuccino-theme.md` for
the `@theme inline` block, the `tailwind.config.js` colours, and the pre-paint
bootstrap script. Only the token values change. Add `"catppuccin-mocha"` to the
theme list and treat it like any other class-based theme.

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
| `foreground` on `background` | ~12 : 1 |
| `muted` on `background` | ~7 : 1 |
| `primary-foreground` on `primary` | ~10 : 1 |
| `foreground` on `surface-raised` | ~9 : 1 |

Body and UI text clear **AA** comfortably (mostly **AAA**). Primary (Red),
danger (Maroon) and warning (Peach) all sit in the warm 340–20° range and at
similar lightness — so never rely on colour alone; pair every status with an
icon or label, and keep destructive actions on `danger`.

---

## Design notes

- **Base as the canvas, Mantle for sunken elements, Surface0 for cards** — the
  standard Catppuccin layering (elevation goes *lighter*).
- **Red primary with near-black (Crust) text** — Catppuccin buttons put the
  Crust/Base on the accent colour.
- **Status colours pulled apart on purpose.** Catppuccin's semantic red *is* the
  accent here, so `danger` borrows Maroon and `warning` borrows Peach to keep
  three readable warm signals instead of one.
- **Everything trends blue-violet (~230–240°)** even in the greys — that cool
  cast under warm accents is the Catppuccin signature.
