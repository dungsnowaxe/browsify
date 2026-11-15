<div align="center">
  <img src="assets/brand/browsify-logo.svg" alt="Browsify Logo" width="128" height="128" />
  <h1>Browsify</h1>
  <p><strong>Always open the right browser &amp; profile — in ~5 MB RAM and 2.3 µs.</strong></p>
  <p>
    <a href="https://github.com/snowaxe/browsify/releases"><img src="https://img.shields.io/badge/macOS-12%2B-blue?style=flat-square&logo=apple&logoColor=white" alt="macOS 12+"></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/built_with-Rust_2021-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust 2021"></a>
    <img src="https://img.shields.io/badge/memory-~5_MB_RSS-brightgreen?style=flat-square" alt="Memory: ~5 MB RSS">
    <img src="https://img.shields.io/badge/latency-2.3_µs-orange?style=flat-square" alt="Latency: 2.3 µs">
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT License"></a>
  </p>
</div>

---

Browsify is an ultra-lean Rust URL router and browser profile launcher for macOS (`com.lavenxsnow.browsify`), engineered as the next-generation, high-performance successor to [Finicky](https://github.com/johnste/finicky).

- **32.5× Lower Memory**: ~4.5–5.7 MB RSS (`0 KB` JS heap when rules are declarative; `~550 KB` dirty anon RAM when using dynamic QuickJS functions) vs. 137–165 MB in Finicky v4.
- **Sub-5 µs Hot Path**: Compile-time hybrid lowering turns declarative rules and `finicky.*` helpers into native Rust matchers (`~2.35 µs` end-to-end) and enters QuickJS (`~42.5 µs`) only for custom JS closures.
- **< 1 µs Cached Profile Resolution**: Typed `serde` parser for 20+ Chromium browsers (`Local State` cached by `(mtime_ns, size)`) and 6+ Firefox-family browsers (`profiles.ini`, Firefox 136+ SQLite Profile Groups, and Multi-Account Containers).
- **Zero-Idle-RAM Visual UI**: Token-authenticated on-demand `127.0.0.1:0` HTTP/SSE UI server that shuts down automatically 30s after the tab closes.
- **100% Finicky v3/v4 Compatibility**: Loads `~/.config/browsify/config.toml`, `~/.browsify.{js,ts}`, `~/.finicky.{js,ts}`, and `rules.json`, and includes `browsify migrate`.

---

## Brand identity

Browsify uses **Route B**: a bold split B on a warm orange tile. Its two channels suggest links opening in separate browser destinations.

- [App icon — SVG](assets/brand/browsify-logo.svg)
- [Monochrome mark — SVG](assets/brand/browsify-mark-mono.svg)
- [Design exploration](assets/brand/redesign/comparison.svg)

The UI favicon and macOS app icon use the same artwork. On macOS, run `./scripts/create-app-icon.sh` to regenerate the app icon from the SVG source.

---

## Quick Start

### Declarative `config.toml` (`~/.config/browsify/config.toml`)

```toml
default_browser = "personal"

[browsers.personal]
name = "Google Chrome"
profile = "Personal"

[browsers.work]
name = "Google Chrome"
profile = "Work"

[[rewrite]]
match = "*"
strip_params = ["utm_*", "fbclid", "gclid"]
force_https = true

[[handlers]]
match = ["github.com/myorg/*", "linear.app/*"]
browser = "work"

[[handlers]]
match_hostnames = ["bsky.app", "reddit.com"]
browser = { name = "Firefox", profile = "default-release", container = "Social" }
```

### Drop-in `.finicky.js` / `.browsify.js`

```js
export default {
  defaultBrowser: "Google Chrome",
  rewrite: [
    finicky.stripParams(["utm_*", "fbclid"]),
    finicky.replaceHost("x.com/*", "xcancel.com"),
  ],
  handlers: [
    {
      match: "bsky.app/*",
      browser: "Firefox",
    },
    {
      match: finicky.matchHostnames(["meet.google.com"]),
      browser: { name: "Google Chrome", profile: "Work" },
    },
  ],
};
```

## CLI Usage

```sh
browsify test https://github.com/myorg/repo?utm_source=slack
browsify open https://github.com/myorg/repo
browsify validate
browsify migrate --check
browsify migrate --to-toml -o ~/.config/browsify/config.toml
browsify list-browsers
browsify list-profiles
browsify ui
```

## Builds and releases

CI runs Rust workspace tests and UI checks/tests on pull requests and pushes to
`main`. The macOS build workflow also produces an unsigned universal app artifact.

To publish a stable release:

1. Update `[workspace.package].version` in `Cargo.toml` and run
   `cargo check --workspace` to refresh the workspace versions in `Cargo.lock`.
2. Commit the version changes and merge to `main` after CI passes.
3. Tag that commit and push the tag (substitute your version):

   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

The Release workflow rejects tags that do not match the Cargo version, runs Rust
and UI checks/tests, builds for Apple Silicon and Intel (macOS 12+), and publishes
`Browsify-universal.tar.gz` and `SHA256SUMS` to GitHub Releases. No additional
secrets are required: publishing uses the workflow's `GITHUB_TOKEN`. The app bundle
version is stamped from Cargo during each build. Only stable `vX.Y.Z` tags are
supported. To retry a failed run, rerun it in GitHub Actions; if a release already
exists, publishing fails rather than overwriting its assets.

For a local release build on macOS, install Node.js 22, Rust, `jq`, and Xcode command
line tools, then run:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
./scripts/install.sh
./scripts/release.sh
(cd dist && shasum -a 256 -c SHA256SUMS)
```

Release builds are **unsigned and not notarized**, so Gatekeeper may block them.
Download and extract the archive, quit Browsify, replace the installed app, then
relaunch it. Developer ID signing and Apple notarization are not configured yet.

### Update support

Browsify can check the latest GitHub Release when configuration is loaded or
updated, with results cached for 24 hours. Enable **Check for updates** in
Preferences to receive release information and a link to the release page.
It does **not** download, install, or restart into new versions automatically.

## Credits & License

MIT License.

- Rust rewrite, hybrid architecture & Browsify rebranding by [@snowaxe](https://github.com/snowaxe)
- Based on [Finicky](https://github.com/johnste/finicky), originally created by [John Sterling](https://github.com/johnste)
- Original Finicky icon designed by [@uetchy](https://github.com/uetchy)
