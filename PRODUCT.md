# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Primary users are macOS users who juggle multiple work, personal, or client accounts across different browsers, profiles, containers, or workspaces and want outbound links from other apps to open in the right place automatically.

Browsify serves two equal workflows within this audience:
- **Visual-first users** who want guided setup, a visual rule editor, and a URL tester without needing to learn TOML or the CLI.
- **Power users and Finicky switchers** who want full parity through a single inspectable TOML file, a CLI (`browsify`), and reviewed import of existing Finicky or legacy configurations.

## Product Purpose

Browsify is a browser and profile router for macOS, built in Rust. Set as the macOS default browser, it intercepts outbound links opened from other apps (Openers), runs Rewrite Rules to clean or transform URLs, evaluates ordered Handler Rules, and opens the resolved Browser Target in the right browser, profile, container, or workspace.

Success means eliminating "browser roulette": links consistently land in the intended account without manual copy-pasting, without silent misrouting, and without paying resident memory overhead for a configuration window that is rarely open.

## Positioning

Browsify combines three mechanisms that neighboring macOS link routers (such as Finicky, Velja, or Choosy) do not offer together:
1. **Zero-idle Rust core with an on-demand loopback UI:** Declarative routing runs natively in Rust, and the configuration UI (`apps/ui`) is served on demand over an ephemeral, token-authenticated `127.0.0.1:0` HTTP/SSE server in the user's browser—keeping idle UI memory at 0 bytes instead of hosting a resident `WKWebView` (see `docs/adr/0002-on-demand-loopback-ui-server.md`).
2. **One inspectable Primary Config shared between visual and text workflows:** Visual rule edits, Preferences, and direct text editing all read and write `~/.config/browsify/config.toml` with multi-tab Config Draft sharing and 3-way conflict resolution—no brittle JavaScript scripting required, alongside reviewed import from Finicky (`.finicky.js`/`.ts`/JSON) and legacy multi-file configs (see `docs/adr/0005-single-toml-config.md`).
3. **Strict profile, container, and workspace routing with no silent fallback:** Targets Chromium profiles, Firefox profiles and containers, Dia profiles (via native macOS automation), and Zen workspaces (via `--space` UUID resolution). Unavailable Destinations, missing or duplicate Dia profiles, and missing or ambiguous Zen workspaces are surfaced or suppressed rather than silently falling back to another account.

## Operating Context

- **Daily routing:** Runs in the background on macOS 13+ (Apple Silicon) as the registered default link handler, receiving URLs and Opener metadata from apps such as Slack, Mail, Linear, calendars, and terminals.
- **Desktop configuration UI (`apps/ui`):** Opened on demand from Browsify or `browsify ui` in a browser tab. Covers Home/Start, Rules, Test URL, Activity (logs), Preferences, About, and Import Config. Multiple tabs in the same browser profile share one Config Draft in local storage; saving writes `config.toml` and reloads the router before clearing the draft.
- **CLI (`apps/cli`):** Invoked inside the app bundle (`/Applications/Browsify.app/Contents/MacOS/browsify`) for `list-browsers`, `list-profiles`, `validate`, `test`, `open`, `migrate`, and `ui`.
- **Browser extension (`apps/extension`):** Chrome and Firefox extension for explicitly sending links from inside a browser through Browsify.
- **Public landing site (`apps/landing`):** Static Astro site deployed to Cloudflare Workers introducing Browsify and linking to GitHub Releases, installation notes, and source.

## Capabilities and Constraints

- **Confirmed capabilities:**
  - Outbound link interception as macOS default browser, matching URLs and Openers (app name, bundle ID, path, optional window title).
  - Rewrite Rules (strip tracking parameters, replace hosts, force HTTPS) evaluated before ordered Handler Rules.
  - Browser Target routing: Chromium/Firefox profiles, Firefox containers, Dia profiles, Zen workspaces (`workspace:`), web app mode, custom browser arguments, foreground/background preference, or launch suppression.
  - Live URL testing (`testUrl` RPC and `browsify test`) that resolves routing without opening a browser.
  - Reviewed migration/import from Finicky JS/TS/JSON and legacy Browsify configs using QuickJS inspection with a copyable report of rules needing manual attention.
  - Signed, opt-out Automatic Updates: daily check and verified package download; installation strictly requires explicit **Restart to Update** (an ordinary quit never installs an update), with Skipped Release and 24-hour Deferred Update support.
- **Technical and distribution constraints:**
  - **OS and architecture:** New builds require macOS 13+ on Apple Silicon (`arm64`). Intel builds are not maintained; a legacy macOS 12 bridge build remains in GitHub Releases.
  - **Code signing:** Current releases are **not** Apple Developer ID–signed or notarized. Gatekeeper warnings must be explained accurately (linking to Apple's guidance; never telling users to disable Gatekeeper system-wide or bypass malware warnings). Release checksums and update-package signatures are not Apple notarization.
  - **Single TOML Primary Config:** Live JavaScript routing is superseded by `config.toml` (ADR 0005); JS/TS/JSON are import formats only. Configuration file paths and internal runtime details are not exposed in the desktop UI.
  - **Browser-specific boundaries:** Dia profile routing requires Dia running with a window open and Automation permission, and does not cover cold startup, profile creation, or cookie isolation when Dia shares browsing data. Zen workspace routing does not create a cookie boundary on its own and cannot override Zen's own per-website space-routing policy.
  - **Licensing and funding:** MIT License. Optional GitHub Sponsors funding has an explicit first goal of US$99/year for the Apple Developer Program to enable signing and notarization; sponsorship unlocks no features.
- **Domain terminology (`GLOSSARY.md`):** Use exact established terms—**Opener** (not caller/sender/source app), **Primary Config** (not script/main config), **Config Draft**, **Rewrite Rule** (not redirect/transform rule), **Handler Rule** (not route/matcher rule), **Browser Target** (not browser config/spec), **Profile Selector** (not profile flag/ID), **Unavailable Destination**, **Launch Plan**, **Automatic Updates** (never "automatic installation"), **Skipped Release**, and **Deferred Update**.

## Brand Commitments

- **Name:** Browsify.
- **Canonical mark:** The selected "Route B" solid stencil B icon at `assets/brand/browsify-logo.svg` (monochrome source at `assets/brand/browsify-mark-mono.svg`, rationale in `assets/brand/redesign/README.md`).
- **Voice:** Plain, direct, and honest. Explain real capabilities, prerequisites, and trade-offs clearly without marketing hype or inflated claims.
- **Attribution:** Credit Finicky (created by John Sterling, MIT License), SIL Open Font License for bundled fonts, CC0 SVG Logos for browser marks, and note that browser names and logos are trademarks of their owners.

## Evidence on Hand

- **Brand assets:** `assets/brand/browsify-logo.svg`, `assets/brand/browsify-mark-mono.svg`.
- **Product screenshots:** `docs/screenshots/rule-editor.png`, `docs/screenshots/open-at-login.png`.
- **Browser marks:** Full-color browser icons and attribution in `apps/ui/public/icons/` (reused by `apps/landing`).
- **Live product data (in `apps/ui`):** Real routing rule counts, verified installed browsers and discovered profiles, live setup checks (default link-handler registration and default Browser Target), and real `testUrl` RPC results.
- **Absences that future work must not fabricate:**
  - No customer testimonials, user quotes, user counts, download counts, or press mentions.
  - No benchmark timings or invented performance numbers beyond the documented architectural fact of 0 bytes idle UI memory.
  - No invented activity charts, time-range filters, analytics dashboards, or AI features.
  - No claim of Apple Developer ID signing, notarization, Intel support, or paid tiers unless those facts actually change.

## Product Principles

1. **Never silently open the wrong account.** Unavailable Destinations, missing or duplicate Dia profiles, and missing or ambiguous Zen workspaces fail or suppress visibly rather than falling back to another profile.
2. **Visual-first with full text and CLI parity.** Everyday setup and rule editing work cleanly in the visual UI, while power users get the exact same single TOML Primary Config with safe conflict resolution and CLI tools.
3. **Zero idle overhead and explicit user control.** Keep the UI on demand with no resident WebView, require explicit saves to activate config edits, and require explicit restart approval to install updates.
4. **Plain honesty over hype.** Show only verified system state and real capabilities; disclose Gatekeeper, browser-automation, and platform constraints directly.
5. **Consistent domain language.** Use the vocabulary defined in `GLOSSARY.md` across UI copy, landing pages, docs, and CLI output.

## Accessibility & Inclusion

- Support full keyboard navigation and immediate, visible keyboard focus across interactive controls.
- Honor OS reduced-motion and reduced-transparency preferences (`prefers-reduced-motion`, `prefers-reduced-transparency`).
- Keep UI labels, error messages, and setup instructions plain and self-explanatory so visual-first users never hit unexplained CLI or TOML jargon.
