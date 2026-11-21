# Local routing verification — 2026-11-18

This report records one Mac's installed builds and bounded checks, not minimum supported versions or the complete acceptance matrix. The initial inspection changed no production code; the subsequent implementation and tests are recorded below.

## Environment and boundaries

- macOS 26.7.1, build 25G241.
- Zen 1.24.2b, bundle version 126.11.11.
- Safari 27.2, bundle version 21625.2.8.18.4.
- Dia 1.57.1, bundle version 90935.
- The existing local debug Browsify executable drove Zen launches using explicit temporary Primary Config and UI Rules File paths. It was not rebuilt, so these results concern that executable, not a newly compiled checkout.
- Preserved the real HOME. Zen used an owned temporary profile. Dia checks used only synthetic tabs in its already-running profile through its scripting interface; no personal tabs, history, credentials or profile stores were enumerated.
- HTTP and HTTPS effective handlers were queried with NSWorkspace before and after: both remained the existing Browsify application. No default change or interception test was performed.

## Results

| Check | Result | Evidence and limit |
| --- | --- | --- |
| Zen released workspace CLI | Passed | Installed executable's `--help` advertises `--space <space> [<url>]`, accepting a name or UUID. Earliest supporting release remains unknown. |
| Zen cold explicit-profile launch | Passed, bounded | Browsify launched a disposable profile; the server received the cold marker with a Firefox user agent and no test cookie. The temporary profile initialized. This tests argument passthrough, not profile-name discovery. |
| Zen warm explicit-profile launch | Passed, bounded | A second launch through Browsify delivered its marker with the cookie set by the first response. |
| Zen nonexistent workspace | **Failed strict-target gate** | A third launch with `--space browsify-nonexistent-space-4b41` delivered its marker with the same synthetic cookie. A missing workspace does not suppress navigation. |
| Zen valid A/B workspace/container selection | Not run | No two-workspace fixture or container binding was established. The cold/warm result must not be presented as workspace isolation proof. |
| Safari dictionary inspection | Completed, bounded | Shipped Safari dictionary has tab/window/document objects but no profile selector. The standard dictionary include also contains no profile vocabulary. No private event conclusion follows. |
| Safari selected-profile navigation | Not run | No demonstrated profile-selecting transport; Shortcuts action catalog and private events were not tested. |
| Dia profile discovery via AppleScript | Passed, bounded | `count profiles of front window` returned 1. No personal profile names or tab contents were read. |
| Dia direct profile-targeted tab creation | Passed, bounded | `make new tab at end of tabs of profile 1 of front window with properties {URL:…}` delivered the marker; returned tab was closed. Server user agent identified Chrome/157. |
| Dia invalid profile index | Passed, bounded | The same command with profile 999 returned AppleScript error -1719 (invalid index); its marker never reached the server. A valid profile control subsequently reached the same server. |
| Dia Chromium CLI/discovery equivalence | Unverified | Did not test `--profile-directory`, read personal Local State, or assume `--user-data-dir` safely isolates Dia's native shell. |
| Dia cross-profile cookie isolation | Not run | Only one profile was exposed. A second synthetic context is necessary to answer this conclusively. |

The local server recorded only synthetic request paths, test cookies and user agents. Zen's three marker paths each arrived once. The cold request had no cookie; warm and nonexistent-space requests both carried `browsify_probe=synthetic`. This demonstrates persistence and missing-target fallback, not the full concurrency or no-leakage matrix.

## Shipped scripting evidence

Resources were read from the installed application bundles without launching Safari or reading browser data stores:

- Safari `Contents/Resources/Safari.sdef`, SHA-256 `a0a05c6e4a37f30e78545c3e715d57facd6217864b252bcd07fbfd579a62b049`.
- Dia `Contents/Resources/Dia.sdef`, SHA-256 `6351b79a9e21380097aba3f83e40dd91106137d21e496f23e3c9cb2ccc57ddb4`.

Dia defines a `profile` class described as a space of tabs, with read-only name and index, profile-contained tabs and a focus command. Windows expose profiles and active profile; tabs support movement to a destination profile. No stable profile ID is exposed by this dictionary. A window ID is not a profile ID. The successful tab-creation command is direct destination selection, unlike focusing a profile then sending a generic URL event.

These are ordinary shell and runtime observations, not REA Evidence records. Attempts to inspect Zen's optimized `omni.ja` with general ZIP tools did not yield usable source; no package-source claim is based on those attempts.

## Implications for the plan

1. Keep strict Zen selectors gated. The missing-workspace hazard is reproduced, not merely hypothetical. Preflight validation cannot close the deletion race.
2. Keep Safari profile routing unpromised. The installed dictionary does not supply the missing transport, but this does not establish impossibility.
3. Add Dia as its own capability investigation. Chromium engine family does not prove Chromium launch, discovery or identity semantics. Its working native scripting surface merits testing before relying on generic Chromium profile flags.
4. Do not require every future selector to have a stable ID before feasibility testing: Dia's exposed name/index surface currently fails that persistence requirement. Determine an authoritative stable identity or keep saved strict selectors unsupported.
5. The next meaningful live test requires two synthetic Dia profiles and two Zen workspaces with known container bindings. Test cookies, wrong-target absence, cold/warm delivery, multiple windows and deletion/reordering independently.

## Cleanup

Closed the Dia tabs created by the probes and expired the synthetic test cookie through a cleanup response. Stopped the owned Zen main process and local servers; confirmed the Zen main process exited. The pre-existing Dia and Browsify processes remained running. Disposable files were removed after recording the results. No security prompt was observed by the scripted probes; no UI-wide prompt inspection was performed. No system settings were changed.

## Zen workspace implementation follow-up (2026-11-18)

Implemented and live-verified native Zen workspace routing through the same
pattern as Dia: a `workspace:` Profile Selector lowered to Zen's native
`--space` flag, with dispatch-time validation that resolves the selector to the
workspace UUID.

Upstream source (pinned `37513f03`) established the semantics: names match
case-insensitively, UUIDs exactly, unknown selectors resolve to null but do not
suppress navigation; a warm `--space` switches the top window's workspace, and
new tabs receive the destination workspace's container immediately. Workspace
metadata persists in `zen-sessions.jsonlz4` (mozLz4 JSON, `spaces` array of
`uuid`/`name`/`containerTabId`) inside the profile directory.

A disposable Zen profile (real HOME, explicit `--profile` temp directory) was
seeded with two workspaces bound to distinct containers. The profile store
round-tripped through Browsify's pure-Rust mozLz4 reader. Live results:

- Direct `open` probes: cold start, warm handoff by name, by UUID and by
  case-variant name all delivered to the correct container cookie jar; a
  12-request alternating concurrent burst delivered every URL exactly once to
  the correct jar.
- The negative gate reproduced the known hazard again: a nonexistent workspace
  still navigated, into the active workspace's container.
- End-to-end through the rebuilt `browsify` CLI (TOML handlers with
  `workspace:` selectors and a custom `--profile` argument): cold, warm
  alternating and a 12-request concurrent burst all delivered to the correct
  jar; the missing-workspace request exited non-zero with the available
  workspaces listed and produced no server hit.
- Discovery on the real installation lists the user's actual workspaces after
  fixing default-profile resolution to prefer the `profiles.ini` install-section
  default over the profile-level `Default=1` marker.

Design decisions: the launch plan carries the raw selector; the launcher
validates and rewrites it to the workspace UUID right before `open` dispatch,
so the stable identity is used even if another workspace is renamed. A missing
or ambiguous workspace suppresses the launch regardless of the configured
missing-profile policy, because Zen navigates anyway. Custom `--space`,
`--new-window`, `--private-window` arguments and `container:` selectors are
rejected in combination. The JS prelude previously truncated multi-colon string
targets at the second colon; it now splits on the first colon only, so
`'Zen:workspace:Research'` keeps the full selector.

Limits: delivery was proven on Zen 1.24.2b with window sync enabled and no
Zen space-routing rules configured; Zen's native per-website space routing can
override the destination, and this implementation does not detect or override
it. Workspaces created moments before a handoff may not yet be persisted in the
session store. Private windows, multiple unsynced windows, browser upgrades and
workspace deletion between validation and dispatch remain unverified. The burst
tests serialized handoffs only through `open` command ordering; Browsify does
not add a Zen handoff lock.

Validation: 21 core tests (two new Zen resolver tests), 14 binary tests, 2 new
Zen CLI integration tests through both static and dynamic JavaScript config
paths, 1 Dia integration test, 19 UI tests, svelte-check clean and the UI build
passed. The six UI-server integration tests remain blocked by the running
daemon's singleton lock, as before.

## Safari profile routing: closed as unsupported

Safari stays without profile-targeted routing. The shipped Safari 27.2
dictionary has no profile vocabulary (re-verified), the bundled Shortcuts
action catalog contains no Safari or URL-profile actions, and Apple documents
external links opening in the last-used profile. No undocumented private event
was found or attempted; an unsupported capability result is the plan's accepted
Phase 0 outcome. Safari remains usable as an untargeted default browser.


## Dia implementation follow-up

The user authorized creating test profiles and implementing support. Created
`Browsify Test A` and `Browsify Test B` in Dia's native Profiles settings with
sharing disabled. Dia's UI explicitly allows selected profiles to share cookies,
logins and other browsing data: a profile label alone is not proof of isolation.

The rebuilt Browsify CLI discovers these names through the running application's
scripting interface. Existing `profile` configuration selects a typed native Dia
launch action; untargeted opening retains the existing application-open path.
No browser stores or credentials are used for discovery. Profile names and URLs
are passed as process arguments to fixed scripts, not interpolated into code.

Observed results:

- Initial direct scripting probes seeded different synthetic cookies in A and B;
  alternating requests retained the correct markers.
- An initial concurrent CLI burst lost one of eight requests to an automation
  error. A larger direct probe reproduced Dia errors -10000 (tab creation), -1728
  (window lookup) and -1719 (invalid window index). No failed request was retried.
- Added an advisory handoff lock shared across Browsify processes, with bounded
  waiting and process timeouts. The subsequent 32-request alternating burst
  delivered every nonce exactly once with its intended A/B cookie marker.
- Missing-profile requests returned a nonzero CLI status and produced no marker
  request. Dry-run described the native action without navigation.
- Dia's remote name filter ignored AppleScript's case-sensitive setting. A
  lowercase selector initially navigated. Replaced it with local comparison of
  returned names; the lowercase selector then returned a nonzero status with no
  marker request, while the correctly cased control succeeded.
- Foreground routing selected the requested profile. Background creation also
  selected the requested profile, even without an explicit focus command. This
  must not be advertised as preserving Dia's previously selected profile. OS
  foreground-application preservation was not established by this check.

Limits: Dia must already be running with an open browser window. Names are
case-sensitive and must be unique; they are not stable IDs. Runtime duplicate-name
rejection is implemented but was not exercised with duplicate live profiles.
Concurrent profile deletion/reordering, multiple windows, macOS permission
revocation and other browser versions remain unverified. Serialization coordinates
Browsify processes, not user actions in Dia. Native acknowledgment is not a general
proof that a website completed loading. No Chromium CLI compatibility conclusion
is made.

Validation:

- The canonical-directory regression failed against the original resolver and
  passed after the native Dia branch was added.
- Rust library/binary suites: 47 tests passed. CLI integration coverage verifies
  native dry-run targets through both static and dynamic JavaScript config paths.
- UI tests: 19 passed. Svelte/TypeScript check: no errors or warnings. UI production
  build passed.
- Full `cargo test --workspace` was attempted. Its six existing UI-server
  integration tests could not acquire the singleton lock held by the user's
  running Browsify daemon. The daemon was not stopped to make those tests pass.
- Existing Cargo manifest warnings were unrelated to this change.

All created marker tabs were closed and test cookies expired. The two named test
profiles remain available for inspection. The user’s original selected profile
was restored after focus testing. Default handlers were not changed. This work
built the local executable; it did not replace or restart the installed app.
