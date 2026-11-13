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

The compact labeled rail links to Overview, Rules, Test URL, Activity, Settings
and About. Settings owns the browser/profile selector and app behavior toggles.

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

Overview uses live configuration counts, installed browsers and browser targets.
The setup meter has two actual checks: default link-handler registration and a
default browser target. Handler rules are optional, not a setup quota. There are
no invented activity charts, time filters or AI capabilities.

`UrlTester.svelte` serves both Overview and the full tester. Tests explicitly
submit to the existing `testUrl` RPC, never open a URL, and display loading,
validation, backend errors and connection failures. Primary Config keeps managed
settings locked. Editing handler rules preserves the saved app behavior options.

Run `npm run check`, `npm run test` (Node 22.6+), and `npm run build` from
`apps/browsify-ui`.
