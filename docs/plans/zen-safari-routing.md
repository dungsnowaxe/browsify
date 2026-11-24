# Draft: Zen workspace/container and Safari profile routing

Research date: 2026-11-17; local verification follow-up: 2026-11-18. Zen workspace routing is now implemented and live-verified (`workspace:` selectors lowering to the native `--space` flag with dispatch-time UUID resolution and fail-closed suppression); Dia has a bounded native-automation implementation. Safari profile targeting closed as unsupported on the evidence gathered; see [local verification results](zen-safari-routing-verification.md) for installed-resource inspection, live checks and the Zen/Safari follow-ups.

## Findings and evidence

### Existing architecture

Read `AGENTS.md`, `docs/agents/domain.md`, `GLOSSARY.md`, and ADRs 0001 and 0002. Reuse the Browser Target → Profile Selector resolution → Launch Plan flow; retain ephemeral QuickJS lowering and the on-demand loopback UI. No ADR conflict is proposed.

- `packages/core/src/browser.rs`: BrowserConfig carries `profile`, optional `container`, custom arguments and URL; Zen is already registered in the Firefox family.
- `packages/core/src/profile.rs`: container selectors become `ext+container:name=…&url=…` before browser-family resolution. Firefox profiles resolve separately. Safari enumeration returns an empty list and profile argument resolution rejects CLI flags. Missing-profile policy defaults to warning and fallback; this is unsafe as the policy for newly introduced explicit isolation targets.
- `apps/cli/src/launcher.rs`: execution currently models macOS `open` with arguments, instance and activation flags. Arguments imply a new instance in the resolver. Neither successful spawning nor process exit proves the selected session received a URL.
- `apps/cli/src/ui_server.rs` and CLI `main.rs` consume profile discovery. `apps/ui/src/components/BrowserProfileSelector.svelte` accepts name-only lists; `types.ts` and URL Tester currently expose profile strings. Structured IDs and capability state need plumbing through these seams.

### Zen: isolation and external targeting are separate

[Zen's workspace manual](https://docs.zen-browser.app/user-manual/workspaces) describes workspace organization. A workspace is not itself a cookie isolation boundary: the relevant native container identity is separate from the workspace and the enclosing Firefox profile. Do not label a workspace “isolated” without establishing its container association.

Strong native-routing evidence exists in upstream development source, pinned at `37513f03c5dcb0eb2520c839e3870b59cdc8aefe`:

- [BrowserContentHandler patch](https://github.com/zen-browser/desktop/blob/37513f03c5dcb0eb2520c839e3870b59cdc8aefe/src/browser/components/BrowserContentHandler-sys-mjs.patch) handles `--space`, switches an existing non-private window, or stores a startup workspace preference. Its help explicitly accepts a workspace name or UUID and optional URL.
- [Workspace command-line tests](https://github.com/zen-browser/desktop/blob/37513f03c5dcb0eb2520c839e3870b59cdc8aefe/src/zen/tests/spaces/browser_zen_workspace_cmdline.js) exercise UUID and case-insensitive name resolution, immediate destination-container selection while switching, initial-tab container selection at startup, and private-window handling. Unknown names leave the current workspace unchanged. These are upstream tests, not a Browsify end-to-end result, and not proof of inclusion in a released build.

Consequently, prioritize native workspace routing with its default container. Do not assume an extension is necessary. Native arbitrary container selection independent of workspace remains unverified; neither the workspace flag nor native container existence proves that capability.

Local verification found a released installation advertising `--space`. Cold and warm launches through Browsify reached a synthetic origin using an explicit disposable Firefox profile. A nonexistent workspace also delivered the URL with the active session's synthetic cookie: strict workspace targeting therefore fails its negative gate on that installation. Do not treat release availability or a successful warm handoff as sufficient to enable strict selectors.

For the existing protocol, [the Open external links in a container author's repository](https://github.com/honsiorovskyi/open-url-in-container) explicitly provides the custom protocol handler and documents the `ext+container:name=…&url=…` form. Its development README also distinguishes stable 1.0.3 from future 2.x and discusses confirmation/signature behavior. This establishes an extension implementation of this syntax, not universal built-in Zen support or compatibility with every installed version. Verify the exact handler, version, profile installation and unknown-container behavior. Do not conflate Mozilla Multi-Account Containers with this separately supplied handler; do not disable confirmation or read signing secrets.

### Safari: profile routing exists; arbitrary external selection is unproven

[Apple's Mac profile guide](https://support.apple.com/en-ie/105100) documents separate cookies and website data beginning with Safari 17. External links ordinarily use the most recently used profile. Safari's “Open Links With Profile” website setting can choose a profile, **unless the website is already open in another profile**. This is a supported user-managed routing feature, but cannot satisfy strict per-request Browsify selection or Opener-based rules. Do not modify those settings or use them as a hidden fallback.

Investigated automation surfaces:

| Surface | Evidence and limitation | Required next evidence |
| --- | --- | --- |
| Apple Events / URL events | Apple documents [kAEGetURL (`GURL`)](https://developer.apple.com/documentation/applicationservices/1578521-anonymous/kaegeturl). A URL event alone establishes no profile-selection parameter. | Inspect versioned Safari scripting resources and demonstrate an event that binds both profile and URL. Any private keyword or UUID parameter is undocumented until Apple documents it; do not invent event codes. |
| AppleScript / scripting dictionary | Apple identifies [the application's dictionary](https://support.apple.com/en-az/guide/script-editor/scpedt1126/mac) as the source of supported commands and objects. No installed Safari dictionary was available for inspection in this research. | Read the shipped Safari dictionary from a clean test macOS installation, record build and resource hash, inspect window/tab/profile objects and open commands. Missing dictionary vocabulary would not prove absence of private events. |
| Shortcuts | Apple supports [running shortcuts from the command line](https://support.apple.com/en-jo/guide/shortcuts-mac/apd455c82f02/mac), including input and output. This does not establish a Safari action accepting a profile and URL. | Inspect the actual action catalog and exported synthetic shortcut on each supported macOS version. A profile-window action followed by generic Open URL must prove it targets that window under concurrency. |
| Accessibility/menu automation | Apple documents creating profile windows through Safari UI in its profile guide; translating that to automation is an unproven proposal. | If considered separately as experimental, verify permissions, localization, window identity, user interference, timeout and cancellation. Global focus switching is not sufficient isolation evidence. |

No claim that Safari external profile targeting is impossible follows from these searches. Undocumented events, profile UUID selectors, URL schemes, and private storage mappings remain hypotheses, not shipping contracts. Avoid reverse-engineering personal history/cookie stores for discovery.

Local inspection of the shipped Safari dictionary and its standard-suite include found no profile-selection vocabulary. This closes the dictionary-inspection task for that installed build only; Shortcuts, private events and live profile delivery remain unverified.

### Dia: distinguish engine family from routing transport

Browsify already registers Dia as Chromium, with `Dia/User Data` discovery and the generic Chromium profile argument path. That is an implementation assumption, not live verification of the directory layout or `--profile-directory` behavior.

The installed Dia scripting dictionary exposes window-scoped profiles, a read-only active profile, profile focus, tabs within a profile, and moving tabs to a profile. It describes a profile as “a space of tabs”; it exposes a name and one-based index, but no stable profile ID. Neither this terminology nor Chromium engine identification establishes cookie isolation or equivalence to Chromium profile directory keys.

Live AppleScript creation of a tab directly within the existing profile delivered a local marker URL. An out-of-range profile index raised an invalid-index error and did not deliver its marker. This is a promising separate transport, not proof of two-profile isolation, cold-start behavior, stable addressing, or race-free rejection. Do not implement focus-then-open or open-then-move as strict routing: navigation may already have occurred in another context.

The follow-up created two non-sharing test profiles and verified distinct cookies, exact-name targeting, missing-name rejection, foreground selection, and a serialized concurrent burst through the rebuilt CLI. The implementation uses a typed Dia action with the existing profile string, exact unique names, an already-open window, and no fallback. It does not claim stable-ID selection, cold startup, Chromium CLI equivalence, or atomic protection against changes made in Dia. Background creation still selects the target profile. See the verification report for the initial failures and fixes.

Context7 was consulted for AppleScript documentation (resolved Apple's archived language guide); general language guidance cannot establish Safari-specific commands. No API implementation syntax is proposed here.

## Phased plan and feasibility gates

### Phase 0 — Establish capability contracts

1. Pin tested Zen release versions to the source above; inspect release source/help without launching user profiles. Verify macOS cold/warm command-line forwarding, explicit Firefox profile selection and workspace/default-container binding. Native workspace routing and arbitrary native container routing are separate capability bits.
2. Build a synthetic metadata inventory: browser installation/version, enclosing profile ID, workspace UUID, container ID, display label and workspace-container relation. Determine authoritative metadata from upstream source before defining a parser. Prefer a narrow read-only metadata surface; never enumerate tabs, history, cookies or credentials. Permission failure means discovery unavailable, not no profiles.
3. For Safari, inspect shipped dictionary/action resources and test only documented candidates first. For Dia, test its shipped profile-targeted scripting surface independently of the existing Chromium-family branch. Record event/action payload, target identifier scope, permissions and success semantics. Undocumented candidates require an explicit experimental decision and a reproducible version matrix.
4. Gate: enable only combinations that prove the URL reaches the requested context. Unknown IDs in Zen can retain the active workspace; preflight alone cannot eliminate deletion races. If the browser cannot reject stale targets atomically, document this limitation and keep strict targeting gated pending an upstream fix or reliable acknowledged transport. Serialization alone does not prevent user interference.

Deliverable: capability matrix with supported/unsupported/unknown states, evidence, minimum tested versions and permission requirements. An unsupported Safari result is a valid phase outcome.

### Phase 1 — Minimal target and resolution changes

Add optional structured selectors without reinterpreting existing `profile` and `container` strings. Keep profile, workspace and container distinct. Proposed selector data: kind, stable ID, enclosing profile ID and display label; final schema follows Phase 0. Stable means stable within its browser/profile scope, not globally or across profile recreation. Names are convenience input: resolve unique matches, reject duplicates (including case-fold collisions), persist ID and display label, refresh labels after rename, and reject deleted IDs. Never silently pick the first match or recreate a missing container.

Preserve existing string configs, container prefixes, custom arguments and existing missing-profile policy. Make all new explicit selectors fail closed regardless of legacy fallback settings. Reject incompatible selector combinations and custom arguments that override the resolved context. An empty selector retains existing browser-default behavior.

Extend the existing resolver with capability-aware branches, not a second routing engine. Reuse argv-based Launch Plans for proven Zen native CLI support. Only add typed launch actions when a proven Safari mechanism requires them: e.g. existing application-open action, typed Apple-event/action payload and explicit suppressed result. Do not encode AppleScript in fake argv or interpolate URLs into shell/script source. Dry-run describes the action without executing it; log target IDs/status without sensitive URL payloads.

### Phase 2 — Config and UI plumbing

Carry selectors through config parsing, JS/TS object conversion and static lowering, dynamic callback results, serialization, UI Rules File, default-browser Settings Override, and rule editing. Inventory all BrowserConfig construction sites before implementation. Keep old discovery consumers working; add structured discovery alongside the name-only endpoint if needed.

Extend the existing selector UI with profile-scoped workspace/container choices and capability reasons. Distinguish not installed, unsupported version, unavailable metadata and permission denied. Disambiguate equal labels. Preserve unavailable saved targets visibly rather than clearing them. URL Tester displays the resolved IDs, transport and suppression reason. Do not create a new resident service or add dependencies without a demonstrated need.

### Phase 3 — Validate and release independently

Ship verified Zen capabilities independently of Safari. Keep unproven transports disabled. Publish tested versions and known restrictions; invalidate capability caches on application upgrades and metadata changes. Unsupported or failed new targets surface an actionable error without reopening in the default context. Do not automatically retry after an uncertain handoff: that can duplicate navigation or leak into another context.

## Acceptance tests

Pure fixture tests cover legacy config round trips, both JS evaluation paths, stable ID resolution, duplicate/Unicode names, renamed/deleted targets, wrong enclosing profile, corrupt/missing metadata, denied access, version gating, URL encoding exactly once, custom argument conflicts, and dry-run non-execution. Existing untargeted Safari and Firefox/container behavior must remain compatible.

Before future live tests, read `.agents/skills/browsify-macos-testing/SKILL.md`. Use a dedicated test account or approved disposable browser data with the real HOME; no personal data, default-handler changes, or temporary-HOME browser launches. The bounded checks in the linked verification report do not satisfy the full acceptance matrix below.

Use a controlled local test origin with synthetic, distinct cookie markers in contexts A and B and a per-request nonce. Assert the server-observed marker and nonce match the requested context, not merely the visible title, active workspace or process exit. Include a no-cookie context; confirm no request reaches the wrong context even briefly.

- Cold launch, warm launch in requested context, warm launch with the other context foreground, multiple windows and private windows.
- Concurrent alternating A/B requests, startup bursts, foreground/background requests and user-driven workspace switching during handoff; every nonce delivered exactly once in the intended context.
- Zen: two workspaces sharing a container, two with different containers, no default container, explicit Firefox profile, absent protocol handler, incompatible handler version, deleted workspace between discovery and dispatch.
- Safari: same website already open in another profile, conflicting website-profile settings, renamed/deleted profiles, permission denial/revocation, action timeout and browser upgrades.
- New target failure: zero navigation in default or last-used session, clear error, no blind retry. If this cannot be guaranteed, the capability does not pass its gate.

## Remaining uncertainties

Earliest released Zen build containing the observed CLI changes; arbitrary native container addressing; authoritative minimal workspace metadata and ID lifetime; warm profile forwarding and concurrent switch ordering; atomic stale-target rejection; Safari dictionary/action availability by build; any undocumented profile-specific event or URL parameter; Safari discovery without private data access; permissions and delivery acknowledgments. Each requires evidence before implementation claims. Browser-native policy may override routing, so a successful handoff is insufficient proof of isolation.
