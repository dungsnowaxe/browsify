---
name: browsify-macos-testing
description: Safely test Browsify on macOS, including browser profile launching, default-browser interception, the loopback UI, and security regressions. Use before live browser/daemon tests, LaunchServices changes, or tests that might touch user profiles or Keychain. Prevent temporary-HOME keychain prompts, unverified default switches, misleading smoke-test passes, and unsafe cleanup.
compatibility: Live checks require macOS and installed test browsers. Automated regression checks use the repository's Rust and UI toolchains.
---

# Safe Browsify testing on macOS

## Read first

Read the repository's `AGENTS.md`, `docs/agents/domain.md`, `GLOSSARY.md`, and relevant ADRs. Load `test-audit` before adding or changing regression tests. Read [session lessons](references/session-lessons.md) before live testing.

Discover the current CLI, launcher, daemon startup behavior, test commands, and available tools. Do not assume commands or flags still match this document. Use current library documentation when needed. No new dependencies are required merely to follow this skill.

This skill grants no permission to change defaults, inspect personal data, install browsers, or click system security dialogs.

## Non-negotiable safety rules

1. **Never launch a real browser with a temporary or synthetic `HOME`.** Preserve the user's real `HOME` throughout the GUI launch process tree, including `open` and LaunchServices helpers. Changing it can make browsers unable to find the login keychain and trigger “Keychain Not Found” / “Reset to Defaults” prompts.
2. Isolate browser data using explicit temporary profile directories, not a fake OS home. Do not read or modify personal browser stores, Keychain, credentials, auth state, or `.env` values.
3. Synthetic homes are appropriate only for non-launching tests: e.g. a `ProfileResolver::new(fixture_home)` fixture, CLI dry runs, or the loopback UI with `--no-open` and no launch/default-changing RPCs. Do not reuse that environment for live launches.
4. Never click **Reset to Defaults**, enter a login password, unlock a keychain, or weaken system security to complete a test. On an unexpected prompt, stop new launches, explain it, and ask the user to cancel. Do not infer data loss from a prompt alone, or guarantee no effects if the user accepted it.
5. Do not start the daemon casually: startup may request default-browser changes even without `--window`. Inspect the current implementation first.
6. A failed or cancelled default switch is a stop condition, not permission to retry repeatedly. Record effective state, restore if authorized, and ask for manual help if necessary.

## Plan and preflight

- Separate automated regressions, dry-run resolution, real profile launches, directed AppleEvent delivery, and system-default interception. Each proves a different contract.
- Explain the proposed live checks, expected windows/prompts, temporary files, system changes, and restoration plan before starting.
- Use explicit temporary Primary Config and UI Rules File paths. Avoid loading personal configuration accidentally. Confirm how the current CLI selects or skips Primary Config.
- Record the original effective application for **both HTTP and HTTPS**, including bundle identifier and application path, using valid sample URLs. Do not inspect private LaunchServices preference files.
- Identify existing browser/daemon processes without terminating them. Reuse or stop none unless explicitly authorized. If a singleton lock blocks a test, report it rather than killing the user's daemon.
- Create a unique, owned test directory; record every child PID, profile directory, application bundle, and registration created. Set bounded startup, network, RPC, and subprocess timeouts.
- Keep machine-specific paths and installed versions out of committed instructions. Use installed tools or mise without changing global version settings unnecessarily.

## Automated regression proof first

Use the strongest owning boundary without adding production seams solely for tests. Prefer the real CLI/HTTP boundary for transport, asset serving, and UI Rules File persistence.

- Run new regressions against pre-fix code and record the intended failures. Use an isolated source snapshot if necessary; do not swap source files underneath an active test run.
- Keep fixtures independent of developer build output, personal rules, browser stores, and network services. Use only temporary marker files for local-file exposure tests.
- Cover complete headers arriving before any body, split headers/terminators, split UTF-8, and payloads larger than one read. A header-fragment test alone does not prove the separate-body regression.
- For persistence, verify actual saved files and effective routing; distinguish default and custom UI Rules Files and assert unrelated files are unchanged. Verify omitted options preserve preferences while explicit changes still work.
- For security negatives, ensure the intended guard rejects the request, not an unrelated authentication or routing failure. Also exercise a valid control.
- Use repository-supported suites and formatting/type/build gates. Report actual outcomes, including pre-existing warnings and skipped checks. Wire durable tests into existing CI where appropriate.

Typical commands, after confirming they remain supported:

```sh
cargo test --workspace
npm --prefix apps/ui test
npm --prefix apps/ui run check
npm --prefix apps/ui run build
git diff --check
```

## Real browser profile launches

1. Start a local marker server bound to loopback with unique URLs for each browser and launch attempt. Keep it alive through all checks. Log only test paths and the minimum browser identification needed.
2. Create temporary Chromium user-data directories and Firefox/Zen explicit profile directories. Use supported profile-isolation and no-first-run/no-default-check arguments; look up syntax rather than guessing. Do not disable TLS, sandboxing, Keychain, or OS privacy protections to suppress a failure.
3. Keep the real `HOME` in the launching environment. Browser-profile isolation is not an OS security boundary; system integration prompts are still possible.
4. Distinguish **Profile Selector resolution** from **argument passthrough**. With a real-home resolver, a newly invented display name may not exist in discovery metadata. Never solve that by changing GUI `HOME` or editing personal metadata. Test synthetic discovery with an explicitly constructed resolver; label live tests using custom profile arguments as passthrough, not end-to-end discovery. If the CLI lacks a safe discovery override, report that limitation.
5. Execute through Browsify's real launcher, not an unrelated direct browser command. A dry-run Launch Plan is useful but is not launch proof.
6. Independently verify both the delivered URL and actual profile. For an isolated Chromium instance, use its temporary DevTools endpoint and `chrome://version` profile path (including Edge), or equivalent browser-owned evidence. For Zen, verify the requested temporary profile was initialized and the marker page arrived in that browser. Never attach to a personal browser's debugger.
7. Test a fresh launch **and a second URL while that instance is already running**. Also test a second isolated profile when relevant. A successful cold launch does not establish handoff behavior.
8. Allow for bounded startup delays; do not silently retry until a flaky check passes. Inspect only the test instance's page/error state to distinguish routing, navigation, startup, and permission failures.

An `open` exit code, launch arguments, initialized directory, or matching URL tab alone is not full proof that the intended browser/profile loaded the page. Report which evidence was obtained.

## Default-browser interception: explicit, supervised, reversible

Only perform this phase when the user has approved temporary system-default changes and can respond to macOS prompts. If unavailable, mark it blocked and continue non-mutating checks.

1. Preserve a restoration record outside any directory that cleanup might remove.
2. Build and verify the test application bundle. Validate its signature and registration before requesting a switch. Signing success does **not** establish handler selection or fix every permission failure.
3. Run interactive changes in the foreground with visible progress. Do not hide prompts behind background jobs or redirect all status output. Make one change at a time; never race HTTP/HTTPS requests or restoration with selection.
4. Wait for completion and independently query the effective application for HTTP **and** HTTPS. Treat an API success/nil error as an acknowledgement, not proof. Compare canonical application paths and bundle identifiers; check for unintended fallback such as Safari.
5. If either handler is wrong or an unexpected prompt appears, do not send test URLs through the system default. Stop the switch attempt and restore/ask for help. Do not launch links into the user's usual browser and count the resulting marker requests as Browsify success.
6. Only after verifying the switch, invoke the system URL opener **without specifying an application**. Verify delivery through Browsify to the intended isolated browser/profile. Test HTTP and HTTPS separately. If HTTPS is rewritten to a local HTTP marker to avoid TLS setup, disclose that it proves incoming HTTPS interception, not TLS navigation.
7. A directed `open -a Browsify ...`, CLI `browsify open`, dry-run plan, or browser marker response without verified handler ownership is not a system-default interception test.
8. Restore the original handlers before stopping/unregistering/removing the test app. Verify both effective handlers again. Use `finally`/equivalent cleanup, but do not assume it guarantees restoration after cancellation, termination, or OS rejection.
9. If restoration fails, stop automatic changes, tell the user the actual current handler and the original one, request manual restoration, then independently verify. Do not finish with an unnoticed fallback or claim restoration from a successful request alone.

## Real UI checks

Use an isolated browser profile and an explicit temporary UI Rules File. Exercise actual controls rather than calling private functions or only replaying RPC payloads.

For Rules persistence, cover immediate and debounced saves, confirm the changed rule on disk, and assert unrelated preferences survive. For bootstrap authentication, navigate to a deep link and refresh after removing the query token; verify session continuity and that unauthenticated requests do not disclose a usable token. Close test tabs and sockets explicitly, including on assertion failure.

## Cleanup and honest handoff

- Stop only owned test servers, browser instances, and helpers. Close DevTools connections on every exit path. Do not use broad `killall`, application-wide Quit, or delete profiles still in use.
- Restore and verify defaults **before** unregistering/deleting the test bundle. Remove only recorded, owned temporary profiles and fixture files. Do not “clean up” the user's Keychain or browser data.
- Confirm no owned test processes remain. Preserve concise, non-sensitive evidence outside removed fixtures; redact session tokens from reports.
- Separate **passed**, **failed**, **blocked**, and **not run**. Keep regressions and live smoke tests distinct. Report a repeat-open failure even when fresh launches pass; do not bury it in a general “browser tests passed.”
- State final handler ownership, restoration status, popup side effects, and any remaining user action. Do not diagnose an OS failure from an error code alone or assert an unverified root cause.
