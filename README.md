# Browsify

**Always open the right browser & profile — in ~5 MB RAM and 2.3 µs.**

Browsify is an ultra-lean Rust URL router and browser profile launcher for macOS (`com.lavenxsnow.browsify`), engineered as a drop-in successor to [Finicky](https://github.com/johnste/finicky).

- **32.5× Lower Memory**: ~4.5–5.7 MB RSS (`0 KB` JS heap when rules are declarative; `~550 KB` dirty anon RAM when using dynamic QuickJS functions) vs. 137–165 MB in Finicky v4.
- **Sub-5 µs Hot Path**: Compile-time hybrid lowering turns declarative rules and `finicky.*` helpers into native Rust matchers (`~2.35 µs` end-to-end) and enters QuickJS (`~42.5 µs`) only for custom JS closures.
- **< 1 µs Cached Profile Resolution**: Typed `serde` parser for 20+ Chromium browsers (`Local State` cached by `(mtime_ns, size)`) and 6+ Firefox-family browsers (`profiles.ini`, Firefox 136+ SQLite Profile Groups, and Multi-Account Containers).
- **Zero-Idle-RAM Visual UI**: Token-authenticated on-demand `127.0.0.1:0` HTTP/SSE UI server that shuts down automatically 30s after the tab closes.
- **100% Finicky v3/v4 Compatibility**: Loads `~/.config/browsify/config.toml`, `~/.browsify.{js,ts}`, `~/.finicky.{js,ts}`, and `rules.json`, and includes `browsify migrate`.

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

## Credits & License

MIT License.

- Rust rewrite & hybrid architecture by [@snowaxe](https://github.com/snowaxe)
- Based on [Finicky](https://github.com/johnste/finicky), originally created by [John Sterling](https://github.com/johnste)
- Original icon designed by [@uetchy](https://github.com/uetchy)
