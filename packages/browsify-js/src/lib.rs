pub mod prelude;
pub mod preprocess;

use browsify_core::{
    decode_protocol_url, BrowserConfig, CompiledHandler, CompiledMatcher, CompiledRewrite,
    CompiledTransform, ConfigOptions, HostnamePattern, ModifierKeys, NativeRouter, OpenerInfo,
    PowerInfo, Rule, SystemInfo,
};
use prelude::JS_PRELUDE;
use preprocess::preprocess_js_ts_config;
use regex::Regex;
use rquickjs::{Context, Function, Object, Runtime};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct CompilationReport {
    pub all_static: bool,
    pub total_rewrites: usize,
    pub static_rewrites: usize,
    pub total_handlers: usize,
    pub static_handlers: usize,
    pub default_browser_name: String,
    pub default_browser_profile: String,
    pub options: ConfigOptions,
    pub referenced_profiles: Vec<(String, String)>,
}

enum ExecutionMode {
    /// 100% of the JS/TS config lowered to native Rust; the QuickJS VM was dropped from memory!
    PureNative(NativeRouter),
    /// One or more rules use dynamic JS callback functions; QuickJS VM is retained.
    DynamicJs {
        _rt: Runtime,
        ctx: Context,
        fallback_rules_json: String,
    },
}

pub struct HybridEngine {
    mode: ExecutionMode,
    report: CompilationReport,
    inspected: InspectedConfig,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InspectedConfig {
    all_static: bool,
    default_browser: InspectedBrowser,
    rewrites: Vec<InspectedRewrite>,
    handlers: Vec<InspectedHandler>,
    options: InspectedOptions,
}

#[derive(Clone, Deserialize)]
struct InspectedBrowser {
    spec: Option<BrowserConfig>,
}

#[derive(Clone, Deserialize)]
struct InspectedRewrite {
    #[serde(rename = "static")]
    is_static: bool,
    matcher: Option<InspectedMatcher>,
    transform: Option<InspectedTransform>,
}

#[derive(Clone, Deserialize)]
struct InspectedHandler {
    #[serde(rename = "static")]
    is_static: bool,
    matcher: Option<InspectedMatcher>,
    browser: Option<BrowserConfig>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InspectedMatcher {
    kind: String,
    pattern: Option<String>,
    patterns: Option<Vec<InspectedHostPattern>>,
    bundle_ids: Option<Vec<String>>,
    items: Option<Vec<InspectedMatcher>>,
}

#[derive(Clone, Deserialize)]
struct InspectedHostPattern {
    kind: String,
    pattern: String,
}

#[derive(Clone, Deserialize)]
struct InspectedTransform {
    kind: String,
    params: Option<Vec<String>>,
    url: Option<String>,
    host: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InspectedOptions {
    keep_running: bool,
    hide_icon: bool,
    log_requests: bool,
    check_for_updates: bool,
}

impl HybridEngine {
    /// Evaluates a `.finicky.js`, `.finicky.ts`, `.browsify.js`, or `.browsify.ts` configuration.
    ///
    /// Performs **Compile-Time Hybrid Lowering**:
    /// - If all rules and rewrites are declarative (wildcards, regexes, `matchHostnames`, `stripParams`,
    ///   and static browser targets), compiles them into a [`NativeRouter`] and **drops the QuickJS VM
    ///   immediately**, reducing JS memory overhead to 0 bytes!
    /// - If dynamic JS functions are present, keeps the lightweight QuickJS context alive.
    pub fn from_js_source(source: &str, fallback_rules: &[Rule]) -> Result<Self, String> {
        let preprocessed = preprocess_js_ts_config(source);
        let rt = Runtime::new().map_err(|e| format!("Failed to init QuickJS runtime: {}", e))?;
        let ctx = Context::full(&rt).map_err(|e| format!("Failed to init QuickJS context: {}", e))?;

        let inspect_json: String = ctx.with(|ctx| {
            if let Err(e) = ctx.eval::<(), _>(JS_PRELUDE) {
                let exc = ctx.catch();
                return Err(format!("Failed to evaluate JS prelude: {} ({:?})", e, exc));
            }
            if let Err(e) = ctx.eval::<(), _>(preprocessed.as_bytes()) {
                let exc = ctx.catch();
                return Err(format!("Failed to evaluate JS config: {} ({:?})", e, exc));
            }
            let globals: Object = ctx.globals();
            let inspect_fn: Function = globals
                .get("__browsifyInspectConfig")
                .map_err(|e| format!("Missing __browsifyInspectConfig: {}", e))?;
            inspect_fn
                .call::<_, String>(())
                .map_err(|e| format!("Invalid config: {}", e))
        })?;

        let inspected: InspectedConfig = serde_json::from_str(&inspect_json)
            .map_err(|e| format!("Failed to parse inspected config: {}", e))?;

        let options = ConfigOptions {
            keep_running: inspected.options.keep_running,
            hide_icon: inspected.options.hide_icon,
            log_requests: inspected.options.log_requests,
            check_for_updates: inspected.options.check_for_updates,
            ..ConfigOptions::default()
        };

        let static_rewrites = inspected.rewrites.iter().filter(|r| r.is_static).count();
        let static_handlers = inspected.handlers.iter().filter(|h| h.is_static).count();
        let default_browser_name = inspected
            .default_browser
            .spec
            .as_ref()
            .map(|b| {
                if b.name.is_empty() {
                    "None".to_string()
                } else {
                    b.name.clone()
                }
            })
            .unwrap_or_else(|| "Dynamic Function".to_string());

        let default_browser_profile = inspected
            .default_browser
            .spec
            .as_ref()
            .map(|b| b.profile.clone())
            .unwrap_or_default();

        let mut referenced_profiles = Vec::new();
        if let Some(spec) = &inspected.default_browser.spec {
            if !spec.name.is_empty() && !spec.profile.is_empty() {
                referenced_profiles.push((spec.name.clone(), spec.profile.clone()));
            }
        }
        for h in &inspected.handlers {
            if let Some(spec) = &h.browser {
                if !spec.name.is_empty() && !spec.profile.is_empty() {
                    let pair = (spec.name.clone(), spec.profile.clone());
                    if !referenced_profiles.contains(&pair) {
                        referenced_profiles.push(pair);
                    }
                }
            }
        }

        let report = CompilationReport {
            all_static: inspected.all_static,
            total_rewrites: inspected.rewrites.len(),
            static_rewrites,
            total_handlers: inspected.handlers.len(),
            static_handlers,
            default_browser_name,
            default_browser_profile,
            options: options.clone(),
            referenced_profiles,
        };

        if inspected.all_static {
            if let Ok(mut native) = lower_to_native_router(&inspected, options) {
                native.set_fallback_rules(fallback_rules);
                // Dropping `rt` and `ctx` right here frees 100% of QuickJS memory!
                drop(ctx);
                drop(rt);
                return Ok(Self {
                    mode: ExecutionMode::PureNative(native),
                    report,
                    inspected,
                });
            }
        }

        let fallback_rules_json = serialize_fallback_rules_for_js(fallback_rules);
        Ok(Self {
            mode: ExecutionMode::DynamicJs {
                _rt: rt,
                ctx,
                fallback_rules_json,
            },
            report,
            inspected,
        })
    }

    /// Exports all static rules and rewrites from this JS/TS config into a clean Browsify `config.toml`.
    pub fn to_toml(&self, source_label: &str) -> String {
        let rep = &self.report;
        let mut out = format!(
            "# Migrated from {}\n# Lowering report: {}/{} rewrites static, {}/{} handlers static (pure_native = {})\n\n",
            source_label,
            rep.static_rewrites,
            rep.total_rewrites,
            rep.static_handlers,
            rep.total_handlers,
            rep.all_static
        );

        if rep.default_browser_profile.is_empty() {
            out.push_str(&format!("default_browser = {:?}\n\n", rep.default_browser_name));
        } else {
            out.push_str(&format!(
                "default_browser = {{ name = {:?}, profile = {:?} }}\n\n",
                rep.default_browser_name, rep.default_browser_profile
            ));
        }

        out.push_str("[options]\n");
        out.push_str(&format!("keep_running = {}\n", rep.options.keep_running));
        out.push_str(&format!("hide_icon = {}\n", rep.options.hide_icon));
        out.push_str(&format!("log_requests = {}\n", rep.options.log_requests));
        out.push_str(&format!(
            "check_for_updates = {}\n\n",
            rep.options.check_for_updates
        ));

        for (idx, rw) in self.inspected.rewrites.iter().enumerate() {
            if !rw.is_static {
                out.push_str(&format!(
                    "# Rewrite #{} uses dynamic JS function and requires .js/.ts config\n\n",
                    idx + 1
                ));
                continue;
            }
            out.push_str("[[rewrite]]\n");
            if let Some(m) = &rw.matcher {
                write_matcher_toml(&mut out, m);
            }
            if let Some(t) = &rw.transform {
                match t.kind.as_str() {
                    "stripParams" => {
                        if let Some(params) = &t.params {
                            out.push_str(&format!("strip_params = {:?}\n", params));
                        }
                    }
                    "forceHttps" => {
                        out.push_str("force_https = true\n");
                    }
                    "replaceHost" => {
                        if let Some(h) = &t.host {
                            out.push_str(&format!("replace_host = {:?}\n", h));
                        }
                    }
                    "replaceUrl" => {
                        if let Some(u) = &t.url {
                            out.push_str(&format!("replace_url = {:?}\n", u));
                        }
                    }
                    _ => {}
                }
            }
            out.push('\n');
        }

        for (idx, h) in self.inspected.handlers.iter().enumerate() {
            if !h.is_static {
                out.push_str(&format!(
                    "# Handler #{} uses dynamic JS callback and requires .js/.ts config\n\n",
                    idx + 1
                ));
                continue;
            }
            out.push_str("[[handlers]]\n");
            if let Some(m) = &h.matcher {
                write_matcher_toml(&mut out, m);
            }
            if let Some(b) = &h.browser {
                if b.profile.is_empty()
                    && b.container.is_none()
                    && b.app_id.is_none()
                    && b.space.is_none()
                {
                    out.push_str(&format!("browser = {:?}\n\n", b.name));
                } else {
                    let mut fields = vec![format!("name = {:?}", b.name)];
                    if !b.profile.is_empty() {
                        fields.push(format!("profile = {:?}", b.profile));
                    }
                    if let Some(c) = &b.container {
                        fields.push(format!("container = {:?}", c));
                    }
                    if let Some(a) = &b.app_id {
                        fields.push(format!("app_id = {:?}", a));
                    }
                    if let Some(s) = &b.space {
                        fields.push(format!("space = {:?}", s));
                    }
                    out.push_str(&format!("browser = {{ {} }}\n\n", fields.join(", ")));
                }
            }
        }

        out
    }

    /// Exports static wildcard handlers from this JS/TS config into a Finicky-compatible `RulesFile` (`rules.json`).
    pub fn to_rules_file(&self) -> browsify_core::RulesFile {
        let mut rules = Vec::new();
        for h in &self.inspected.handlers {
            if !h.is_static {
                continue;
            }
            let Some(m) = &h.matcher else { continue };
            let Some(b) = &h.browser else { continue };
            let mut pats = Vec::new();
            collect_wildcard_patterns(m, &mut pats);
            if !pats.is_empty() && !b.name.is_empty() {
                rules.push(Rule {
                    match_patterns: pats,
                    browser: b.name.clone(),
                    profile: b.profile.clone(),
                });
            }
        }
        browsify_core::RulesFile {
            default_browser: self.report.default_browser_name.clone(),
            default_profile: self.report.default_browser_profile.clone(),
            options: Some(browsify_core::RulesOptions {
                keep_running: Some(self.report.options.keep_running),
                hide_icon: Some(self.report.options.hide_icon),
                log_requests: Some(self.report.options.log_requests),
                check_for_updates: Some(self.report.options.check_for_updates),
            }),
            rules,
        }
    }

    /// Returns `true` if the QuickJS VM was retained (config has dynamic JS callbacks),
    /// or `false` if the config was 100% lowered to native Rust and the JS VM was dropped.
    pub fn has_js_vm(&self) -> bool {
        matches!(self.mode, ExecutionMode::DynamicJs { .. })
    }

    pub fn report(&self) -> &CompilationReport {
        &self.report
    }

    pub fn set_fallback_rules(&mut self, rules: &[Rule]) {
        match &mut self.mode {
            ExecutionMode::PureNative(router) => router.set_fallback_rules(rules),
            ExecutionMode::DynamicJs {
                fallback_rules_json,
                ..
            } => {
                *fallback_rules_json = serialize_fallback_rules_for_js(rules);
            }
        }
    }

    pub fn set_system_state(
        &self,
        modifiers: ModifierKeys,
        sys_info: Option<&SystemInfo>,
        power_info: Option<&PowerInfo>,
        running_apps: &[String],
    ) {
        if let ExecutionMode::DynamicJs { ctx, .. } = &self.mode {
            ctx.with(|ctx| {
                let mod_json = serde_json::to_string(&modifiers).unwrap_or_default();
                let apps_json = serde_json::to_string(running_apps).unwrap_or_default();
                let mut script = format!(
                    "globalThis.__browsifyModifierKeys = {}; globalThis.__browsifyRunningApps = {};",
                    mod_json, apps_json
                );
                if let Some(si) = sys_info {
                    if let Ok(j) = serde_json::to_string(si) {
                        script.push_str(&format!("globalThis.__browsifySystemInfo = {};", j));
                    }
                }
                if let Some(pi) = power_info {
                    if let Ok(j) = serde_json::to_string(pi) {
                        script.push_str(&format!("globalThis.__browsifyPowerInfo = {};", j));
                    }
                }
                let _ = ctx.eval::<(), _>(script.as_bytes());
            });
        }
    }

    pub fn resolve(
        &self,
        raw_url: &str,
        opener: Option<&OpenerInfo>,
        open_in_background: bool,
    ) -> Result<BrowserConfig, String> {
        match &self.mode {
            ExecutionMode::PureNative(router) => {
                router.resolve(raw_url, opener, open_in_background)
            }
            ExecutionMode::DynamicJs {
                ctx,
                fallback_rules_json,
                ..
            } => {
                let decoded = decode_protocol_url(raw_url);
                let opener_json = match opener {
                    Some(op) => serde_json::to_string(op).unwrap_or_default(),
                    None => String::new(),
                };
                let res_json: String = ctx.with(|ctx| {
                    let globals: Object = ctx.globals();
                    let open_fn: Function = globals
                        .get("__browsifyOpenUrl")
                        .map_err(|e| format!("Missing __browsifyOpenUrl: {}", e))?;
                    open_fn
                        .call((
                            decoded.as_str(),
                            opener_json.as_str(),
                            raw_url,
                            fallback_rules_json.as_str(),
                        ))
                        .map_err(|e| format!("Failed evaluating URL in JS config: {}", e))
                })?;

                let mut cfg: BrowserConfig = serde_json::from_str(&res_json)
                    .map_err(|e| format!("Invalid browser config returned from JS: {}", e))?;
                if cfg.open_in_background.is_none() {
                    cfg.open_in_background = Some(open_in_background);
                }
                Ok(cfg)
            }
        }
    }
}

fn lower_matcher(m: &InspectedMatcher) -> Result<CompiledMatcher, String> {
    match m.kind.as_str() {
        "always" => Ok(CompiledMatcher::Always),
        "wildcard" => Ok(CompiledMatcher::from_wildcard(
            m.pattern.clone().unwrap_or_default(),
        )),
        "regex" => {
            let pat = m.pattern.as_deref().unwrap_or("");
            let re = Regex::new(pat).map_err(|e| format!("Unsupported regex '{}': {}", pat, e))?;
            Ok(CompiledMatcher::Regex(re))
        }
        "hostnames" => {
            let mut list = Vec::new();
            for hp in m.patterns.as_deref().unwrap_or(&[]) {
                if hp.kind == "regex" {
                    let re = Regex::new(&hp.pattern)
                        .map_err(|e| format!("Invalid hostname regex: {}", e))?;
                    list.push(HostnamePattern::Regex(re));
                } else {
                    list.push(HostnamePattern::Exact(hp.pattern.clone()));
                }
            }
            Ok(CompiledMatcher::Hostnames(list))
        }
        "openerBundleId" => Ok(CompiledMatcher::OpenerBundleId(
            m.bundle_ids.clone().unwrap_or_default(),
        )),
        "any" => {
            let mut items = Vec::new();
            for item in m.items.as_deref().unwrap_or(&[]) {
                items.push(lower_matcher(item)?);
            }
            Ok(CompiledMatcher::Any(items))
        }
        other => Err(format!("Unsupported matcher kind: {}", other)),
    }
}

fn write_matcher_toml(out: &mut String, m: &InspectedMatcher) {
    match m.kind.as_str() {
        "always" => out.push_str("match = \"*\"\n"),
        "wildcard" => {
            if let Some(p) = &m.pattern {
                out.push_str(&format!("match = {:?}\n", p));
            }
        }
        "regex" => {
            if let Some(p) = &m.pattern {
                out.push_str(&format!("match_regex = {:?}\n", p));
            }
        }
        "hostnames" => {
            let hosts: Vec<String> = m
                .patterns
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|h| h.pattern.clone())
                .collect();
            out.push_str(&format!("match_hostnames = {:?}\n", hosts));
        }
        "openerBundleId" => {
            if let Some(ids) = &m.bundle_ids {
                if ids.len() == 1 {
                    out.push_str(&format!("opener_bundle_id = {:?}\n", ids[0]));
                } else {
                    out.push_str(&format!("opener_bundle_id = {:?}\n", ids));
                }
            }
        }
        "any" => {
            let mut pats = Vec::new();
            collect_wildcard_patterns(m, &mut pats);
            if !pats.is_empty() {
                out.push_str(&format!("match = {:?}\n", pats));
            }
        }
        _ => {}
    }
}

fn collect_wildcard_patterns(m: &InspectedMatcher, out: &mut Vec<String>) {
    match m.kind.as_str() {
        "wildcard" => {
            if let Some(p) = &m.pattern {
                out.push(p.clone());
            }
        }
        "any" => {
            for item in m.items.as_deref().unwrap_or(&[]) {
                collect_wildcard_patterns(item, out);
            }
        }
        _ => {}
    }
}

fn lower_to_native_router(
    inspected: &InspectedConfig,
    options: ConfigOptions,
) -> Result<NativeRouter, String> {
    let default_browser = inspected
        .default_browser
        .spec
        .clone()
        .ok_or_else(|| "Dynamic defaultBrowser".to_string())?;

    let mut rewrites = Vec::new();
    for r in &inspected.rewrites {
        let m = lower_matcher(r.matcher.as_ref().ok_or("Missing rewrite matcher")?)?;
        let t_spec = r.transform.as_ref().ok_or("Missing rewrite transform")?;
        let transform = match t_spec.kind.as_str() {
            "stripParams" => {
                CompiledTransform::StripParams(t_spec.params.clone().unwrap_or_default())
            }
            "forceHttps" => CompiledTransform::ForceHttps,
            "replaceHost" => {
                CompiledTransform::ReplaceHost(t_spec.host.clone().unwrap_or_default())
            }
            "replaceUrl" => CompiledTransform::ReplaceUrl(t_spec.url.clone().unwrap_or_default()),
            _ => return Err("Dynamic rewrite transform".to_string()),
        };
        rewrites.push(CompiledRewrite {
            matcher: m,
            transform,
        });
    }

    let mut handlers = Vec::new();
    for h in &inspected.handlers {
        let m = lower_matcher(h.matcher.as_ref().ok_or("Missing handler matcher")?)?;
        let b = h.browser.clone().ok_or("Missing handler browser")?;
        handlers.push(CompiledHandler {
            matcher: m,
            target: b,
        });
    }

    Ok(NativeRouter {
        default_browser,
        rewrites,
        handlers,
        fallback_handlers: Vec::new(),
        options,
    })
}

fn serialize_fallback_rules_for_js(rules: &[Rule]) -> String {
    let items: Vec<serde_json::Value> = rules
        .iter()
        .filter(|r| !r.match_patterns.is_empty() && !r.browser.is_empty())
        .map(|r| {
            let valid: Vec<String> = r
                .match_patterns
                .iter()
                .filter(|m| !m.is_empty())
                .cloned()
                .collect();
            let match_val = if valid.len() == 1 {
                serde_json::Value::String(valid[0].clone())
            } else {
                serde_json::json!(valid)
            };
            let browser_val = if r.profile.is_empty() {
                serde_json::Value::String(r.browser.clone())
            } else {
                serde_json::json!({ "name": r.browser, "profile": r.profile })
            };
            serde_json::json!({
                "match": match_val,
                "browser": browser_val
            })
        })
        .collect();
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use browsify_core::AppType;

    fn mock_opener() -> OpenerInfo {
        OpenerInfo {
            name: "TestApp".to_string(),
            bundle_id: "com.test.app".to_string(),
            path: "/Applications/TestApp.app".to_string(),
            window_title: None,
        }
    }

    #[test]
    fn test_static_js_config_drops_vm() {
        let js = r#"
export default {
  defaultBrowser: "Safari",
  rewrite: [
    finicky.stripParams(["utm_*", "fbclid"]),
  ],
  handlers: [
    { match: "browser.spotify.com*", browser: "Spotify" },
    { match: ["github.com*", /amazon/], browser: "Google Chrome" },
    {
      match: ["figma.com*", "sketch.com*"],
      browser: { name: "Google Chrome", profile: "Design" },
    },
    {
      match: finicky.matchHostnames(["meet.google.com"]),
      browser: { name: "Google Chrome", profile: "Work" },
    },
  ],
};
"#;
        let engine = HybridEngine::from_js_source(js, &[]).unwrap();
        // Verify the QuickJS VM was completely dropped!
        assert!(
            !engine.has_js_vm(),
            "Expected static JS config to lower to pure Rust and drop QuickJS VM"
        );
        assert!(engine.report().all_static);

        let r1 = engine
            .resolve(
                "https://figma.com/file/123?utm_source=slack",
                Some(&mock_opener()),
                false,
            )
            .unwrap();
        assert_eq!(r1.name, "Google Chrome");
        assert_eq!(r1.profile, "Design");
        assert_eq!(r1.url, "https://figma.com/file/123");

        let r2 = engine
            .resolve("https://meet.google.com/abc-defg-hij", None, false)
            .unwrap();
        assert_eq!(r2.name, "Google Chrome");
        assert_eq!(r2.profile, "Work");
    }

    #[test]
    fn test_finicky_config_test_suite_parity() {
        // Port of `packages/config-api/src/config.test.ts`
        let handler_config = r#"
export default {
  defaultBrowser: "Firefox",
  handlers: [
    { match: "browser.spotify.com*", browser: "Spotify" },
    { match: ["github.com*", /amazon/], browser: "Google Chrome" },
    {
      match: ["figma.com*", "sketch.com*"],
      browser: { name: "Google Chrome", profile: "Design" },
    },
    {
      match: (url: URL) => url.pathname.includes("docs"),
      browser: "Google Chrome",
    },
  ],
};
"#;
        let engine = HybridEngine::from_js_source(handler_config, &[]).unwrap();
        assert!(engine.has_js_vm());

        let opener = mock_opener();
        assert_eq!(
            engine
                .resolve("https://example.com", Some(&opener), false)
                .unwrap()
                .name,
            "Firefox"
        );
        assert_eq!(
            engine
                .resolve("https://browser.spotify.com/track/123", Some(&opener), false)
                .unwrap()
                .name,
            "Spotify"
        );
        assert_eq!(
            engine
                .resolve("https://github.com/some-repo", Some(&opener), false)
                .unwrap()
                .name,
            "Google Chrome"
        );
        assert_eq!(
            engine
                .resolve("https://amazon.com/product", Some(&opener), false)
                .unwrap()
                .name,
            "Google Chrome"
        );
        let figma = engine
            .resolve("https://figma.com/file/123", Some(&opener), false)
            .unwrap();
        assert_eq!(figma.name, "Google Chrome");
        assert_eq!(figma.profile, "Design");
        assert_eq!(
            engine
                .resolve("https://example.com/docs", Some(&opener), false)
                .unwrap()
                .name,
            "Google Chrome"
        );
    }

    #[test]
    fn test_finicky_rewrites_and_v3_legacy_objects() {
        let rewrite_config = r#"
module.exports = {
  defaultBrowser: "Safari",
  rewrite: [
    {
      match: "https://slack-redir.net/link?url=*",
      url: (url) => url.href,
    },
    {
      match: /^https:\/\/t\.co\/.+/,
      url: (url) => url.href.replace("t.co", "twitter.com"),
    },
    {
      match: (url) => url.href.includes("shortened"),
      url: (url) => url.href + "/expanded",
    },
    {
      match: (url) => url.href.includes("use-url-object"),
      url: (url) => new URL("https://example.com"),
    },
    {
      match: (url) => url.href.includes("use-v3-legacy"),
      url: (url) => ({
        protocol: "https",
        host: "legacy.com",
        pathname: "/legacy-path",
        search: "query=value",
        hash: "fragment",
      }),
    },
  ],
};
"#;
        let engine = HybridEngine::from_js_source(rewrite_config, &[]).unwrap();
        let opener = mock_opener();

        assert_eq!(
            engine
                .resolve("https://t.co/abc123", Some(&opener), false)
                .unwrap()
                .url,
            "https://twitter.com/abc123"
        );
        assert_eq!(
            engine
                .resolve("https://my.shortened.url/123", Some(&opener), false)
                .unwrap()
                .url,
            "https://my.shortened.url/123/expanded"
        );
        assert_eq!(
            engine
                .resolve("https://www.use-url-object.com", Some(&opener), false)
                .unwrap()
                .url,
            "https://example.com/"
        );
        assert_eq!(
            engine
                .resolve("https://www.use-v3-legacy.com", Some(&opener), false)
                .unwrap()
                .url,
            "https://legacy.com/legacy-path?query=value#fragment"
        );
    }

    #[test]
    fn test_merged_js_and_json_rules_precedence() {
        // Port of `TestResolveURL_MergedJSAndJSON_WithCachedRules` in `resolver_test.go`
        let js = r#"
export default {
  defaultBrowser: "Safari",
  handlers: [
    { match: "*github.com/*", browser: "Firefox" }
  ]
};
"#;
        let json_rules = vec![Rule {
            match_patterns: vec!["linear.app/*".to_string()],
            browser: "Google Chrome".to_string(),
            profile: "Work".to_string(),
        }];

        let engine = HybridEngine::from_js_source(js, &json_rules).unwrap();
        assert!(!engine.has_js_vm());

        assert_eq!(
            engine.resolve("https://github.com/foo", None, false).unwrap().name,
            "Firefox"
        );
        let lin = engine
            .resolve("https://linear.app/team/issue/1", None, false)
            .unwrap();
        assert_eq!(lin.name, "Google Chrome");
        assert_eq!(lin.profile, "Work");
        assert_eq!(
            engine.resolve("https://example.com", None, false).unwrap().name,
            "Safari"
        );
    }

    #[test]
    fn test_null_browser_suppression() {
        let js = r#"
export default {
  defaultBrowser: null,
};
"#;
        let engine = HybridEngine::from_js_source(js, &[]).unwrap();
        let res = engine.resolve("https://example.com", None, false).unwrap();
        assert_eq!(res.app_type, AppType::None);
        assert_eq!(res.name, "");
    }

    #[test]
    fn test_finicky_url_legacy_properties_and_keys_error() {
        // Port of `packages/config-api/src/FinickyURL.test.ts`
        let js = r#"
export default {
  defaultBrowser: "Safari",
  handlers: [
    {
      match: (url) => {
        if (url.hostname === "test-keys.com") {
          return Boolean(url.keys);
        }
        return (
          url.urlString === "https://user:pass@example.com:8080/path?query=value#fragment" &&
          url.url.host === "example.com" &&
          url.url.protocol === "https" &&
          url.url.pathname === "/path" &&
          url.url.search === "query=value" &&
          url.url.username === "user" &&
          url.url.password === "pass" &&
          url.url.port === 8080 &&
          url.url.hash === "fragment" &&
          url.opener?.bundleId === "com.test.app"
        );
      },
      browser: "Firefox",
    },
  ],
};
"#;
        let engine = HybridEngine::from_js_source(js, &[]).unwrap();
        let opener = mock_opener();

        let res = engine
            .resolve(
                "https://user:pass@example.com:8080/path?query=value#fragment",
                Some(&opener),
                false,
            )
            .unwrap();
        assert_eq!(res.name, "Firefox");

        let err = engine
            .resolve("https://test-keys.com", Some(&opener), false)
            .unwrap_err();
        assert!(err.contains("Failed evaluating URL in JS config"));
    }

    #[test]
    fn test_repo_example_configs_and_testdata() {
        let example_js = include_str!("../../../example-config/example.js");
        let eng_js = HybridEngine::from_js_source(example_js, &[]).unwrap();
        assert_eq!(
            eng_js
                .resolve("https://example.com", None, false)
                .unwrap()
                .name,
            "Firefox"
        );

        let example_ts = include_str!("../../../example-config/example.ts");
        let eng_ts = HybridEngine::from_js_source(example_ts, &[]).unwrap();
        assert_eq!(
            eng_ts
                .resolve("https://my.workplace.com/dashboard", None, false)
                .unwrap()
                .name,
            "Firefox"
        );
        let slack_opener = OpenerInfo {
            name: "Slack".to_string(),
            bundle_id: "com.tinyspeck.slackmacgap".to_string(),
            path: "/Applications/Slack.app".to_string(),
            window_title: None,
        };
        assert_eq!(
            eng_ts
                .resolve("https://example.com", Some(&slack_opener), false)
                .unwrap()
                .name,
            "Firefox"
        );

        let testdata_js = include_str!("../../../testdata/config.js");
        let eng_td = HybridEngine::from_js_source(testdata_js, &[]).unwrap();
        assert!(
            !eng_td.has_js_vm(),
            "testdata/config.js is 100% static and should drop the QuickJS VM!"
        );
        assert_eq!(
            eng_td
                .resolve("https://github.com/snowaxe/browsify", None, false)
                .unwrap()
                .name,
            "Google Chrome"
        );
        assert_eq!(
            eng_td
                .resolve("https://linear.app/issue/1", None, false)
                .unwrap()
                .name,
            "Firefox"
        );
    }
}
