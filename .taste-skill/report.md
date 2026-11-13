GATE: OPEN 16 (rounds used up)

# Browsify desktop redesign

## Delivered

Software — a working desktop URL-router interface, not a marketing website.

- Fixed **1080 × 760 CSS-pixel app frame**, centered in the existing browser tab. Smaller viewports constrain the frame; content scrolls inside the workspace. ADR 0002 and the native launcher are unchanged.
- Compact, labeled charcoal rail, as selected by the user: Overview, Rules, Test URL, Activity, Settings and About.
- Reference-led porcelain/gray surfaces, rounded panels and coral actions. The accent is slightly darkened to `#c5492e` for white-label contrast. First dealt software pairing: Sora / Figtree, hosted locally. No marketing hero, shader, scene or scroll-driven moment was added to this utility interface.
- Live handler/rewrite counts, installed browsers and default browser target. The setup meter reports two actual requirements, not an invented goal or rule quota.
- Shared, working URL tester with normalization, validation, loading, browser/profile results, backend errors, connection failures and a no-response timeout. Tests never open a destination. No AI capability is implied.
- Dedicated Settings page, preserved Primary Config locks and browser/profile controls, autosaved behavior settings, keyboard-accessible appearance controls and readable activity/empty states.
- Rule saves preserve behavior options. Custom browser editing no longer resets an empty or partially edited value. Locked settings no longer contain nested buttons, and their explanations work by keyboard.

Voice: technical-dry; exact, sentence-case labels; browser targets and handler rules are named consistently; no invented activity history, customers, AI responses or measured claims.

## Verification

From `apps/browsify-ui`:

- `npm run check`: **0 errors, 0 warnings**.
- `npm run test`: **16 passing URL-normalization tests**.
- `npm run build`: **PASS**.
- `git diff --check`: **PASS**.

Headless Chromium checks used the production bundle and an isolated fixture RPC server. They did not read or modify any real user configuration.

- **33 functional browser assertions passed**: URL validation/results/errors/connection failures; result persistence and clearing; settings saves; custom browsers and profiles; rule creation and pattern saves; preservation of options; Primary Config locks; absence of nested buttons; keyboard tooltips; theme switching and arrow/Home navigation; scroll reset and browser icons.
- **71 responsive assertions passed** across 1024×768, 820×1180, 768×1024, 390×844 and 320×640: frame bounds, no horizontal overflow, internal scrolling, focus ring, controls across all pages, touch targets and footer containment. The 1024×768 overview fits without internal scrolling.
- The 1440×900 check confirms an exact 1080×760 frame and an overview that fits without internal scrolling.
- Screenshots were inspected at all these sizes, plus light/dark desktop, Settings and Rules. Artifacts are in `.taste-skill/shots/`.

### Performance

The skill performance check: **0 FAIL, 0 WARN**, at both 1440 and 390.

Approximately 312 KiB transferred on the uncompressed fixture server; two local font files totaling 44.8 KiB; no WebGL contexts, broken requests or long tasks. CPU-only median FCP/LCP was 88 ms desktop and 68 ms mobile; measured CLS was 0.001 and 0.000. These are local test results, not field-performance claims. Details: `.taste-skill/perf.json`.

### Website-oriented checker results

The full consistency run reports 84 raw FAIL occurrences and 26 WARN occurrences across five widths, with no browser execution errors. These include repeated instances of the website-structure rules listed below and the five waived app-specific cases. The final gate deduplicates these to **16 open findings and five waivers**. It is not reported as passing.

The standalone layout checker also treats the default-target card's internal paragraph as page-level text (`edge-left`); it reports no horizontal page overflow. This is the fifth waiver below.

Two additional full-run probes need qualification:

- `contrast`, 768px, “URL to test”: the crop samples the desktop background below the app, where that offscreen label is clipped by the scroll container. It does not show a visible low-contrast label. The crop is `.taste-skill/consistency/768-06-contrast.jpg`.
- `hover-move`, desktop destination card: the coarse-pointer headless probe does not demonstrate the fine-pointer-only CSS hover lift. The 2px hover treatment remains in the product; actual macOS pointer behavior was not checked.

Copy-lint reports 0 FAIL / 0 WARN on the rendered snapshot, but its marketing-section extraction counts only three words, so that is **not a comprehensive UI-copy audit**. UI copy was reviewed manually. The retained `copy-actions` checker warning asks for “Test URL” in a marketing closing section; this app intentionally has no such section. Public share/canonical metadata is also intentionally absent from the private loopback configuration UI.

The final gate used a bundle-content revision in its local URL because its monorepo-root fingerprint otherwise reused an older result instead of noticing changes under `apps/browsify-ui`. No checker or kit file was changed, and no application content was hidden from checks.

## Five formal waivers

1. **`focus-visible` — `input#dashboard-url`:** the visible focus outline belongs to the enclosing `.url-field:focus-within`. The checker only inspects the input's own outline. Verified by screenshot and responsive assertions.
2. **`stagger` — navigation:** equal-width rail targets and their icon/label centers align. Different label lengths intentionally have different left edges.
3. **`edge-left` — “Managed by Primary Config”:** this is text inside the default-target card's 20px padding, not a page-level paragraph that should start at the workspace edge.
4. **`heading-scale` — browser count:** the 28px figure is a data value deliberately emphasized over the 20px section heading, still below the 40px page title.
5. **`flush` — About/sidebar:** the sidebar and workspace are horizontal panes with a 28px gap, not consecutive vertical marketing sections. Their vertical extents intentionally overlap.

Formal definitions are in `.taste-skill/waivers.json`.

## All 16 open gate findings

The explicit fixed-frame working-app requirement takes precedence over these marketing-page recipes. They remain open, rather than redesigning the app to manufacture a gate pass.

| # | Check | Target / reason retained |
|---|---|---|
| 1 | `hero-fill` | The workspace displays setup, tester and summary together; it is not a full-screen marketing hero. |
| 2 | `cta` | The sidebar is misidentified as a hero with no promotional CTA. The actual URL-test form is functional. |
| 3 | `moment-none` | No decorative scroll moment was added to a fixed-height configuration utility. |
| 4 | `hero-screen` | The app frame is 760px tall, not one browser viewport tall. This is the user's chosen behavior. |
| 5 | `section-height` | The centered frame begins below the viewport top rather than on a marketing screen grid. |
| 6 | `one-section` | `main` contains the app's layout/panes, not a sequence of website sections. |
| 7 | `one-heading` | The dashboard intentionally has several region headings within one app screen. |
| 8 | `container-edges` | Fixed-frame margins at 1440 exceed the website checker's 64–120px range. |
| 9 | `container-edges` | Svelte's `#app` mount and app shell are required, rather than the starter website's direct body children. |
| 10 | `container-edges` | The tester's empty-state heading is centered inside its panel, not aligned to the whole page's left edge. |
| 11 | `container-edges` | The setup heading follows the card's padding, not the outer frame's edge. |
| 12 | `container-edges` | Overview headings align to the workspace after the rail and pane gap, not to the rail's left edge. |
| 13 | `container-edges` | The footer spans the workspace, intentionally excluding the navigation rail. |
| 14 | `row-lines` | Summary cards align to workspace edges, not the outer rail-inclusive frame edges. |
| 15 | `split-pair` | The reference-led two-panel dashboard is not a marketing 12-column split recipe. |
| 16 | `overlap-text` | At 390px, the checker compares the footer with offscreen tester-heading coordinates without respecting the workspace's overflow clip. The heading is not painted behind the footer; scrolling brings the tester into view. |

Complete original messages, targets and widths: `.taste-skill/gate-final.log` and `.taste-skill/gate.json`.

## Assets and limits

- Sora and Figtree: local WOFF2 files; SIL Open Font License texts in `apps/browsify-ui/public/fonts/`.
- Full-color Safari, Chrome, Firefox, Edge, Brave and Opera marks: resolver-fetched local SVGs; sources/licenses/trademarks in `apps/browsify-ui/public/icons/icons-credits.json`. Generic interface glyphs use the existing Lucide family.
- No photographs, stock stand-ins, coined brand names, assumed prices/hours, client-asset slots, contact forms or `.example` contact endpoints were needed. Browsify is the existing product name.
- Displayed counts and profiles in QA screenshots are fixture responses, not invented production statistics. Production UI continues to use the existing RPC/SSE data.
- Native macOS browser discovery, system-default registration, browser launching, authenticated SSE and the WebKit bridge were not end-to-end tested on this Linux host. Their backend architecture was not changed. The ten-second no-response timeout was not separately waited through in the automated run; transport and backend failures were exercised.
- Existing uncommitted StyleX/appearance work was retained. No new runtime dependency or native-window implementation was added.
