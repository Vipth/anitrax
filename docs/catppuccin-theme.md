# Catppuccin — four UI themes

[Catppuccin](https://catppuccin.com/) is a widely-ported pastel palette: four
_flavours_ (Latte, Frappé, Macchiato, Mocha) of 26 fixed colours each, MIT
licensed and explicitly built for ports. This maps all four onto our semantic
token set — Latte is light, the other three are progressively darker.

**All four use the Red accent as `--primary`.** Because Catppuccin's own error
colour is also Red, `danger` uses **Maroon** and `warning` uses **Peach**, so the
three warm status signals stay distinct from the primary. Swap `--primary` /
`--ring` / `--accent*` for Mauve, Blue, Teal… to use a different accent — see
<https://catppuccin.com/palette>.

Token → swatch mapping (same for every flavour):

| Token | Catppuccin swatch |
|---|---|
| `background` | Base (light: Mantle) |
| `surface` | Mantle (light: Crust) |
| `surface-raised` | Surface0 (light: Base) |
| `border` / `border-strong` | Surface1 / Surface2 |
| `foreground` | Text |
| `muted` / `muted-foreground` | Subtext0 / Subtext1 |
| `primary` / `ring` | Red |
| `primary-foreground` | Crust (light: near-white) |
| `success` / `warning` / `danger` | Green / Peach / Maroon |

Values below are `H S% L%` (no `hsl()` wrapper).

---

## Latte (light)

```css
.catppuccin-latte {
  --background: 220 22% 92%;
  --surface: 220 21% 89%;
  --surface-raised: 220 23% 95%;
  --border: 223 16% 83%;
  --border-strong: 225 14% 77%;
  --foreground: 234 16% 35%;
  --muted: 233 10% 47%;
  --muted-foreground: 233 13% 41%;
  --primary: 347 87% 44%;
  --primary-foreground: 220 23% 97%;
  --accent: 347 55% 90%;
  --accent-foreground: 347 75% 35%;
  --success: 109 58% 40%;
  --warning: 22 99% 52%;
  --danger: 355 68% 50%; /* Maroon, darkened for the light ground */
  --ring: 347 87% 44%;
  --radius: 0.75rem;
}
```

## Frappé (dark — warmest, lightest)

```css
.catppuccin-frappe {
  --background: 229 19% 23%;
  --surface: 231 19% 20%;
  --surface-raised: 230 16% 30%;
  --border: 227 15% 37%;
  --border-strong: 228 13% 44%;
  --foreground: 227 70% 87%;
  --muted: 228 29% 73%;
  --muted-foreground: 227 44% 80%;
  --primary: 359 68% 71%;
  --primary-foreground: 229 22% 14%;
  --accent: 359 24% 30%;
  --accent-foreground: 359 60% 85%;
  --success: 96 44% 68%;
  --warning: 20 79% 70%;
  --danger: 358 66% 76%;
  --ring: 359 68% 71%;
  --radius: 0.75rem;
}
```

## Macchiato (dark — middle)

```css
.catppuccin-macchiato {
  --background: 232 23% 18%;
  --surface: 233 23% 15%;
  --surface-raised: 230 19% 26%;
  --border: 231 16% 34%;
  --border-strong: 230 14% 41%;
  --foreground: 227 68% 88%;
  --muted: 227 27% 72%;
  --muted-foreground: 228 39% 80%;
  --primary: 351 74% 73%;
  --primary-foreground: 236 23% 12%;
  --accent: 351 24% 27%;
  --accent-foreground: 351 60% 86%;
  --success: 105 48% 72%;
  --warning: 21 86% 73%;
  --danger: 355 71% 77%;
  --ring: 351 74% 73%;
  --radius: 0.75rem;
}
```

## Mocha (dark — deepest, most saturated)

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

---

## Wiring

Activate a flavour with its class on `<html>`. The three dark flavours go in the
`dark:` variant selector; Latte does not:

```css
/* Tailwind v4 */
@custom-variant dark (&:where(
  .dark, .dark *,
  .catppuccin-frappe, .catppuccin-frappe *,
  .catppuccin-macchiato, .catppuccin-macchiato *,
  .catppuccin-mocha, .catppuccin-mocha *
));
```

For the `@theme inline` block, the `tailwind.config.js` colours, plain-CSS usage,
the token-semantics table, and the pre-paint bootstrap script, see
`docs/cappuccino-theme.md` — none of that changes, only the token values.

---

## Accessibility

Every flavour clears WCAG **AA** for body and UI text (`foreground` on
`background` ≈ 9–13 : 1; `primary-foreground` on `primary` ≈ 6–10 : 1). Primary
(Red), danger (Maroon) and warning (Peach) all sit in the warm 340–20° range at
similar lightness, so never rely on colour alone — pair every status with an icon
or label, and keep destructive actions on `danger`.

## Design notes

- **The flavours are a lightness ladder.** Latte is the light one; Frappé →
  Macchiato → Mocha get darker and more saturated. Hues barely move — the accent
  Red shifts from a strong crimson (Latte `#d20f39`) to a soft pink (Mocha
  `#f38ba8`) simply because it has to work on a lighter vs darker ground.
- **Elevation goes lighter** (dark flavours) — Base canvas, Surface0 cards. Latte
  inverts this the "cappuccino" way: sinks inputs to Crust, floats cards to Base.
- **Everything trends blue-violet (~225–240°)**, even the greys — that cool cast
  under warm accents is the Catppuccin signature.
