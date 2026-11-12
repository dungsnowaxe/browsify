software — Browsify desktop URL-router interface, not a marketing website.
Reference-led palette: porcelain #f5f5f3, soft gray #eaeae7, charcoal #252624, coral #c5492e (darkened slightly from the reference for accessible white labels); the attached concept explicitly supplies the colors, rounded surfaces, compact rail and split workspace. Sora / Figtree is the first software type deal. Fixed 1080 × 760 app frame as confirmed by the user; only constrained when the browser viewport is smaller. No native window changes (ADR 0002).

## Facts and voice
- Browsify routes URLs to browser targets, profiles and containers on macOS.
- The existing authenticated loopback RPC supplies configuration, handler/rewrite counts, installed browsers, URL-test results and logs.
- Primary Config takes precedence over the UI Rules File. Managed settings remain locked.
- Main action: Test URL. Existing actions: edit handler rules, set the default browser, change app behavior, inspect logs.
- Voice: technical-dry; exact and approachable; sentence-case labels; no invented usage history, targets, progress, AI capabilities or success states.
- The setup meter reports only two real requirements: OS link-handler registration and a configured default browser target. Handler rules are optional.

## Screens and real material
- Overview: two-column routing setup / real URL tester; four quick actions; bottom row shows real handler/rewrite counts, detected browsers and default browser target.
- Settings: default URL-handler registration, default browser/profile selector and all existing background/maintenance toggles.
- Rules, URL tester, Activity and About: same shell, tokens, typography and compact navigation; long content scrolls inside the workspace.
- Real browser marks are downloaded locally with the skill icon resolver. No photographs or client-asset stand-ins are relevant.

## Interaction and constraints
- Buttons step on hover and press; linked cards lift 2px; immediate visible keyboard focus; reduced motion and reduced transparency are respected.
- A utility interface does not get marketing hero sections, footers, scroll-driven moments, shader effects or decorative charts. Those skill recipes conflict with the explicitly requested fixed-height working app.
- The URL tester submits to the existing testUrl RPC; it must display returned errors, loading, timeout and empty states honestly. It never opens the tested URL.
- Preserve existing uncommitted StyleX integration and all routing/save/lock behavior. No added runtime dependencies.

## Checks
- Svelte/TypeScript checks and production build.
- Pure URL-normalization tests and browser interaction tests with isolated fixture responses (no user configuration read or modified).
- Skill screenshot/layout/consistency/performance/gate checks on the built application, including compact viewports; report website-only mismatches rather than changing the product to satisfy them.
