---
inclusion: always
---

# Design System — "Void & Astral" + Per-Game Themes

This is the locked visual identity for the Mod Manager app. All UI work (existing and new)
must use these tokens. Do not introduce ad-hoc hex colors, new fonts, or new shape values
without updating this document first.

## Rationale

The base "Void & Astral" palette (dark surfaces, cyan/purple/gold) is the neutral shell. On
top of it, **each supported game gets its own sub-theme** — a distinct accent pair, display
font, corner shape, and background texture — so the app's personality shifts to match
whichever game the user is currently modding, rather than staying visually static:

| Game | Vibe | Font | Shape | Texture |
|---|---|---|---|---|
| Honkai: Star Rail | Sci-fi luxury, cosmic minimalism | Oxanium | Soft (14px radius), thin borders | Faint starfield + grid |
| Genshin Impact | Celestial fantasy, classic elegance | Cinzel (serif) | Very soft (22px radius) | Gold/white sparkle field, no grid |
| Wuthering Waves | Post-apocalyptic desolation, fluid tech | Teko (tall/condensed) | Near-sharp (2px radius) | Horizontal frequency lines |
| Zenless Zone Zero | Streetwear, retro-futurism, comic pop | Bungee (display) | Blocky (4px radius), thick 2px borders | Halftone dot field |
| Arknights: Endfield | Industrial sci-fi, tactical HUD | Russo One (stencil) | Sharp (0px radius) | Coordinate/blueprint grid |

This is the entire point of the theme system — don't build UI that looks identical across
games and only swaps a highlight color. Shape, texture, and type should shift too.

## Token source of truth

All colors, fonts, and shape values are CSS custom properties in `src/styles/globals.css`,
exposed to Tailwind via `tailwind.config.js`. Never hardcode hex values, font names, or
`px` radius values in components — always use the classes/tokens below.

### Base palette (universal — does not change per game)

| Tailwind class | CSS variable | Hex | Role |
|---|---|---|---|
| `bg-surface-0` | `--surface-0` | `#0D0F12` | App base background |
| `bg-surface-1` | `--surface-1` | `#161920` | Cards, panels, sidebar |
| `bg-surface-2` | `--surface-2` | `#1E222B` | Hover / raised surfaces |
| `bg-surface-3` / `border-surface-3` | `--surface-3` | `#282D38` | Borders, dividers |
| `bg-accent` / `text-accent` | `--accent` | `#00F0FF` | Electric cyan — reserved for truly universal chrome that never changes with the game (see below) |
| `bg-gold` / `text-gold` | `--gold` | `#FFD166` | 5-star gold — favorites, "update available" badges, premium flags ONLY |
| `text-primary` / `text-secondary` / `text-muted` | see file | `#F3F4F6` / `#C3C6CB` / `#9CA3AF` | Text hierarchy |

### Per-game dynamic tokens (change with the active game)

Applied via `data-game="<game-id>"` on the root element (`src/App.tsx`, driven by
`activeGame` in `appStore`). Attribute selectors in `globals.css` override these:

| Tailwind class | CSS variable | Role |
|---|---|---|
| `bg-game` / `text-game` / `border-game` | `--game-accent` | Primary identity color for the active game. This is the main interactive color — nav active state, buttons, focus rings, selection state. |
| `bg-game2` / `text-game2` / `border-game2` | `--game-accent2` | Secondary identity color for the active game (its paired/contrast color). Status tags, secondary emphasis, section label color. |
| `font-display` | `--font-display` | The active game's display typeface. Page titles, `.hud-label`, wordmark. |
| (used by `.game-panel`) | `--radius-panel` | Corner radius for cards/panels — ranges from 0px (Endfield) to 22px (Genshin). |
| (used by `.game-control`) | `--radius-control` | Corner radius for buttons/inputs/chips. |
| (used by `.app-shell`) | `--bg-pattern`, `--bg-pattern-size` | The ambient background texture unique to each game. |

**Never use the raw per-game hex values directly in components.** Always go through
`game`/`game2` so switching games updates every surface automatically.

## Per-game accent + typography reference

| Game ID | Primary (`game`) | Secondary (`game2`) | Display font |
|---|---|---|---|
| `honkai-star-rail` (default) | Starlight Purple `#8B5CF6` | Electric Cyan `#00F0FF` | Oxanium |
| `genshin-impact` | Celestial Teal `#48D1CC` | Mappa Gold `#E5C158` | Cinzel |
| `wuthering-waves` | Wave Cyan `#00E5FF` | Monochrome `#E2E8F0` | Teko |
| `zenless-zone-zero` | Acid Yellow `#FAFD00` | Neon Magenta `#FF0055` | Bungee |
| `arknights-endfield` | Hazard Orange `#FF5500` | Safety Yellow `#FFC400` | Russo One |

## Usage rules

- **`game` (primary per-game accent)**: the default color for anything interactive —
  active nav/tab state, buttons, focus rings, toggle-on switches, selection highlights,
  the logo mark. This replaces what used to be a fixed cyan; it now shifts with the game.
- **`game2` (secondary per-game accent)**: status tags, section label headers (`.hud-label`
  color), dropdown/badge accents — anything that's a supporting highlight rather than the
  primary action.
- **`accent` (fixed cyan)**: reserve for chrome that must stay recognizable regardless of
  game — e.g. a system "connected/ready" status dot, app-level (not game-level) alerts.
  Use sparingly; most interactive elements should use `game`, not `accent`.
- **Gold**: reserved exclusively for favorites, "new version available" badges, and
  premium/pinned flags, regardless of active game. Do not use gold for generic emphasis.
- **Glassmorphism**: `.glass-panel` / `bg-surface-1/90 backdrop-blur-xl` for floating
  panels (sidebar, detail panel, dropdowns) so the ambient shell glow reads through the
  edges. Don't make panels fully opaque.
- **Text hierarchy**: `text-primary` for headings/body, `text-secondary` for supporting
  text, `text-muted` for metadata (authors, versions, timestamps) — unaffected by game.

## Typography

- `font-display` resolves to `var(--font-display)`, which changes per game (see table
  above). Use it for page titles (`h1`), `.hud-label` section headers, and the sidebar
  wordmark. Weight 500-700 depending on the font's available weights.
- `font-sans` (Inter, fixed) — everything else: body text, buttons, inputs, mod names,
  descriptions. Never switches per game — only display/heading text should shift font.
- Don't use `font-display` for dense body copy; several of the per-game fonts (Teko,
  Bungee, Russo One) are display faces that become hard to read at small sizes /
  long strings.

## Shape language

Corner radius is a per-game token, not a fixed value. Two utility classes apply it:

- **`.game-panel`**: sets `border-radius: var(--radius-panel)` (and standardizes border
  width via `--border-width-panel`). Use on cards, panels, dropdowns, list rows — anything
  that would otherwise be `rounded-md`/`rounded-lg`/`rounded-2xl`.
- **`.game-control`**: sets `border-radius: var(--radius-control)`. Use on buttons, inputs,
  chips, small icon buttons — anything that would otherwise be `rounded-md`.
- **Never hardcode `rounded-md` / `rounded-lg` / `rounded-xl` / `rounded-2xl` on a
  panel or control.** The one exception is small circular status dots (`rounded-full`),
  which should stay circular regardless of game — a notched or square dot reads as broken,
  not thematic.
- **`.hud-panel`**: an angular notched-corner clip-path keyed to `--radius-control`, for
  an industrial-hardware feel on top of `.game-panel`. Best on Wuthering Waves / Endfield
  (sharp shape language); has little visible effect on Genshin's large radius, which is
  fine — don't force it there.

## Ambient background & texture

- **`.app-shell`** (applied to the root `<div>` in `App.tsx`): draws two radial glows
  (using `game` + `game2`) plus the active game's `--bg-pattern` texture (starfield for
  Star Rail, sparkles for Genshin, frequency lines for Wuthering Waves, halftone dots for
  ZZZ, coordinate grid for Endfield). This must stay on the root element — never cover it
  with an opaque `bg-surface-0` on a direct child, or the texture disappears (this was a
  real bug during Phase 1 — `MainContent` had `bg-surface-0` which blocked it entirely).
- When adding a new page/view, keep its outermost container transparent (no `bg-surface-*`
  fill) so the shell texture is visible behind panels and empty states.

## Expressive HUD details

- **`shadow-glow-game` / `shadow-glow-game-lg` / `shadow-glow-game2`**: glow shadows keyed
  to the active game's accents. Use for active/selected/focused elements — color alone
  reads flat without a glow to sell the "lit HUD element" feeling.
- **`shadow-glow-accent`**: fixed cyan glow, only for the universal chrome described above.
- **`shadow-glow-gold`**: pair only with favorite/premium/update-available elements.
- **`.hud-rule`**: thin glowing gradient divider (fades from `game` accent to transparent),
  used directly under section headers instead of a plain `border-b`.
- **`.hud-label`**: uppercase, letter-spaced, `font-display` weight 600 — pair with
  `.hud-rule` beneath for section headers ("GAME DIRECTORIES", "NAVIGATION").
- **Status dots**: small `rounded-full` dot in `bg-game` (or a static hex swatch when
  representing a *different, non-active* game in a list — see `GameSelector.tsx` for the
  pattern) with a matching glow shadow. Prefer this over checkmark icons for state that's
  tied to color identity.
- **`animate-pulse-glow`**: slow breathing opacity animation for idle status indicators.
  Don't use it for anything urgent — that needs its own faster animation later.

## Adding new UI

1. Background containers use `surface-0` (app shell only, via `.app-shell`) →
   `surface-1/90` + `backdrop-blur-xl` (panels/cards) → `surface-2` (hover/raised) →
   `surface-3` (borders), in that nesting order. Keep panels translucent, not opaque.
2. Interactive elements default to `game` (not a fixed color) for anything that should
   feel like "this game's UI." Pair with `shadow-glow-game` when representing
   active/selected/focused state.
3. Section headers use `.hud-label` + `.hud-rule`.
4. Cards/panels use `.game-panel`; buttons/inputs/chips use `.game-control`. Never
   hardcode a `rounded-*` radius on these.
5. When listing multiple games at once (e.g. a game picker), each entry's own identity
   color must come from a static per-game hex map in that component (see
   `GameSelector.tsx`'s `swatch` field) — not from `--game-accent`, which only reflects
   the currently active game and would make every entry look the same.
6. Never add a new color token, font, or shape value without updating this file,
   `globals.css`, and `tailwind.config.js` together, for all five games consistently.
7. Icons: continue using `lucide-react`.
