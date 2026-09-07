# Kanagawa — a dark UI theme

[Kanagawa](https://github.com/rebelot/kanagawa.nvim) is rebelot's neovim
colorscheme inspired by Katsushika Hokusai's woodblock print *The Great Wave off
Kanagawa* — warm sumi-ink greys, paper-white text, a crystal-blue wave. This is
the **Wave** variant (the default) mapped onto our semantic token set. Dark only.

Token → Kanagawa swatch:

| Token | Swatch |
|---|---|
| `background` | `sumiInk3` |
| `surface` | `sumiInk0` |
| `surface-raised` | `sumiInk4` |
| `border` / `border-strong` | `sumiInk5` / `sumiInk6` |
| `foreground` | `fujiWhite` |
| `muted-foreground` | `oldWhite` |
| `primary` / `ring` | `crystalBlue` |
| `primary-foreground` | `sumiInk0` |
| `accent` | `waveBlue2` |
| `success` | `springGreen` |
| `warning` | `carpYellow` |
| `danger` | `waveRed` |

Values are `H S% L%` (no `hsl()` wrapper).

---

## Drop-in CSS

```css
.kanagawa {
  --background: 240 13% 14%;
  --surface: 240 14% 10%;
  --surface-raised: 240 13% 19%;
  --border: 240 13% 24%;
  --border-strong: 240 13% 38%;
  --foreground: 51 33% 80%;
  --muted: 48 12% 58%;
  --muted-foreground: 51 33% 68%;
  --primary: 220 54% 67%;
  --primary-foreground: 240 14% 10%;
  --accent: 205 39% 29%;
  --accent-foreground: 198 45% 80%;
  --success: 87 37% 58%;
  --warning: 39 66% 71%;
  --danger: 353 70% 65%;
  --ring: 220 54% 67%;
  --radius: 0.75rem;
}
```

Activate with `class="kanagawa"` on `<html>`; add it to the `dark:` variant
selector:

```css
/* Tailwind v4 */
@custom-variant dark (&:where(.dark, .dark *, .kanagawa, .kanagawa *));
```

For the `@theme inline` block, `tailwind.config.js` colours, plain-CSS usage, the
token-semantics table, and the pre-paint bootstrap script, see
`docs/cappuccino-theme.md` — only the token values change.

---

## Accessibility

Approximate contrast ratios (WCAG 2.1):

| Pair | Ratio |
|---|---|
| `foreground` on `background` | ~10 : 1 |
| `muted` on `background` | ~4.6 : 1 |
| `primary-foreground` on `primary` | ~7 : 1 |
| `foreground` on `surface-raised` | ~8 : 1 |

Body and UI text clear **AA**. `muted` sits just above 4.5 : 1 — fine for text,
but keep fine print at `foreground`. As always pair every status with an icon or
label rather than relying on the (fairly muted) hue alone.

## Design notes

- **Sumi-ink neutrals carry a faint violet** (~240°, ~13% sat) — the ink is
  never a flat grey.
- **`foreground` is `fujiWhite` — a warm paper cream**, not white. That
  ink-on-washi contrast is the whole aesthetic; pairing it with the cool
  `crystalBlue` primary is the print's own palette.
- **Status colours are the muted Kanagawa set** — `carpYellow` (a soft gold, not
  a shouting amber) and `waveRed` (a rose-red) — so warnings and errors read as
  part of the woodblock rather than stickers on top of it.
- Kanagawa also ships **Dragon** (darker, desaturated) and **Lotus** (light);
  swap the token values from the palette in the repo to add those.
