use crate::browser::{BrowserConfig, ProfileSelector};
use crate::profile::MissingProfilePolicy;
use crate::rules::{Rule, RulesFile, TomlBrowserTarget, TomlConfig};
use crate::wildcard::{compile_wildcard, match_wildcard};
use crate::OpenerInfo;
use base64::Engine;
use regex::Regex;
use url::Url;

#[derive(Debug, Clone)]
pub enum CompiledMatcher {
    Wildcard {
        raw: String,
        regex: Option<Regex>,
    },
    Regex(Regex),
    Hostnames(Vec<HostnamePattern>),
    OpenerBundleId(Vec<String>),
    OpenerWindowTitle(Vec<String>),
    Any(Vec<CompiledMatcher>),
    All(Vec<CompiledMatcher>),
    Always,
}

#[derive(Debug, Clone)]
pub enum HostnamePattern {
    Exact(String),
    Regex(Regex),
}

impl CompiledMatcher {
    pub fn from_wildcard(pattern: impl Into<String>) -> Self {
        let raw = pattern.into();
        let regex = if raw.contains('*') {
            compile_wildcard(&raw)
        } else {
            None
        };
        CompiledMatcher::Wildcard { raw, regex }
    }

    pub fn uses_window_title(&self) -> bool {
        match self {
            CompiledMatcher::OpenerWindowTitle(_) => true,
            CompiledMatcher::Any(list) | CompiledMatcher::All(list) => {
                list.iter().any(|m| m.uses_window_title())
            }
            _ => false,
        }
    }

    pub fn matches(&self, url: &Url, url_href: &str, opener: Option<&OpenerInfo>) -> bool {
        match self {
            CompiledMatcher::Always => true,
            CompiledMatcher::Wildcard { raw, regex } => {
                if raw.is_empty() {
                    return false;
                }
                if !raw.contains('*') {
                    return raw == url_href
                        || (url_href.ends_with('/')
                            && !raw.ends_with('/')
                            && raw == url_href.trim_end_matches('/'));
                }
                let Some(re) = regex else {
                    return match_wildcard(raw, url_href);
                };
                if re.is_match(url_href) {
                    return true;
                }
                if url_href.ends_with('/') && !raw.ends_with('/') && !raw.ends_with('*') {
                    return re.is_match(url_href.trim_end_matches('/'));
                }
                false
            }
            CompiledMatcher::Regex(re) => re.is_match(url_href),
            CompiledMatcher::Hostnames(patterns) => {
                let Some(host) = url.host_str() else {
                    return false;
                };
                patterns.iter().any(|p| match p {
                    HostnamePattern::Exact(h) => h.eq_ignore_ascii_case(host),
                    HostnamePattern::Regex(re) => re.is_match(host),
                })
            }
            CompiledMatcher::OpenerBundleId(ids) => {
                let Some(op) = opener else {
                    return false;
                };
                ids.iter().any(|id| id == &op.bundle_id)
            }
            CompiledMatcher::OpenerWindowTitle(pats) => {
                let Some(title) = opener.and_then(|o| o.window_title.as_deref()) else {
                    return false;
                };
                pats.iter().any(|pat| {
                    if pat.contains('*') {
                        match_wildcard(pat, title)
                    } else {
                        title.contains(pat.as_str())
                    }
                })
            }
            CompiledMatcher::Any(list) => list.iter().any(|m| m.matches(url, url_href, opener)),
            CompiledMatcher::All(list) => list.iter().all(|m| m.matches(url, url_href, opener)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompiledTransform {
    /// Strip query parameters matching exact keys or prefix wildcards (e.g. `"utm_*"`).
    StripParams(Vec<String>),
    ForceHttps,
    ReplaceHost(String),
    ReplaceUrl(String),
    Combined(Vec<CompiledTransform>),
}

impl CompiledTransform {
    pub fn apply(&self, url: &mut Url) {
        match self {
            CompiledTransform::StripParams(patterns) => {
                if url.query().is_none() || patterns.is_empty() {
                    return;
                }
                let kept: Vec<(String, String)> = url
                    .query_pairs()
                    .filter(|(k, _)| !param_matches_any(k, patterns))
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect();
                if kept.is_empty() {
                    url.set_query(None);
                } else {
                    let mut ser = url::form_urlencoded::Serializer::new(String::new());
                    for (k, v) in kept {
                        ser.append_pair(&k, &v);
                    }
                    let q = ser.finish();
                    url.set_query(Some(&q));
                }
            }
            CompiledTransform::ForceHttps => {
                if url.scheme() == "http" {
                    let _ = url.set_scheme("https");
                }
            }
            CompiledTransform::ReplaceHost(new_host) => {
                let _ = url.set_host(Some(new_host));
            }
            CompiledTransform::ReplaceUrl(new_url) => {
                if let Ok(parsed) = Url::parse(new_url) {
                    *url = parsed;
                }
            }
            CompiledTransform::Combined(steps) => {
                for step in steps {
                    step.apply(url);
                }
            }
        }
    }
}

fn param_matches_any(key: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pat| {
        if let Some(prefix) = pat.strip_suffix('*') {
            key.starts_with(prefix)
        } else {
            key == pat
        }
    })
}

#[derive(Debug, Clone)]
pub struct CompiledRewrite {
    pub matcher: CompiledMatcher,
    pub transform: CompiledTransform,
}

#[derive(Debug, Clone)]
pub struct CompiledHandler {
    pub matcher: CompiledMatcher,
    pub target: BrowserConfig,
}

#[derive(Debug, Clone)]
pub struct ConfigOptions {
    pub keep_running: bool,
    pub hide_icon: bool,
    pub log_requests: bool,
    pub check_for_updates: bool,
    pub resolve_short_urls: bool,
    pub on_missing_profile: MissingProfilePolicy,
}

/// Which [`ConfigOptions`] were explicitly set by the Primary Config;
/// the UI Rules File only fills in the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplicitOptions {
    pub keep_running: bool,
    pub hide_icon: bool,
    pub log_requests: bool,
    pub check_for_updates: bool,
}

impl ExplicitOptions {
    pub const NONE: Self = Self {
        keep_running: false,
        hide_icon: false,
        log_requests: false,
        check_for_updates: false,
    };

    pub const ALL: Self = Self {
        keep_running: true,
        hide_icon: true,
        log_requests: true,
        check_for_updates: true,
    };
}

/// Inherits `options` from the UI Rules File (`rules.json`) for every option the
/// Primary Config left unset.
pub fn inherit_rules_file_options(
    options: &mut ConfigOptions,
    rf: &RulesFile,
    explicit: ExplicitOptions,
) {
    let Some(opts) = &rf.options else {
        return;
    };
    if !explicit.keep_running {
        if let Some(v) = opts.keep_running {
            options.keep_running = v;
        }
    }
    if !explicit.hide_icon {
        if let Some(v) = opts.hide_icon {
            options.hide_icon = v;
        }
    }
    if !explicit.log_requests {
        if let Some(v) = opts.log_requests {
            options.log_requests = v;
        }
    }
    if !explicit.check_for_updates {
        if let Some(v) = opts.check_for_updates {
            options.check_for_updates = v;
        }
    }
}

/// Resolves the UI Rules File's `defaultBrowser`/`defaultProfile` pair into a
/// [Browser Target](../../../GLOSSARY.md), or `None` when no default is set.
pub fn rules_file_default_browser(rf: &RulesFile) -> Option<BrowserConfig> {
    if rf.default_browser.is_empty() {
        return None;
    }
    let mut db = BrowserConfig::from_spec_str(&rf.default_browser, "");
    if !rf.default_profile.is_empty() {
        match ProfileSelector::parse(&rf.default_profile) {
            ProfileSelector::Container(c) => {
                db.container = (!c.is_empty()).then(|| c.to_string());
                db.profile.clear();
            }
            ProfileSelector::Profile(p) => db.profile = p.to_string(),
        }
    }
    Some(db)
}

impl Default for ConfigOptions {
    fn default() -> Self {
        Self {
            keep_running: true,
            hide_icon: false,
            log_requests: false,
            check_for_updates: false,
            resolve_short_urls: false,
            on_missing_profile: MissingProfilePolicy::WarnAndFallback,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NativeRouter {
    pub default_browser: BrowserConfig,
    pub has_explicit_default_browser: bool,
    pub rewrites: Vec<CompiledRewrite>,
    pub handlers: Vec<CompiledHandler>,
    pub fallback_handlers: Vec<CompiledHandler>,
    pub options: ConfigOptions,
}

impl Default for NativeRouter {
    fn default() -> Self {
        Self {
            default_browser: BrowserConfig::safari_default("", false),
            has_explicit_default_browser: false,
            rewrites: Vec::new(),
            handlers: Vec::new(),
            fallback_handlers: Vec::new(),
            options: ConfigOptions::default(),
        }
    }
}

impl NativeRouter {
    pub fn uses_window_title(&self) -> bool {
        self.rewrites.iter().any(|r| r.matcher.uses_window_title())
            || self.handlers.iter().any(|h| h.matcher.uses_window_title())
            || self
                .fallback_handlers
                .iter()
                .any(|h| h.matcher.uses_window_title())
    }

    pub fn from_rules_file(rf: &RulesFile) -> Self {
        let has_explicit_default_browser = !rf.default_browser.is_empty();
        let mut default_browser = rules_file_default_browser(rf)
            .unwrap_or_else(|| BrowserConfig::safari_default("", false));
        // `defaultProfile` also applies to the Safari fallback when no default browser is set.
        if rf.default_browser.is_empty() && !rf.default_profile.is_empty() {
            let (profile, container) = ProfileSelector::parse(&rf.default_profile).split_owned();
            default_browser.profile = profile;
            if let Some(c) = container {
                default_browser.container = Some(c);
            }
        }

        let mut options = ConfigOptions::default();
        inherit_rules_file_options(&mut options, rf, ExplicitOptions::NONE);

        let handlers = compile_json_rules(&rf.rules);
        Self {
            default_browser,
            has_explicit_default_browser,
            rewrites: Vec::new(),
            handlers,
            fallback_handlers: Vec::new(),
            options,
        }
    }

    pub fn from_toml_config(toml_cfg: &TomlConfig) -> Result<Self, String> {
        let has_explicit_default_browser = toml_cfg.default_browser.is_some();
        let default_browser = match &toml_cfg.default_browser {
            Some(target) => target.to_browser_config(&toml_cfg.browsers, ""),
            None => BrowserConfig::safari_default("", false),
        };

        let mut options = ConfigOptions::default();
        if let Some(v) = toml_cfg.options.keep_running {
            options.keep_running = v;
        }
        if let Some(v) = toml_cfg.options.hide_icon {
            options.hide_icon = v;
        }
        if let Some(v) = toml_cfg.options.log_requests {
            options.log_requests = v;
        }
        if let Some(v) = toml_cfg.options.check_for_updates {
            options.check_for_updates = v;
        }
        if let Some(v) = toml_cfg.options.resolve_short_urls {
            options.resolve_short_urls = v;
        }
        options.on_missing_profile = toml_cfg.options.on_missing_profile;

        let mut rewrites = Vec::new();
        for rw in &toml_cfg.rewrite {
            let matcher = if let Some(re_str) = &rw.match_regex {
                let re = Regex::new(re_str)
                    .map_err(|e| format!("Invalid rewrite regex '{}': {}", re_str, e))?;
                CompiledMatcher::Regex(re)
            } else if let Some(pats) = &rw.match_patterns {
                let vec = pats.clone().into_vec();
                if vec.len() == 1 && vec[0] == "*" {
                    CompiledMatcher::Always
                } else {
                    CompiledMatcher::Any(
                        vec.into_iter()
                            .map(CompiledMatcher::from_wildcard)
                            .collect(),
                    )
                }
            } else {
                CompiledMatcher::Always
            };

            let mut steps = Vec::new();
            if rw.force_https == Some(true) {
                steps.push(CompiledTransform::ForceHttps);
            }
            if let Some(host) = &rw.replace_host {
                steps.push(CompiledTransform::ReplaceHost(host.clone()));
            }
            if let Some(repl) = &rw.replace_url {
                steps.push(CompiledTransform::ReplaceUrl(repl.clone()));
            }
            if !rw.strip_params.is_empty() {
                steps.push(CompiledTransform::StripParams(rw.strip_params.clone()));
            }
            if !steps.is_empty() {
                let transform = if steps.len() == 1 {
                    steps.remove(0)
                } else {
                    CompiledTransform::Combined(steps)
                };
                rewrites.push(CompiledRewrite { matcher, transform });
            }
        }

        let mut handlers = Vec::new();
        for h in &toml_cfg.handlers {
            let mut conditions = Vec::new();
            if let Some(pats) = &h.match_patterns {
                let list: Vec<CompiledMatcher> = pats
                    .clone()
                    .into_vec()
                    .into_iter()
                    .map(CompiledMatcher::from_wildcard)
                    .collect();
                conditions.push(if list.len() == 1 {
                    list.into_iter().next().unwrap()
                } else {
                    CompiledMatcher::Any(list)
                });
            }
            if let Some(re_str) = &h.match_regex {
                let re = Regex::new(re_str)
                    .map_err(|e| format!("Invalid handler regex '{}': {}", re_str, e))?;
                conditions.push(CompiledMatcher::Regex(re));
            }
            if let Some(hosts) = &h.match_hostnames {
                let patterns = hosts
                    .clone()
                    .into_vec()
                    .into_iter()
                    .map(HostnamePattern::Exact)
                    .collect();
                conditions.push(CompiledMatcher::Hostnames(patterns));
            }
            if let Some(openers) = &h.opener_bundle_id {
                conditions.push(CompiledMatcher::OpenerBundleId(openers.clone().into_vec()));
            }
            if let Some(titles) = &h.opener_window_title {
                conditions.push(CompiledMatcher::OpenerWindowTitle(titles.clone().into_vec()));
            }

            if conditions.is_empty() {
                continue;
            }
            let matcher = if conditions.len() == 1 {
                conditions.remove(0)
            } else {
                CompiledMatcher::All(conditions)
            };
            let target = h.browser.to_browser_config(&toml_cfg.browsers, "");
            handlers.push(CompiledHandler { matcher, target });
        }

        Ok(Self {
            default_browser,
            has_explicit_default_browser,
            rewrites,
            handlers,
            fallback_handlers: Vec::new(),
            options,
        })
    }

    pub fn set_fallback_rules(&mut self, rules: &[Rule]) {
        self.fallback_handlers = compile_json_rules(rules);
    }

    /// Applies `rules.json` handlers and inherits `defaultBrowser`/`options` when not explicitly set by the Primary Config.
    pub fn apply_rules_file_fallbacks(&mut self, rf: &RulesFile, explicit: ExplicitOptions) {
        self.fallback_handlers = compile_json_rules(&rf.rules);
        if !self.has_explicit_default_browser {
            if let Some(db) = rules_file_default_browser(rf) {
                self.default_browser = db;
            }
        }
        inherit_rules_file_options(&mut self.options, rf, explicit);
    }

    /// Resolves `raw_url` through all compiled rewrites, primary handlers, and fallback JSON rules.
    pub fn resolve(
        &self,
        raw_url: &str,
        opener: Option<&OpenerInfo>,
        open_in_background: bool,
    ) -> Result<BrowserConfig, String> {
        let decoded = decode_protocol_url(raw_url);
        let mut parsed =
            Url::parse(&decoded).map_err(|e| format!("Invalid URL '{}': {}", decoded, e))?;

        for rw in &self.rewrites {
            let href = parsed.as_str().to_string();
            if rw.matcher.matches(&parsed, &href, opener) {
                rw.transform.apply(&mut parsed);
            }
        }

        let final_href = parsed.as_str().to_string();

        for h in self.handlers.iter().chain(self.fallback_handlers.iter()) {
            if h.matcher.matches(&parsed, &final_href, opener) {
                let mut out = h.target.clone();
                out.url = final_href;
                if out.open_in_background.is_none() {
                    out.open_in_background = Some(open_in_background);
                }
                return Ok(out);
            }
        }

        let mut out = self.default_browser.clone();
        out.url = final_href;
        if out.open_in_background.is_none() {
            out.open_in_background = Some(open_in_background);
        }
        Ok(out)
    }
}

pub fn compile_json_rules(rules: &[Rule]) -> Vec<CompiledHandler> {
    let mut out = Vec::with_capacity(rules.len());
    for r in rules {
        let valid_pats: Vec<CompiledMatcher> = r
            .match_patterns
            .iter()
            .filter(|m| !m.is_empty())
            .cloned()
            .map(CompiledMatcher::from_wildcard)
            .collect();
        if valid_pats.is_empty() || r.browser.is_empty() {
            continue;
        }
        let matcher = if valid_pats.len() == 1 {
            valid_pats.into_iter().next().unwrap()
        } else {
            CompiledMatcher::Any(valid_pats)
        };
        let mut target = TomlBrowserTarget::NameOrAlias(r.browser.clone())
            .to_browser_config(&Default::default(), "");
        if !r.profile.is_empty() {
            let (profile, container) = ProfileSelector::parse(&r.profile).split_owned();
            target.profile = profile;
            if let Some(c) = container {
                target.container = Some(c);
            }
        }
        out.push(CompiledHandler { matcher, target });
    }
    out
}

/// Decodes `finicky://open/<base64>` and `browsify://open/<base64>` URLs emitted by the browser extension.
pub fn decode_protocol_url(url_str: &str) -> String {
    let encoded = url_str
        .strip_prefix("finicky://open/")
        .or_else(|| url_str.strip_prefix("browsify://open/"));
    if let Some(b64) = encoded {
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
            if let Ok(decoded) = String::from_utf8(bytes) {
                return decoded;
            }
        }
    }
    url_str.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_selector_parsing() {
        use crate::browser::ProfileSelector;
        assert_eq!(
            ProfileSelector::parse("Work").split_owned(),
            ("Work".to_string(), None)
        );
        assert_eq!(
            ProfileSelector::parse("container:Work").split_owned(),
            (String::new(), Some("Work".to_string()))
        );
        assert_eq!(
            ProfileSelector::parse("container:").split_owned(),
            (String::new(), None)
        );
    }

    #[test]
    fn test_decode_protocol_urls() {
        let b64 = base64::engine::general_purpose::STANDARD.encode("https://github.com/snowaxe/browsify");
        assert_eq!(
            decode_protocol_url(&format!("finicky://open/{}", b64)),
            "https://github.com/snowaxe/browsify"
        );
        assert_eq!(
            decode_protocol_url(&format!("browsify://open/{}", b64)),
            "https://github.com/snowaxe/browsify"
        );
    }

    #[test]
    fn test_toml_router_rewrites_and_handlers() {
        let toml_src = r#"
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

[[rewrite]]
match = ["x.com/*", "*.x.com/*"]
replace_host = "xcancel.com"

[[handlers]]
match = ["github.com/myorg/*", "linear.app/*"]
browser = "work"

[[handlers]]
match_hostnames = ["bsky.app", "reddit.com"]
browser = { name = "Firefox", profile = "default-release", container = "Social" }
"#;
        let cfg = TomlConfig::from_toml_str(toml_src).unwrap();
        let router = NativeRouter::from_toml_config(&cfg).unwrap();

        let res1 = router
            .resolve(
                "http://github.com/myorg/repo?utm_source=slack&keep=1",
                None,
                false,
            )
            .unwrap();
        assert_eq!(res1.name, "Google Chrome");
        assert_eq!(res1.profile, "Work");
        assert_eq!(res1.url, "https://github.com/myorg/repo?keep=1");

        let res2 = router
            .resolve("https://x.com/user/status/123?fbclid=abc", None, false)
            .unwrap();
        assert_eq!(res2.name, "Google Chrome");
        assert_eq!(res2.profile, "Personal");
        assert_eq!(res2.url, "https://xcancel.com/user/status/123");

        let res3 = router
            .resolve("https://bsky.app/profile/someone", None, false)
            .unwrap();
        assert_eq!(res3.name, "Firefox");
        assert_eq!(res3.profile, "default-release");
        assert_eq!(res3.container.as_deref(), Some("Social"));
    }
}
