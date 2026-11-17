# Builds and releases

## Continuous integration

CI runs Rust workspace tests and UI checks/tests on pull requests and pushes to
`main`. The macOS build workflow also produces an unsigned universal app artifact.

## Publish a stable release

1. Update `[workspace.package].version` in `Cargo.toml` and run
   `cargo check --workspace` to refresh the workspace versions in `Cargo.lock`.
2. Commit the version changes and merge to `main` after CI passes.
3. Tag that commit and push the tag (substitute your version):

   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

The Release workflow rejects tags that do not match the Cargo version, runs Rust
and UI checks/tests, builds for Apple Silicon and Intel (macOS 12+), and publishes:

- `Browsify-universal.tar.gz`
- `Browsify-universal.tar.gz.sig` — detached Ed25519 update-package signature
- `SHA256SUMS`

Publishing uses the workflow’s `GITHUB_TOKEN`. Signed updates also require the
repository variable `BROWSIFY_UPDATE_PUBLIC_KEY` and secret
`BROWSIFY_UPDATE_PRIVATE_KEY_BASE64`; see [update signing](update-signing.md).
The app bundle version is stamped from Cargo during each build. Only stable
`vX.Y.Z` tags are supported. To retry a failed run, rerun it in GitHub Actions;
if a release already exists, publishing fails rather than overwriting its assets.

## Build locally

On macOS, install Node.js 22, Rust, `jq`, and Xcode command line tools. From the
repository root:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
./scripts/install.sh
./scripts/release.sh
(cd dist && shasum -a 256 -c SHA256SUMS)
```

This produces the universal app at `apps/browsify/build/Browsify.app` and the
archive and checksums in `dist/`. The local release script does not create the
detached update signature; the publish job does that separately.

Release builds are **not Apple Developer ID–signed or notarized**. Update-package
signing does not replace Apple signing or notarization, so Gatekeeper may block
initial installation.

## Update behavior

Automatic Updates is enabled by default: while Browsify runs, it checks for stable
releases at most once daily and downloads and verifies signed updates. The native
ready dialog offers **Skip This Version**, **Later** (remind after 24 hours), and
**Restart to Update**. Installation never happens on ordinary quit.

Downloaded updates survive quitting. Only writable installed app bundles are
supported, without administrator authorization. Installation keeps the previous
bundle until the replacement acknowledges startup, with rollback on failure.

Disabling Automatic Updates stops background checks, cancels background downloads,
and discards automatically staged packages. Existing `checkForUpdates: false`
(JavaScript/UI Rules File) or `check_for_updates = false` (TOML) preferences stay
off; these compatibility keys control automatic checks **and** downloads.

**Check for Updates…** remains available in Preferences and the menu, bypasses the
daily interval and skipped-release filter, and asks before downloading. Preferences
also provides release notes, manual downloads, update status, and retry controls.

Check-only installations need a one-time manual upgrade to an updater-enabled
release. Builds without an embedded verification key cannot install updates;
there is no unsigned fallback. For a manual upgrade, quit Browsify, replace the
installed app with the downloaded version, and relaunch it.
