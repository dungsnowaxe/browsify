---
name: Browsify
description: Warm-stone macOS desktop instrument for deterministic browser and profile routing.
colors:
  terracotta-accent: "#c5492e"
  terracotta-ink: "#b94027"
  on-accent: "#ffffff"
  warm-desk: "#d3d4d0"
  warm-canvas: "#f5f5f3"
  stone-surface: "#eaeae7"
  porcelain-panel: "#fbfbfa"
  crisp-control: "#ffffff"
  warm-inset: "#e9e9e5"
  charcoal-ink: "#252624"
  slate-muted: "#62645f"
  control-stroke: "#80827a"
  graphite-rail: "#454642"
  rail-paper: "#f0f0ec"
  switch-track: "#767872"
  danger-crimson: "#b83238"
  amber-warning: "#875b06"
  moss-success: "#32704b"
  obsidian-canvas: "#252622"
  dark-surface: "#30312c"
  dark-panel: "#34352f"
  coral-accent: "#f28a6b"
  coral-ink: "#ff9c7f"
  dark-on-accent: "#301c15"
typography:
  display:
    fontFamily: "Sora, sans-serif"
    fontSize: "38px"
    fontWeight: 400
    lineHeight: 1.12
    letterSpacing: "-0.03em"
  headline:
    fontFamily: "Sora, sans-serif"
    fontSize: "28px"
    fontWeight: 400
    lineHeight: 1.15
    letterSpacing: "-0.03em"
  title:
    fontFamily: "Figtree, sans-serif"
    fontSize: "16px"
    fontWeight: 500
    lineHeight: 1.35
    letterSpacing: "-0.02em"
  body:
    fontFamily: "Figtree, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0em"
  label:
    fontFamily: "Figtree, sans-serif"
    fontSize: "13px"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "0em"
  mono:
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "0em"
rounded:
  control: "14px"
  panel: "24px"
  frame: "38px"
  pill: "999px"
spacing:
  xs: "6px"
  sm: "10px"
  md: "16px"
  lg: "20px"
  xl: "28px"
components:
  button-primary:
    backgroundColor: "{colors.terracotta-accent}"
    textColor: "{colors.on-accent}"
    rounded: "{rounded.pill}"
    padding: "10px 18px"
    height: "44px"
  button-secondary:
    backgroundColor: "{colors.crisp-control}"
    textColor: "{colors.charcoal-ink}"
    rounded: "{rounded.pill}"
    padding: "10px 18px"
    height: "44px"
  button-danger:
    backgroundColor: "{colors.crisp-control}"
    textColor: "{colors.danger-crimson}"
    rounded: "{rounded.pill}"
    padding: "10px 18px"
    height: "44px"
  panel-card:
    backgroundColor: "{colors.porcelain-panel}"
    textColor: "{colors.charcoal-ink}"
    rounded: "{rounded.panel}"
    padding: "20px"
  input-field:
    backgroundColor: "{colors.crisp-control}"
    textColor: "{colors.charcoal-ink}"
    rounded: "{rounded.control}"
    padding: "10px 14px"
    height: "44px"
  nav-rail:
    backgroundColor: "{colors.graphite-rail}"
    textColor: "{colors.rail-paper}"
    rounded: "{rounded.pill}"
    padding: "16px 4px"
    width: "84px"
  nav-item-active:
    backgroundColor: "{colors.crisp-control}"
    textColor: "{colors.terracotta-ink}"
    rounded: "{rounded.pill}"
    padding: "8px 0"
    height: "48px"
---

# Design System: Browsify

## Overview

**Creative North Star: "The Precision Desk Instrument"**

Browsify is a local-first macOS desktop utility presented as a calm, tactile desk instrument rather than a sprawling SaaS dashboard. Every screen sits inside a grounded, warm-stone window frame anchored by a dark graphite navigation rail on the left and clean porcelain work panels on the right. The interface favors immediate legibility, direct manipulation, and predictable macOS conventions—giving technical and everyday Mac users confidence that clicking any link opens the exact browser and profile they expect.

The aesthetic rejects generic neon-purple AI gradients, decorative glass blur, uppercase eyebrow kickers, and marketing chrome. Color stays quiet until it carries operational meaning: warm stone neutrals absorb visual noise, terracotta marks the single primary action or active route, and semantic tones (crimson, amber, moss) appear strictly for errors, unsaved drafts, and verified readiness. Both Light and Dark themes preserve this material contrast, shifting from sunlit warm paper to low-glare olive-charcoal without altering layout geometry or hierarchy.

**Key Characteristics:**
- Warm stone and porcelain surfaces paired with a single dark graphite navigation rail for instant spatial orientation.
- One terracotta accent voice reserved for primary calls to action, active toggles, and active navigation focus.
- Concentric rounded geometry stepping down cleanly from the outer desktop frame (`38px`) to cards (`24px`), inputs (`14px`), and tactile pill buttons (`999px`).
- Accessible, unhurried interaction geometry: every interactive button, toggle target, select, and link meets a `44px` minimum hit height with unmistakable focus rings.
- Local-first transparency: explicit connection status, staged Config Draft save/discard footers, and dry-run URL testing that never opens a browser by surprise.

## Colors

The palette pairs olive-tinted warm stone neutrals with a single earthy terracotta accent, ensuring ≥ 4.5:1 text contrast and ≥ 3:1 interactive control contrast across both Light and Dark themes.

### Primary
- **Terracotta Accent** (`#c5492e` light / `#f28a6b` dark): Reserved for primary action buttons (`Save changes`, `Add rule`, `Retry connection`), checked toggle tracks, Rewrite Rule ring arcs, and the primary URL test submit button. Paired with crisp white (`#ffffff`) ink in light mode and deep espresso (`#301c15`) ink in dark mode.
- **Terracotta Ink** (`#b94027` light / `#ff9c7f` dark): Dedicated accessible ink token for inline text links, active navigation rail labels, focus-visible rings (`2px solid`), and debug log badges, maintaining ≥ 4.5:1 contrast on light panels and dark surfaces.

### Neutral
- **Warm Desk** (`#d3d4d0` light / `#22231f` dark): Outer viewport backdrop behind the floating desktop window frame.
- **Warm Canvas** (`#f5f5f3` light / `#252622` dark): Primary interior background of the desktop `.app-frame` workspace.
- **Stone Surface** (`#eaeae7` light / `#30312c` dark): Recessed grouping containers such as the Home setup card and the URL tester housing.
- **Porcelain Panel** (`#fbfbfa` light / `#34352f` dark): Elevated content cards, Handler Rule rows, Preference sections, and modal dialogs.
- **Crisp Control** (`#ffffff` light / `#3f4039` dark): Interactive input fields, dropdown selects, secondary pill buttons, and active segmented pills.
- **Warm Inset** (`#e9e9e5` light / `#3c3d36` dark): Subtle recessed wells for segmented tracks, disabled controls, inline code spans, and secondary hover states.
- **Charcoal Ink** (`#252624` light / `#f3f3ed` dark): Primary body text, headings, and numeric totals.
- **Slate Muted** (`#62645f` light / `#b6b8ad` dark): Secondary descriptions, form labels, timestamps, and metadata hints.
- **Control Stroke** (`#80827a` light / `#8e9085` dark): High-contrast `1px` structural borders on interactive inputs, selects, buttons, and filter chips (≥ 3:1 non-text contrast against adjacent surfaces). Softens to `--browsify-border-strong` (`rgba(37, 38, 36, 0.14)` light / `rgba(255, 255, 255, 0.14)` dark) when a control is disabled.
- **Graphite Rail** (`#454642` light / `#191a17` dark): Persistent navigation rail surface paired with **Rail Paper** (`#f0f0ec` / `#efefe9`) text for clear separation between global navigation and page content.

### Semantic
- **Danger Crimson** (`#b83238` light / `#ff9da0` dark): Destructive actions (`Discard changes`, `Delete rule`, `Clear logs`), disconnected status dots, validation errors, and error activity badges.
- **Amber Warning** (`#875b06` light / `#edc77e` dark): Staged `Unsaved` pills, dirty-draft notices, incomplete rule warnings, and macOS Login Item approval prompts.
- **Moss Success** (`#32704b` light / `#a2c6a8` dark): Connected status dots, verified routing setup readiness, `Up to date` indicator, and matched Handler Rule indicators.

### Named Rules
**The One Accent Voice Rule.** Terracotta (`#c5492e` / `#f28a6b`) is used strictly for the single primary action, active route, or enabled toggle—never for decorative gradients, card fills, or secondary icons.
**The Interactive Stroke Rule.** Every interactive input, select, secondary button, and segmented thumb uses `Control Stroke` (`#80827a` / `#8e9085`) to guarantee ≥ 3:1 non-text contrast; disabled controls step down to `--browsify-border-strong` so unavailable controls visibly recede without collapsing contrast on labels.
**The Neutral Stone Rule.** Never introduce cold pure-gray (`#808080`) or pure-black (`#000000`) surfaces; all neutral surfaces carry Browsify's subtle warm-olive undertone (`#252624` through `#fbfbfa`).

## Typography

**Display Font:** `Sora` (fallback: `sans-serif`), self-hosted Latin `woff2` (`400`–`500`).
**Body Font:** `Figtree` (fallback: `sans-serif`), self-hosted Latin `woff2` (`400`–`500`).
**Label/Mono Font:** `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace` for URLs, wildcard patterns, CLI commands, TOML paths, and activity log streams.

**Character:** `Sora` gives page titles and numeric counts an architectural, geometric clarity at regular weight (`400`), while `Figtree` keeps dense configuration forms, rule summaries, and diagnostic copy warm, crisp, and effortless to scan.

### Hierarchy
- **Display** (`400`, `38px` desktop / `28px` mobile, `1.12` line-height, `-0.03em` tracking): Reserved exclusively for the Home hero greeting (`Browser routing, at a glance.`).
- **Headline** (`400`, `28px`, `1.15` line-height, `-0.03em` tracking): Top-of-page `<h1>` titles inside `PageContainer` (`Rules`, `URL tester`, `Activity`, `Preferences`, `About`).
- **Title** (`500`, `16px`–`19px`, `1.25`–`1.35` line-height, `-0.02em` to `-0.025em` tracking): Section group headers (`Primary Config`, `This Mac & app`, `Your routing`) and card titles (`Default Browser Target`, `Browsify updates`).
- **Body** (`400`, `14px`, `1.5` line-height): Standard page descriptions, form inputs, option descriptions, and modal body copy (constrained to ~56–65ch for comfortable reading).
- **Label** (`500`, `13px`, `1.4` line-height): Button text, navigation labels, form field labels, footer metadata, and status badges. Never rendered smaller than `12px`.
- **Mono** (`400`, `12px`–`13px`, `1.45` line-height): Inline commands (`browsify ui`), wildcard pattern examples (`*github.com/*`), final rewritten URLs, and log entries.

### Named Rules
**The Zero-Eyebrow Rule.** Never place tiny uppercase kicker/eyebrow labels above headings; lead directly with the clear sentence-case heading and a concise subtitle when needed.
**The Tabular Metric Rule.** All counts, rule numbers, log timestamps, and version numbers apply `font-variant-numeric: tabular-nums` so numbers align cleanly and never jitter when live data updates.
**The Tracking Floor Rule.** Tightened display tracking never goes below `-0.035em` (and never below `-0.04em` anywhere), preserving letter separation in `Sora` across macOS Retina and standard displays.

## Layout

Browsify uses a pinned desktop application shell centered on the viewport, transitioning cleanly into a single-column mobile layout below `700px`.

- **Desktop Window Frame (`> 700px`):** Fixed `1080px × 760px` `.app-frame` (`max-width: calc(100vw - 32px)`, `max-height: calc(100svh - 32px)`) with `16px` outer padding, `28px` gap between the `84px` capsule navigation rail on the left and the flexible right `.workspace`. Inside `.workspace`, `<main class="container">` scrolls independently while draft error banners, the sticky unsaved `Config Draft` action bar, and the global `AppFooter` stay anchored at the bottom.
- **Home Bento Grid:** Two-column top split (`1.1fr 1fr`, `22px` gap) pairing the routing overview and setup status card with the compact `URL tester` (`356px` fixed height on desktop), followed by a 3-column `Your routing` summary row (`1.15fr 1fr 1fr`, `14px` gap). Collapses to a single column at `≤ 800px`.
- **Configuration & Diagnostic Pages:** Vertical stack with `14px`–`18px` gaps inside `PageContainer`. `Preferences` groups controls into two explicit save-scope regions (`Primary Config` staged in draft vs. `This Mac & app` immediate), separated by a `1px` divider and `20px` top clearance. `App behavior` uses a 2-column grid (`12px` gap) collapsing to 1 column at `≤ 800px`.
- **Mobile Adaptation (`≤ 700px`):** `.app-frame` expands to fill the viewport minus `8px` outer margin (`border-radius: 28px`, `10px` padding). The navigation rail becomes a compact top bar housing the brand header and a 6-slot icon+label grid (`Home`, `Rules`, `Test`, `Activity`, `Prefs`, `About`). The scroll container uses clean `18px` bottom padding with a subtle upward divider shadow on `AppFooter`—never a text-clipping bottom `mask-image`.

## Elevation & Depth

Browsify uses a **hybrid tonal-layering + subtle ambient shadow** depth system modeled after precision milled hardware resting on a warm desk. Surface hierarchy is established primarily by lightness step (`Desk` → `Canvas` → `Surface` → `Panel` → `Control`), reinforced by soft low-opacity directional shadows and `1px` top-edge inner highlights that catch ambient light without harsh drop shadows.

### Shadow Vocabulary
- **Frame Shadow** (`box-shadow: 0 24px 80px color-mix(in srgb, var(--browsify-rail) 18%, transparent)`): Grounds the `1080×760` desktop window frame against the outer viewport desk.
- **Panel Shadow** (`box-shadow: 0 6px 18px rgba(37, 38, 36, 0.035), inset 0 1px 0 rgba(255, 255, 255, 0.6)` light / `0 6px 18px rgba(0, 0, 0, 0.06), inset 0 1px 0 rgba(255, 255, 255, 0.025)` dark): Lifts `.glass-panel` cards subtly above the warm canvas.
- **Control Shadow** (`box-shadow: 0 3px 7px rgba(37, 38, 36, 0.06), inset 0 1px 0 rgba(255, 255, 255, 0.5)` light / `0 3px 7px rgba(0, 0, 0, 0.12)` dark): Gives toggle thumbs, active segmented pills, and URL input fields a crisp tactile edge.
- **Inset Well Shadow** (`box-shadow: inset 0 1px 3px rgba(37, 38, 36, 0.06)` light / `inset 0 1px 3px rgba(0, 0, 0, 0.14)` dark): Recesses setup progress tracks, segmented control troughs, and recessed code/status wells.
- **Footer Scroll Divider** (`box-shadow: 0 -6px 14px -10px color-mix(in srgb, var(--browsify-rail) 25%, transparent)`): Softly separates scrolling page content from the fixed `AppFooter` without obscuring text.

### Named Rules
**The Flat-Inside-Cards Rule.** Never nest bordered, shadowed cards inside another `.glass-panel` card; subdivide content inside a panel using `1px` hairline dividers (`--browsify-border-strong`) or subtle flat inset fills.
**The Calm Motion Rule.** Interactive lifts stay restrained (`translateY(-1px)` on fine-pointer hover, `translateY(1px) scale(0.985)` on active press, `140ms`–`160ms ease`), and every transform and transition collapses cleanly under `prefers-reduced-motion: reduce`.

## Shapes

Every radius in Browsify follows a strict **concentric geometry ladder** so nested elements never clash visually:

- **Outer Frame (`38px` desktop / `28px` mobile):** Smooth hardware-like silhouette framing the entire application.
- **Panels & Cards (`24px`, with `28px` on the full-height `URL tester` container):** Generous, calm card containers with `1px` subtle highlight borders (`--browsify-border`).
- **Controls & Inputs (`14px`):** Text inputs, `<Select>` dropdowns, rule validation banners, history items, and modal dialogs (`18px` for `<dialog>`).
- **Pills & Toggles (`999px`):** All `.app-button` actions, active navigation pills, segmented `AppearanceMenu` options, filter chips, toggle tracks (`44px × 28px`), and compact status pills (`Unsaved`, `Dry run only`).

## Components

Every component is designed to feel **quiet, tactile, and immediately predictable**.

### Buttons
- **Shape:** Full pill (`999px`), minimum `44px` height, `10px 18px` padding, `500` weight `Figtree`.
- **Primary (`.app-button.primary`):** `Terracotta Accent` background with `On Accent` ink (`#ffffff` light / `#301c15` dark), borderless; switches cleanly to `Warm Inset` background + `Slate Muted` text + `--browsify-border-strong` border when disabled.
- **Secondary (`.app-button`):** `Crisp Control` fill, `Charcoal Ink` label, `1px solid Control Stroke` border; hovers to `Warm Inset`.
- **Danger (`.app-button.danger` / `.danger-solid`):** Outlined `Danger Crimson` for reversible actions (`Discard changes`), solid `Danger Crimson` fill strictly for destructive confirmations inside native `<dialog>` modals (`Clear logs`, `Replace config`).
- **Ghost Action (`.ghost-action`):** Borderless `44px × 44px` hit target for rule reordering (`Top`, `↑`, `↓`), `Remove` pattern, and `Delete rule`, revealing a `Warm Inset` pill background on hover.
- **Hover / Focus:** `translateY(-1px)` on fine-pointer hover, `translateY(1px) scale(0.985)` on press, `2px solid Terracotta Ink` focus ring with `3px` offset.

### Cards / Containers
- **Corner Style:** `24px` radius (`--browsify-radius-panel`).
- **Background:** `Porcelain Panel` with subtle top-left directional sheen (`--browsify-panel-gradient`), or `Stone Surface` for recessed home containers.
- **Shadow Strategy:** `Panel Shadow` + `1px solid --browsify-border`.
- **Internal Padding:** `18px`–`20px` standard (`10px 16px` for collapsed Handler Rule summary rows).
- **Disconnected Recovery Card:** Single calm `.glass-panel` across `/rules`, `/test`, `/troubleshoot`, and `/preferences` stating `Browsify is not connected`, concise reopen steps (`Applications`, `Spotlight`, or `browsify ui`), and a single primary `Retry connection` button.

### Inputs & Selects
- **Style:** `44px` minimum height, `14px` radius, `Crisp Control` fill, `1px solid Control Stroke`, `10px 14px` padding (`36px` right padding on custom-chevron `<Select>`).
- **Focus:** `2px solid Terracotta Ink` outline with `2px` offset.
- **Error / Disabled:** Validation error shifts border to `Danger Crimson`; warning states (e.g. `Unavailable Destination`) shift border to `Amber Warning`; disabled inputs recede to `0.65` opacity with softened `--browsify-border-strong` border.

### Toggle Switches
- **Geometry:** `44px × 28px` pill track (`999px`) with a `20px` circular thumb sliding `16px` (`160ms ease`).
- **States:** Off uses `Switch Track` (`#767872` / `#262722`) with `1px` switch border; On switches to `Terracotta Accent` with `On Accent` thumb. Never stack opacity on both a label wrapper and the `<Toggle disabled>` control.

### Navigation Rail
- **Desktop (`84px` wide capsule):** `Graphite Rail` vertical gradient (`#666762` → `#3c3d39` light / `#34352f` → `#171814` dark), `42px` capsule radius, brand icon (`48px`) at top (`Home`), stacked icon+label items (`Rules`, `Test URL`, `Activity`, flex spacer, `Preferences`, `About`), and a terracotta error count badge on `Activity` when errors exist. The active route highlights its `40px` circular icon disc in `--browsify-control-solid` with `Terracotta Ink` (`#b94027`) icon and `#ffffff` label.
- **Mobile (`≤ 700px`):** Full-width `22px`-radius top bar housing a 6-column equal-width grid (`min-height: 48px`, `16px` active pill radius) with `--browsify-rail-active-bg` (`#ffffff` light / `#3f4039` dark with `#9a9c91` border) and `Terracotta Ink` (`#b94027` light / `#ff9c7f` dark) text and icon.

### Native Dialogs (`DraftDialog`)
- **Element:** Native HTML `<dialog>` using `showModal()`, `440px` max width, `18px` radius, `Porcelain Panel` fill, `Backdrop` scrim (`rgba(18, 19, 16, 0.58)` light / `rgba(10, 11, 9, 0.78)` dark).
- **Accessibility:** Explicit `aria-labelledby`, `aria-describedby`, `Escape` dismissal, and deterministic focus restoration to the triggering control or `<main>` after destructive confirms (`Clear logs`, `Replace config`).

### URL Tester (`UrlTester`)
- **Dual Modes:** Compact card (`356px` height, pill URL input + circular submit button) on Home; full diagnostic workbench (optional `Opener` preset/custom bundle ID selector, rule-match attribution linking to Handler Rule `#N`, rewritten final URL readout, and clickable `Recent tests` history) on `/test`.
- **Safety:** Always labeled `Dry run only`; shows an inline `Save & test` notice whenever `Config Draft` has unsaved edits and guards rule-index deep links when draft indices may have shifted.

### Browser Target Picker (`BrowserTargetPicker`)
- **Layout:** Paired `Browser Target` and `Profile` / `Workspace` `<Select>` fields with inline custom-target input fallback, clear `×` button, and `Unavailable Destination` amber warning when a configured browser or profile is missing on the current Mac. Inside expanded Handler Rules, URL patterns always sit above `BrowserTargetPicker` to match the mental model (`URL pattern(s) → Browser Target`).

## Do's and Don'ts

### Do:
- **Do** preserve the pinned `1080px × 760px` desktop `.app-frame` dimensions and `84px` graphite capsule navigation rail so Browsify keeps its focused desktop-instrument proportions.
- **Do** use `Control Stroke` (`#80827a` light / `#8e9085` dark) on every interactive border and `Terracotta Ink` (`#b94027` light / `#ff9c7f` dark) for links, active rail labels, and `2px` focus rings.
- **Do** keep `44px` minimum height on every button, input, select, link action, and interactive list item, and preserve `prefers-reduced-motion` parity.
- **Do** use canonical Browsify domain terms (`Primary Config`, `Config Draft`, `Handler Rule`, `Rewrite Rule`, `Browser Target`, `Opener`, `Unavailable Destination`) across all labels and feedback copy.
- **Do** group immediate macOS/window controls (`This Mac & app`) separately from draft-staged `Primary Config` controls so users always know whether `Save changes` is required.
- **Do** render a single calm disconnected `.glass-panel` with reopen steps (`Applications`, `Spotlight`, or `browsify ui`) and one primary `Retry connection` button on offline subpages.

### Don't:
- **Don't** add tiny uppercase eyebrow/kicker labels above page or section headings.
- **Don't** nest bordered or shadowed cards inside `.glass-panel` containers; use `1px` dividers (`--browsify-border-strong`) instead.
- **Don't** apply `opacity` to a parent label around a disabled `<Toggle>` or `<button>`, which compounds dimming and breaks WCAG AA text contrast.
- **Don't** use a bottom `mask-image` fade on scrollable containers that slices the last visible line of text on mobile viewports.
- **Don't** hijack macOS `Option+Arrow` text navigation inside `<input>`, `<select>`, or `<textarea>` fields for keyboard rule reordering.
- **Don't** introduce secondary brand accents, saturated hero gradients, or marketing copy inside the desktop utility workspace.
