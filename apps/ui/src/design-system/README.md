# Browsify design system

Porcelain surfaces, a charcoal rail and a coral accent, adapted from the supplied
desktop concept. Sora is the display face; Figtree is the interface face. Fonts
and full-color browser marks are local assets with licenses in `public/`.

## App frame

The desktop frame is **1080 × 760 CSS pixels**, centered in the browser tab. It
does not grow with a larger display. On smaller viewports its maximum dimensions
are constrained to the available space; the workspace scrolls internally and the
navigation remains available. This does not create or resize a native window:
Browsify keeps the on-demand browser UI from ADR 0002.

The Browsify logo opens Home. The compact labeled rail links to Rules, Test URL,
Activity, Preferences and About; Home does not duplicate these navigation links.
Rules, Preferences and Import share `BrowserTargetPicker.svelte` (backed by `Select.svelte`) for Browser Targets, Profile Selectors, Unavailable Destination warnings, and Dia/Zen boundary disclosures. Preferences separates immediate macOS integration (`Default link handler`, `Open at Login`) from Primary Config behavior toggles.

## Boundaries

- StyleX owns semantic tokens and shared visuals in `tokens.stylex.ts`,
  `styles.ts` and `navigation.ts`. Svelte uses `stylex.attrs()`.
- StyleX definitions stay outside `.svelte` files; theme definitions use the
  `.stylex.ts` suffix. Light, dark and system modes share the same tokens.
- Local components wrap native controls. No headless component library is needed.
- `app.css` bridges existing screen styles onto the same semantic tokens.
- Keep surfaces neutral and shadows subtle. State, focus and primary actions use
  coral. Keyboard focus is immediate; reduced motion and transparency are honored.

## Data and interactions

Home uses live routing counts, verified installed browsers and browser targets.
The setup meter has two actual checks: default link-handler registration and a
default browser target. Handler rules are optional, not a setup quota. There are
no invented activity charts, time filters or AI capabilities.

`UrlTester.svelte` serves both Home and the full tester. Tests explicitly
submit to the existing `testUrl` RPC, never open a URL, and display loading,
validation, backend errors and connection failures.

Preferences and Handler Rules share one TOML configuration and one Config Draft.
Tabs in the same browser profile share unsaved edits. Save changes activates
the configuration after conflict resolution. Reset to defaults clears explicit
preferences from the draft while preserving Handler Rules.

Automatic updates check daily and download signed releases in the background;
installation strictly requires explicit **Restart to Update** approval, with
support for **Skipped Release** and 24-hour **Deferred Update**. Manual checks
are always available even with automatic updates off, bypass the daily cache,
and report network failures rather than claiming the app is up to date.
Configuration paths and runtime details are not displayed in the interface.

Run `npm run check`, `npm run test` (Node 24.22.1), and `npm run build` from
`apps/ui`.
