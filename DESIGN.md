---
name: Ratatui Kit
description: The site is a TUI app — hairline panes, mono chrome, amber only where something is live.
colors:
  ink-teal-ground: "#081518"
  panel: "#0A1A1E"
  panel-raised: "#0D2126"
  ink: "#E8E3D5"
  ink-dim: "rgba(232, 227, 213, 0.74)"
  teal-structure: "#46C2C7"
  teal-deep: "#2E8A90"
  amber-live: "#FCA91C"
  amber-ink: "#1A1206"
  term-bg: "#05090B"
  term-panel: "#081114"
  paper-ground: "#F4F0E6"
  paper-ink: "#0C3236"
  light-teal: "#0A5B60"
  light-amber: "#B06A05"
typography:
  display:
    fontFamily: "Space Grotesk Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "clamp(2.2rem, 5vw, 3.6rem)"
    fontWeight: 620
    lineHeight: 1.06
    letterSpacing: "-0.028em"
  body:
    fontFamily: "Space Grotesk Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.02rem"
    fontWeight: 400
    lineHeight: 1.68
  label:
    fontFamily: "JetBrains Mono Variable, ui-monospace, SFMono-Regular, Menlo, monospace"
    fontSize: "0.72rem"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "0.05em"
  code:
    fontFamily: "JetBrains Mono Variable, ui-monospace, SFMono-Regular, Menlo, monospace"
    fontSize: "0.8rem"
    lineHeight: 1.66
rounded:
  none: "2px"
spacing:
  xs: "0.45rem"
  sm: "0.75rem"
  md: "1.15rem"
  lg: "1.4rem"
  section: "clamp(5rem, 10vw, 8.5rem)"
components:
  button-primary:
    backgroundColor: "{colors.amber-live}"
    textColor: "{colors.amber-ink}"
    rounded: "{rounded.none}"
    padding: "0.6rem 1.15rem"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "0.6rem 1.15rem"
  panel:
    backgroundColor: "{colors.panel}"
    rounded: "{rounded.none}"
    padding: "1.4rem"
  terminal-window:
    backgroundColor: "{colors.term-bg}"
    rounded: "{rounded.none}"
---

# Design System: Ratatui Kit

## Overview

**Creative North Star: "The Site Is a TUI App"**

The website speaks the same material as the product: a terminal application's chrome, rendered with web craft. Every container is a square hairline pane with a small mono tab sitting on its border — the boxed-title grammar terminals taught us. The ground is deep ink-teal that darkens one tone per scroll act, like descending through an app. Space Grotesk carries human copy; JetBrains Mono carries everything the machine says — labels, keybinds, data, code. Two brand colors hold strict jobs: **teal is structure** (borders, pane titles, links), **amber is live state** (the running demo's hint, the primary CTA, modal focus, the copied confirmation). Terminal windows stay dark in both page themes because a terminal is dark.

Key Characteristics:

- Square corners everywhere (2px); no pills, no soft SaaS cards.
- Hairline panes with mono boxed-title tabs on the border.
- Collapsed-border grids (1px gap over a line-colored background) for dense module fields.
- Terminal windows are interactive and faithful to real example source code.
- The page ends on a vim-style status line.
- Light theme is warm paper; the dark terminals anchor it.

## Colors

Brand duo from the logo, disciplined by subtraction: one structural hue, one active hue, everything else neutral ink.

### Primary
- **Teal Structure** (#46C2C7 dark / #0A5B60 light): pane titles inside terminals, links, run-step commands, structural accents. Never used for "active" state.
- **Teal Deep** (#2E8A90): Starlight accent variables on docs pages.

### Secondary
- **Amber Live** (#FCA91C dark / #B06A05 light text): exclusively for what is live or actionable now — the "this terminal is live" hint, primary CTA fill, confirm-modal focus, selected browser marker, copy-success state. On light pages as text it darkens to #B06A05 for AA.

### Neutral
- **Ink-Teal Ground** (#081518 → #040D0F): page background, a five-stop gradient deepening down the scroll.
- **Panel** (#0A1A1E) / **Panel Raised** (#0D2126): pane fills and hover fills.
- **Ink Ivory** (#E8E3D5) with dim at 0.74 alpha: text.
- **Terminal Black** (#05090B) with **Terminal Panel** (#081114): fixed dark scene for terminal windows, immune to page theme.
- **Paper Ground** (#F4F0E6) with **Paper Ink** (#0C3236): the light theme.

### Named Rules
**The Amber-Is-Alive Rule.** Amber appears only on things that are live, active, or the single next action. If it isn't doing something right now, it is teal or ink.

**The Terminal-Is-Dark Rule.** Terminal windows never follow the page theme. A terminal is dark; on light pages it earns a soft shadow (0 10px 34px rgba(10,50,54,0.2)) instead of a border color change.

## Typography

**Display Font:** Space Grotesk Variable (self-hosted, @fontsource-variable)
**Body Font:** Space Grotesk Variable
**Label/Mono Font:** JetBrains Mono Variable (self-hosted)

**Character:** A geometric grotesk with slight quirk for human statements; a true terminal mono for machine chrome. The pairing reads "engineered, not decorated."

### Hierarchy
- **Display** (620, clamp(2.2rem, 5vw, 3.6rem), 1.06): manifesto H1 only, left-aligned, ≤16ch.
- **Headline** (560, clamp(1.7rem, 3vw, 2.5rem)): section H2s.
- **Title** (580, 1.18rem): module card titles.
- **Body** (400, 1.02rem, 1.68): section copy, ≤62ch measure.
- **Label** (500 mono, 0.72rem, 0.05em tracking): pane tabs, keybind hints, status line, loc counts. Tabular numerals for any data.

### Named Rules
**The Mono-Has-a-Job Rule.** Mono is for code, keys, data, and chrome identity — never a costume for body copy. Human sentences stay in the grotesk.

## Layout

One content column: shell max-width 1152px centered; the header's inner row joins the same column; page gutter is `var(--sl-nav-pad-x, 1.5rem)` so chrome and content align. Sections are separated by deep deliberate gaps (clamp(5rem, 10vw, 8.5rem)) — stacked blocks, not continuous scroll. Dense fields (feature modules, versus panes, browser) use collapsed-border grids: container background `--rk-line`, 1px gaps, children on panel fill — panes share hairlines exactly once, the way terminal splits do. The hero reserves ~880px above the fold for headline, CTAs, and the complete terminal including its status line.

## Elevation & Depth

Depth is hairlines, not shadows. Panels declare themselves by border (1px rgba teal-ink 16%), never by shadow; the single soft shadow in the system appears only under dark terminal windows on light pages, where the border alone cannot carry the separation. The ground itself deepens through five tonal stops across the scroll.

### Named Rules
**The One-Declaration Rule.** A surface declares elevation exactly once — border or shadow, never both.

## Shapes

Square. 2px is the only radius in the system (buttons, panes, windows, code frames). Selection rows square-fill; markers are typographic (`>`, `›`, `→`) in the mono voice. No pills except the browser's mobile chip row, which inherits 2px.

## Components

### Buttons
- **Shape:** square (2px), mono label, 0.02em tracking, min-height 2.6rem.
- **Primary:** amber fill #FCA91C, ink #1A1206; hover lightens to #FFBD45.
- **Ghost:** 1px teal-ink border (0.56 alpha dark), transparent; hover teal text + teal border.
- **Focus:** 2px amber outline, offset 2px, page-wide.

### Terminal Window (signature)
Fixed dark scene regardless of theme: 1px teal border (0.34 alpha), mono bar (`id` left in teal, live status right), interactive app panes with boxed-title tabs centered on their borders, keybind status line at the foot with the amber live hint. Selection rows fill teal #46C2C7 with near-black ink — faithful to the real example's `black().on_cyan()`.

### Pane (the container idea)
1px hairline border, panel fill, and a mono tab label positioned on the top border (negative margin over the line). The tab names the pane; headings inside carry their own weight.

### Component Browser
File-browser topology: mono list rail (selected = amber `>` + raised fill) beside a stage pane; every component renders in the same twin-window frame (code + recording). j/k and arrows work when the list has focus.

### Status Line
The page footer: mono 0.74rem, `ratatui-kit v{version} · MIT` left, amber mode word + doc/github links right.

## Do's and Don'ts

### Do:
- **Do** keep amber for live state only (The Amber-Is-Alive Rule).
- **Do** align everything to the single 1152px column, header included.
- **Do** use tabular numerals for any count, version, or loc figure.
- **Do** keep terminal demos faithful to real example source — data, keybinds, and colors.
- **Do** ship both EN and zh-CN in structural parity.

### Don't:
- **Don't** add a third hue; the palette is teal + amber over neutral ink.
- **Don't** round corners past 2px or introduce pills for large controls.
- **Don't** use eyebrows/kickers above headings; pane tabs name panes, headings speak.
- **Don't** put entrance animations on sections; the terminal's life is the page's one motion budget.
- **Don't** theme terminal windows to follow the page theme.
