<div align="center">
  <img src="assets/brand/browsify-logo.svg" alt="Browsify" width="96" height="96" />
  <h1>Browsify</h1>
  <p><strong>The right browser. The right profile. Every link.</strong></p>
  <p>A browser and profile router for macOS, built in Rust.</p>
  <p>
    <a href="https://github.com/snowaxe/browsify/releases">Download</a> ·
    <a href="#getting-started">Getting started</a> ·
    <a href="#support">Support</a> ·
    <a href="LICENSE">MIT License</a>
  </p>
</div>

## Why Browsify?

Keep work links in your work profile, personal links in your personal browser,
and meeting links where you want them. Set Browsify as your default browser and
it routes links opened from other apps using your rules.

- **Browser and profile routing** — choose a browser, a Chromium or Firefox profile,
  or a Firefox container for matching links.
- **URL rewrites** — remove tracking parameters, replace hosts, or force HTTPS
  before choosing a destination.
- **Visual editor or config files** — manage rules in the browser-based UI, or use
  TOML, JavaScript, or TypeScript.
- **Finicky migration** — load existing Finicky configurations or migrate them to
  native TOML. Validate your configuration and test important links when migrating.
- **Rust core** — declarative rules run natively; custom JavaScript functions use
  QuickJS. The local UI server runs on demand rather than staying open while idle.

## Installation

Requires **macOS 12 or later**, on Apple Silicon or Intel.

1. Download `Browsify-universal.tar.gz` from [GitHub Releases](https://github.com/snowaxe/browsify/releases).
2. Extract the archive and move **Browsify.app** to **Applications**.
3. Open Browsify, then follow the setup below.

### Gatekeeper notice

**Current releases are not Apple Developer ID–signed or notarized.** The project
has not yet enrolled in the paid Apple Developer Program, so macOS Gatekeeper may
block the app on first launch.

If you trust the download, follow [Apple’s instructions for opening an app from
an unidentified developer](https://support.apple.com/en-us/102445). Do not disable
Gatekeeper system-wide. If macOS reports malware or that the app will damage your
computer, do not bypass that warning.

Release archives have checksums and separate update-package signatures. These
are **not** Apple signing or notarization, and do not remove Gatekeeper warnings.
See [Support](#support) for the signing and notarization funding goal.

## Getting started

1. Open Browsify’s Preferences and choose your default destination browser.
2. Add a rule for a site you want to open in another browser or profile.
3. Choose **Set as default** in Preferences to let Browsify handle links from other apps.

You can manage rules visually or create a Primary Config at
`~/.config/browsify/config.toml`:

```toml
default_browser = "Safari"

[browsers.work]
name = "Google Chrome"
profile = "Work"

[[rewrite]]
match = "*"
strip_params = ["utm_*", "fbclid", "gclid"]

[[handlers]]
match = ["github.com/myorg/*", "linear.app/*"]
browser = "work"
```

Replace `Work` with an existing profile name and `myorg` with your organization.
Links matching these rules open in your work profile; other links open in Safari.

Browsify also loads `~/.browsify.js`, `~/.browsify.ts`, `~/.finicky.js`, and
`~/.finicky.ts`. For example:

```js
export default {
  defaultBrowser: "Safari",
  handlers: [
    {
      match: "meet.google.com/*",
      browser: { name: "Google Chrome", profile: "Work" },
    },
  ],
};
```

Primary Config handler rules run before rules saved by the visual editor. Explicit
settings chosen in the UI override corresponding Primary Config settings without
rewriting your file.

### Command line

The executable is inside the app bundle. To use the commands below without
modifying your system `PATH`, define a shell function:

```sh
browsify() { /Applications/Browsify.app/Contents/MacOS/browsify "$@"; }

browsify list-browsers
browsify list-profiles
browsify validate
browsify test 'https://github.com/myorg/repo?utm_source=slack'
browsify ui
```

`test` resolves a link without opening a browser. Use `browsify open '<URL>'` to
launch the resolved destination, or `browsify --help` for all options.

## Updates

Automatic Updates is enabled by default. While running, Browsify checks for stable
releases at most once daily and downloads verified update packages. Installation
requires choosing **Restart to Update**; an ordinary quit never installs an update.
You can skip a version, postpone the prompt, or disable automatic checks and
downloads in Preferences. **Check for Updates…** remains available manually.

Updates require an embedded verification key and a writable installed app bundle;
there is no unsigned fallback or administrator authorization. Older check-only
versions need a one-time manual upgrade. To update manually, quit Browsify,
replace the installed app with the downloaded version, and reopen it.

## Development

On macOS, install Rust, Node.js 22, `jq`, and Xcode command line tools. From the
repository root:

```sh
./scripts/install.sh
./scripts/build.sh
```

The app is built at `apps/browsify/build/Browsify.app`. Local builds are not
Developer ID–signed or notarized either.

Run checks:

```sh
cargo test --workspace --locked
(cd apps/browsify-ui && npm run check && npm test)
```

For universal builds, release publishing, and update signing, see the
[release guide](docs/releases.md). Contributions are welcome: open an issue to
discuss a change or send a focused pull request with tests where appropriate.

## Support

Browsify is free and open source. The immediate funding goal is to cover the
**Apple Developer Program membership: US$99 per year**, with regional pricing
where applicable ([Apple’s pricing](https://developer.apple.com/programs/enroll/)).

Membership would allow the maintainer to set up Developer ID signing and Apple
notarization, making installation easier. Enrollment alone does not sign or
notarize the app; that release workflow still needs to be implemented.

A donation link is not available yet. Financial support will be optional and will
not unlock features. Until then, bug reports, contributions, and sharing the
project are welcome ways to help.

## Credits and license

Inspired by and based on [Finicky](https://github.com/johnste/finicky), created by
[John Sterling](https://github.com/johnste). Rust rewrite and Browsify development
by [snowaxe](https://github.com/snowaxe).

Licensed under the [MIT License](LICENSE).
