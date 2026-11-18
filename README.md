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

I juggle enough work accounts that clicking a link felt like playing browser
roulette. [Finicky](https://github.com/johnste/finicky) got me most of the way
there, but it didn’t have a few features I wanted. So I thought, “Why not fork
it and vibe away?” 🤣 Anyway, now there’s a Rust app.

Set Browsify as your default browser and it routes links opened from other apps
using your rules. Keep work links in your work profile, personal links in your
personal browser, and meeting links wherever you want them.

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

New builds require **macOS 13 or later** on an **Apple Silicon Mac**. The last macOS 12-compatible build remains available in GitHub Releases. Intel builds aren’t
maintained. If you need one, [open an issue](https://github.com/dungsnowaxe/browsify/issues).

1. Download `Browsify-macos13-arm64.tar.gz` (or `Browsify-arm64.tar.gz` for the macOS 12 bridge) from [GitHub Releases](https://github.com/snowaxe/browsify/releases).
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

### Dia profiles on macOS

Use `Dia` or `company.thebrowser.dia` with the profile's **exact, unique name**:

```toml
default_browser = "dia_work"

[browsers.dia_work]
name = "Dia"
profile = "Work"
```

Keep Dia running with a browser window open. Browsify discovers profiles through
Dia's native automation interface and creates the tab directly in the selected
profile. macOS may require Automation permission for Browsify to control Dia.
`browsify list-profiles Dia` lists the currently available names.

Missing or duplicate names and automation failures do not fall back to another
profile. Rename your saved selector when you rename a Dia profile. Profile-targeted
launches cannot be combined with custom browser arguments, containers or app mode.
An empty profile retains ordinary browser-default opening.

Dia can share browsing data across profiles; disable that option in Dia when you
need separate logins. Creating a tab selects its profile even with background
opening; background mode only omits Browsify's explicit focus command. Browsify
serializes its own Dia handoffs, but cannot guarantee isolation against profile
changes made concurrently in Dia. A timeout may leave delivery uncertain, so
Browsify does not retry automatically. Cold startup and profile creation are not
part of this transport.

### Zen workspaces on macOS

Use `Zen` or `app.zen-browser.zen` with a `workspace:` Profile Selector naming a
Zen workspace (space) by name or UUID:

```toml
default_browser = "zen_research"

[browsers.zen_research]
name = "Zen"
profile = "workspace:Research"
```

Browsify lowers the selector to Zen's native `--space` flag and resolves it to
the workspace's UUID right before opening, so the stable identity survives
renames of other workspaces. `browsify list-profiles Zen` lists the current
workspaces of your default Zen profile; add `args = ["--profile", "<path>"]` to
target a specific Zen profile directory. A workspace missing at dispatch
suppresses the open instead of falling back: Zen itself would still navigate in
the active workspace. Ambiguous duplicate names are likewise rejected.

A workspace is not itself a cookie boundary: cookies follow the container the
workspace is bound to. Give workspaces separate containers in Zen when you need
separate logins. `workspace:` selectors cannot be combined with `container:`,
custom `--space`/`--new-window`/`--private-window` arguments. Zen's own
per-website space-routing settings can still override where a link opens; a
successful handoff does not override browser-native policy.

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

For macOS builds, release publishing, and update signing, see the
[release guide](docs/releases.md). Contributions are welcome: open an issue to
discuss a change or send a focused pull request with tests where appropriate.

## Support

If Browsify saves you from signing into the wrong account for the tenth time
this week, consider sponsoring it. The first goal is to cover the **US$99/year
[Apple Developer Program membership](https://developer.apple.com/programs/enroll/)**
so I can sign and notarize future releases and make them easier to install on macOS.

GitHub Sponsors is pending approval. I’ll add a link here once it’s live.
Browsify will stay free and open source. In the meantime, a bug report or a link
shared with someone who also has too many browser profiles goes a long way.

## Credits and license

Inspired by and based on [Finicky](https://github.com/johnste/finicky), created by
[John Sterling](https://github.com/johnste). Rust rewrite and Browsify development
by [snowaxe](https://github.com/snowaxe).

Licensed under the [MIT License](LICENSE).
