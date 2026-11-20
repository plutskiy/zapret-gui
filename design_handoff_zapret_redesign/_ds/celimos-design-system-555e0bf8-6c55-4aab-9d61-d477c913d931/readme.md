# Celimos Design System

**Celimos** is a modern desktop music client for self-hosted **[Navidrome](https://www.navidrome.org/)** (Subsonic-API) servers — built for people who own their music and want a premium, cover-forward listening experience on macOS. The aesthetic sits in the "deep-black premium player" genre: near-black surfaces, album art as the hero, high-contrast type, a single luminous teal accent, and a custom-branded window that still feels native to the Mac.

This project is the design system **and** a working UI-kit recreation of the client.

> **Provenance / sources.** Celimos is an original brand created in this project — there is no external codebase, Figma, or brand kit behind it. The product brief: *"a player for Navidrome on Mac with a modern design… in the spirit of a premium streaming app."* The visual genre references high-end streaming desktop clients generally; **no competitor's proprietary brand, logo, color, or layout is copied.** All artist/album/playlist names in the demo catalog are invented, and all cover art is generated procedurally (`CoverArt`), so the system ships nothing copyrighted.

---

## CONTENT FUNDAMENTALS

How Celimos writes copy.

- **Voice:** calm, confident, unfussy. The product is a quiet utility for music lovers — it never shouts. Think "liner notes," not "marketing."
- **Person:** addresses the listener as **you** ("Your Library", "Made for you", "Artists you follow"). The app refers to itself in third person only in system contexts ("Celimos · Playlist").
- **Casing:** **Sentence case everywhere** for titles, buttons, and section headers ("Recently added", "Save & sync", "Browse all") — never Title Case. The two exceptions: the **overline labels** (UPPERCASE, wide tracking, used for shelf kickers and column headers like `NOW PLAYING`, `#`, `TITLE`), and the wordmark **Celimos**.
- **Greetings & time:** warm, time-aware ("Good morning / afternoon / evening") — the only sentimental touch.
- **Metadata is terse and technical:** durations `3:42`, counts `48 songs`, totals `2 hr 14 min`, formats `FLAC · 1411 kbps · 44.1 kHz`, listener stats `1,284,902 monthly listeners`. Set in the mono face. Middots (`·`) separate metadata fields, never commas or pipes.
- **Buttons are verbs:** "Play all", "Follow", "Save & sync", "Test connection", "Disconnect server". Destructive actions name the consequence ("Disconnect server", not "Remove").
- **Empty / error states are plain and kind:** "No results for ‘…’" — no jokes, no exclamation points.
- **Numbers:** thousands get separators (`1,284 albums`); abbreviate large play counts (`1.8M`).
- **Emoji:** none. Ever. The interface speaks in iconography, not emoji.
- **Quotes:** curly typographic quotes (' ' " ") in display copy.

---

## VISUAL FOUNDATIONS

The motifs that make something look like Celimos.

### Color & mood
- **Deep-black, near-monochrome chrome.** A tight ink ramp from `#060607` (app) through `#181820` (card) to `#34343f` (fills). Surfaces are distinguished by *value*, hairline borders, and a faint top-edge highlight — **not** by big shadows or color. See `tokens/colors.css`.
- **One brand color: luminous teal** `#00d9c0`. Used sparingly and with intent — the play FAB, active/selected states, focus rings, progress fill, quality badges, the equalizer animation. Because it's the only saturated color on a black field, it carries enormous weight; don't dilute it.
- **Album art is the only other color source.** Detail pages derive a **color wash** (a top-down gradient) from the cover's hue, fading to black. The full-screen Now Playing builds its whole background from the current cover's hue. Imagery is rich and saturated; the chrome stays neutral so art pops.
- **Accent is themeable** via `data-accent="indigo|gold|crimson"` — the variation axis the brief asked for. Everything semantic re-points; the dark foundation never changes.
- **Text** is four levels of warm near-white (`#f4f5f7` at 100/66/42/26%). Never pure `#fff` except over art.

### Type
- **Inter Tight** for display & headers (heavy 700–800, tight `-0.02em` tracking) — gives titles a premium, condensed-poster feel.
- **Inter** for all UI & body. **JetBrains Mono** for time codes, bitrates, counts — anything tabular/technical.
- Spacious scale: page titles 32px, shelf headers 24px, detail titles 44px (hero artist names up to 72px). Metadata floor 12px. See `tokens/typography.css`.

### Space, shape & elevation
- **8px grid**, generous gutters (40px page padding — "spacious" per the brief).
- **Radii:** 10px controls/cards, 14px album tiles, 20px on the Now-Playing panel/dialogs, full pills for buttons/chips/segmented controls, circles for artists & the play FAB.
- **Elevation** reads through low-spread cool shadows (`0 8px 24px rgba(0,0,0,.5)`) + hairline borders, *not* glow — except the **accent glow** under the play FAB and focused primary action. Cover tiles get a larger "art lift" shadow on hover.
- **Glass / blur** is reserved for floating chrome: the title bar and (optionally) a floating play bar use `backdrop-filter: saturate(160%) blur(18px)` over a translucent ink fill.

### Motion
- **Standard** `cubic-bezier(.4,0,.2,1)` at 140–220ms for hovers and most transitions.
- **ease-out** `cubic-bezier(.16,1,.3,1)` for entrances (view changes, the play FAB revealing on a tile, panels sliding in).
- **spring** `cubic-bezier(.34,1.56,.64,1)` for tactile moments — play/pause press, switch knob, tile play button.
- Cover crossfades are slow (560ms). The **equalizer bars** on the active track loop subtly; nothing else loops. All durations collapse to 0 under `prefers-reduced-motion`.

### Interaction states
- **Hover:** surfaces lighten one ink step; album tiles raise 2px + reveal the play FAB; track-row index becomes a play glyph; like/more icons fade in.
- **Active/selected:** teal text + a teal `--teal-tint` row wash; nav items get a 3px teal left bar.
- **Press:** controls scale down (0.92–0.97); the FAB springs.
- **Focus:** 2px teal ring + soft halo (keyboard only).

### Layout rules
- Fixed **title bar** (60px, traffic lights + global search + account) and **play bar** (80px) bracket the window; a fixed 256px **sidebar** and an optional 340px **queue** flank a single scrolling content column.
- Detail pages: cover + metadata header over the color wash, then an action row (FAB · shuffle · save · download · more), then the track list with a column header.
- Cards never use a colored left-border accent; selection is shown by fill/text color and the nav left-bar only.

---

## ICONOGRAPHY

- **One icon component** is the source of truth: `components/core/Icon.jsx`. It carries ~40 glyphs at a 24×24 grid.
- **Outline icons follow [Lucide](https://lucide.dev) geometry** (ISC-licensed) — 1.9px stroke, round caps/joins, `currentColor`. These cover navigation and chrome (search, home, library, settings, heart, share, sort, server, volume, etc.). Lucide was chosen as the closest match to the desired thin, geometric, modern stroke; the subset is hand-embedded so the system has **no runtime icon-font dependency**. The full Lucide set is CDN-available (`https://unpkg.com/lucide-static`) if a glyph is missing — match the 1.9px stroke when adding one.
- **Transport glyphs are filled** (play, pause, skip, shuffle, repeat, repeatOne, heartFill) — universal player symbols, drawn solid for presence at small sizes. These are geometric primitives, not branded marks.
- **The `[E]` explicit mark** and **Lossless/Hi-Res quality pills** are bespoke text badges, not icons (`components/core/Badge.jsx`).
- **No emoji, no unicode-glyph icons.** The brand mark uses `♪` only as the Avatar fallback when a name is empty.
- Icon sizes: 16 (inline/meta), 20 (default), 24 (large controls). Color follows text level or goes teal when a control is `active`.
- **Brand marks** live in `assets/`: `logo-mark.svg` (the nested-C sound-pulse glyph), `logo-wordmark.svg`, `app-icon.svg` (rounded-rect macOS icon). The mark reads simultaneously as the letter **C** and as sound radiating from a point.

---

## Index — what's in this project

**Foundations**
- `styles.css` — the single entry point consumers link (import list only).
- `tokens/` — `fonts.css`, `colors.css` (+ accent themes), `typography.css`, `spacing.css`, `elevation.css` (shadow/blur/motion), `base.css` (reset).
- `guidelines/*.card.html` — foundation specimen cards (Colors, Type, Spacing, Brand) shown in the Design System tab.
- `assets/` — `logo-mark.svg`, `logo-wordmark.svg`, `app-icon.svg`.

**Components** (`window.CelimosDesignSystem_555e0b`)
- `components/core/` — `Icon`, `Button`, `IconButton`, `PlayButton`, `Slider`, `Input`, `Badge` (+ `ExplicitBadge`, `QualityBadge`), `Chip`, `Avatar`, `Switch`, `SegmentedControl`, `Tabs`.
- **Customization layer** (`components/core/`) — `Select` (dark dropdown), `AccentPicker` (+ `ACCENT_PRESETS`), `NumberStepper`, `SettingsGroup` (+ `SettingsRow`). These build any preferences UI.
- `components/media/` — `CoverArt` (generative art), `TrackRow`, `AlbumTile`.
- Each directory has a `*.card.html` (Design System tab), and each component has a `.d.ts` + `.prompt.md`.

**UI kit**
- `ui_kits/celimos-app/` — the full interactive Navidrome client (`index.html` + screens). See its `README.md`.

**Meta**
- `SKILL.md` — makes this design system usable as a downloadable Agent Skill.
- `EXPORT.md` — how to consume the system in your own React/Electron Navidrome app (drop-in, source import, and the configurable-preferences layer).
- Generated (do not edit): `_ds_bundle.js`, `_ds_manifest.json`, `_adherence.oxlintrc.json`.

---

## Configurability (Feishin-style)

Celimos is built to be reconfigured at runtime, the way Feishin is. The full
mechanism lives in `ui_kits/celimos-app/preferences.js` + a few token scopes:

- **Theme** — `data-theme="dark|light"` on a container flips the entire surface
  palette (`tokens/colors.css`). Dark is the default and the brand's home.
- **Accent** — `data-accent="teal|indigo|emerald|amber|gold|crimson|custom"`
  re-points `--accent` everywhere; `custom` reads an inline `--accent` hex, so
  any color works. Driven by the `AccentPicker` component.
- **Density / text size / tile size** — CSS-var overrides (`--content-pad`,
  `--row-h`, `--tile-lg`, `--app-font-scale`) computed by `preferences.rootProps()`.
- **Layout** — sidebar visibility, queue location (side panel vs in-player),
  play-bar style (docked vs floating), library view (grid vs list).
- **Player** — now-playing background (cover-hue wash vs solid), animated equalizer.

The **Settings → Appearance** tab in the UI kit is a complete worked example —
every control live-updates the running app and persists to `localStorage`.
Extend it by adding keys to `defaults` and reading `prefs.<key>`.

---

## Caveats / substitutions
- **Fonts** are loaded from Google Fonts (`Inter`, `Inter Tight`, `JetBrains Mono`) via `@import` in `tokens/fonts.css`. They are real choices, not placeholders, but to ship fully **offline** they should be self-hosted as woff2. (The brief mentioned a Söhne-like grotesque — Inter is the free, closest match; swap in a licensed face if desired.)
- **Icons** use an embedded Lucide subset (ISC). Flagged as the closest match to the intended thin-stroke style.
