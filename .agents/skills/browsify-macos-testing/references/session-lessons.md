# Lessons from the Browsify macOS regression session

These are observations and limits, not permanent assumptions about macOS or particular browser versions.

## Temporary HOME leaked into GUI launches

The test fixture changed `HOME` to isolate Browsify configuration and Chromium discovery metadata, then launched installed Chrome and Edge through `open`. The user saw storage/Keychain dialogs offering Reset to Defaults and cancelled them. A missing normal login keychain under the inherited temporary home was the likely cause; no private keychain inspection was performed.

**Correction:** synthetic resolver/configuration fixtures must not supply the environment of live GUI launches. Isolate browser profiles using supported browser arguments while preserving the real home. Cancel unexpected security prompts; do not reset or bypass Keychain to make tests pass. Do not claim the prompts proved password loss.

## Default-handler requests were mistaken for effective changes

Some requests returned success/nil errors while effective HTTP/HTTPS ownership fell back from Finicky to Safari. Other requests reported permission/cancellation errors. Background runs and repeated switch/restoration attempts complicated supervision.

An invalid temporary bundle signature was found, but signing a later bundle did not resolve the default-switch failure. The session did not establish a definitive root cause.

**Correction:** inspect daemon startup side effects; get approval before any live default mutation; make foreground, sequential requests; verify both effective handlers independently. Stop on the first failed/unexpected switch. Do not keep retrying or infer success from return codes. Preserve the test app until restoration is verified.

The user eventually restored Finicky manually. Both HTTP and HTTPS were independently verified as Finicky afterward. Default-browser interception remained **blocked**, not passed.

## Marker delivery was initially insufficient evidence

Early requests reached the local marker server through other browsers while Browsify was not the effective default. Those requests were not valid evidence of Browsify interception.

**Correction:** prove handler ownership before using the system URL opener, then verify the actual isolated browser/profile and delivered URL. Keep CLI launch, directed AppleEvent delivery, and system-default interception results separate.

## Cold launches did not prove repeat handoff

Later checks verified Chrome and Edge's actual temporary profile paths through DevTools, and Zen initialized an explicit temporary profile and loaded its local marker page. A second URL aimed at an already-running Chrome instance did not arrive during the bounded check. The session did not establish why.

**Correction:** include repeat-open checks and report them independently. Do not collapse a failed running-instance handoff into an overall profile-launch pass or declare a launcher root cause without reproduction.

## Useful proof to retain

Six real CLI/HTTP regression tests failed against an isolated pre-fix source snapshot for the intended reasons and passed after the fixes. They covered traversal, token disclosure, fragmented requests, invalid/oversized framing, preference preservation, and custom UI Rules File persistence.

Actual Rules-page immediate and debounced saves preserved preferences. A real-browser deep-link refresh without the query token also succeeded. These live checks supplemented, rather than replaced, durable regression tests.

**Correction:** keep pre-fix regression proof, test real persistence, and close all browser-debugging sockets/children on failure so the QA runner cannot hang after producing its report.
