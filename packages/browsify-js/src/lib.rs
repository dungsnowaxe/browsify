pub mod prelude;
pub mod preprocess;

use browsify_core::{
    compile_json_rules, decode_protocol_url, BrowserConfig, CompiledHandler, CompiledMatcher,
    CompiledRewrite, CompiledTransform, ConfigOptions, HostnamePattern, ModifierKeys, OpenerInfo,
    PowerInfo, Rule, RulesFile, SystemInfo,
};
use prelude::JS_PRELUDE;
use preprocess::preprocess_js_ts_config;
use regex::Regex;
use rquickjs::{Context, Function, Object, Runtime};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use url::Url;

pub struct HostQueryCallbacks {
    pub get_modifier_keys: fn() -> ModifierKeys,
    pub get_system_info: fn() -> SystemInfo,
    pub get_power_info: fn() -> PowerInfo,
    pub is_app_running: fn(&str) -> bool,
}

static HOST_CALLBACKS: OnceLock<HostQueryCallbacks> = OnceLock::new();

/// Registers lazy OS host callbacks invoked on-demand when a JS rule calls
/// `finicky.getModifierKeys()`, `finicky.getSystemInfo()`, `finicky.getPowerInfo()`,
/// or `finicky.isAppRunning(id)`.
pub fn register_host_callbacks(callbacks: HostQueryCallbacks) {
    let _ = HOST_CALLBACKS.set(callbacks);
}

#[derive(Debug, Clone)]
pub struct CompilationReport {
    pub all_static: bool,
    pub total_rewrites: usize,
    pub static_rewrites: usize,
    pub total_handlers: usize,
    pub static_handlers: usize,
    pub default_browser_name: String,
    pub default_browser_profile: String,
    pub explicit_default_browser: bool,
    pub explicit_keep_running: bool,
    pub explicit_hide_icon: bool,
    pub explicit_log_requests: bool,
    pub explicit_check_for_updates: bool,
    pub uses_window_title: bool,
    pub options: ConfigOptions,
    pub referenced_profiles: Vec<(String, String)>,
}

enum HybridRewrite {
    Native(CompiledRewrite),
    Js(usize),
}

enum HybridHandler {
    Native(CompiledHandler),
    Js(usize),
}

enum HybridDefaultBrowser {
    Native(BrowserConfig),
    Js,
}

struct JsVmState {
    _rt: Runtime,
    ctx: Context,
}

pub struct HybridEngine {
    vm: Option<JsVmState>,
    rewrites: Vec<HybridRewrite>,
    handlers: Vec<HybridHandler>,
    fallback_handlers: Vec<CompiledHandler>,
    default_browser: HybridDefaultBrowser,
    report: CompilationReport,
    inspected: InspectedConfig,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InspectedConfig {
    default_browser: InspectedBrowser,
    rewrites: Vec<InspectedRewrite>,
    handlers: Vec<InspectedHandler>,
    options: InspectedOptions,
}

#[derive(Clone, Deserialize)]
struct InspectedBrowser {
    #[serde(default)]
    explicit: bool,
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
    flags: Option<String>,
    patterns: Option<Vec<InspectedHostPattern>>,
    bundle_ids: Option<Vec<String>>,
    items: Option<Vec<InspectedMatcher>>,
}

#[derive(Clone, Deserialize)]
struct InspectedHostPattern {
    kind: String,
    pattern: String,
    #[serde(default)]
    flags: String,
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
    keep_running: Option<bool>,
    hide_icon: Option<bool>,
    log_requests: Option<bool>,
    check_for_updates: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ParsedUrlComponents {
    scheme: String,
    has_slashes: bool,
    username: String,
    password: String,
    hostname: String,
    port: String,
    pathname: String,
    search: String,
    hash: String,
}

fn rust_parse_url_for_js(input: String, base: String) -> String {
    let parsed = if base.is_empty() {
        Url::parse(input.trim())
    } else {
        Url::parse(base.trim()).and_then(|b| b.join(input.trim()))
    };
    let Ok(u) = parsed else {
        return String::new();
    };
    let scheme = u.scheme().to_string();
    let after_scheme = u.as_str().get(scheme.len() + 1..).unwrap_or("");
    let has_slashes = after_scheme.starts_with("//") || u.has_host();
    let username = percent_decode_str(u.username());
    let password = u.password().map(percent_decode_str).unwrap_or_default();
    let hostname = u.host_str().unwrap_or("").to_string();
    let port = u.port().map(|p| p.to_string()).unwrap_or_default();
    let pathname = u.path().to_string();
    let search = match u.query() {
        Some(q) if !q.is_empty() => format!("?{}", q),
        _ => String::new(),
    };
    let hash = match u.fragment() {
        Some(f) if !f.is_empty() => format!("#{}", f),
        _ => String::new(),
    };

    serde_json::to_string(&ParsedUrlComponents {
        scheme,
        has_slashes,
        username,
        password,
        hostname,
        port,
        pathname,
        search,
        hash,
    })
    .unwrap_or_default()
}

fn percent_decode_str(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn bind_host_functions(ctx: &rquickjs::Ctx<'_>) -> Result<(), String> {
    let globals = ctx.globals();

    let parse_url_fn = Function::new(ctx.clone(), rust_parse_url_for_js)
        .map_err(|e| format!("Failed binding __browsifyParseUrl: {}", e))?;
    globals
        .set("__browsifyParseUrl", parse_url_fn)
        .map_err(|e| e.to_string())?;

    let get_mod_fn = Function::new(ctx.clone(), || -> String {
        let mods = HOST_CALLBACKS
            .get()
            .map(|cb| (cb.get_modifier_keys)())
            .unwrap_or_default();
        serde_json::to_string(&mods).unwrap_or_default()
    })
    .map_err(|e| e.to_string())?;
    globals
        .set("__browsifyHostGetModifierKeys", get_mod_fn)
        .map_err(|e| e.to_string())?;

    let get_sys_fn = Function::new(ctx.clone(), || -> String {
        let sys = HOST_CALLBACKS
            .get()
            .map(|cb| (cb.get_system_info)())
            .unwrap_or_else(|| SystemInfo {
                localized_name: "Mac".to_string(),
                name: "mac".to_string(),
            });
        serde_json::to_string(&sys).unwrap_or_default()
    })
    .map_err(|e| e.to_string())?;
    globals
        .set("__browsifyHostGetSystemInfo", get_sys_fn)
        .map_err(|e| e.to_string())?;

    let get_pwr_fn = Function::new(ctx.clone(), || -> String {
        let pwr = HOST_CALLBACKS
            .get()
            .map(|cb| (cb.get_power_info)())
            .unwrap_or_else(|| PowerInfo {
                is_charging: false,
                is_connected: true,
                percentage: Some(100),
            });
        serde_json::to_string(&pwr).unwrap_or_default()
    })
    .map_err(|e| e.to_string())?;
    globals
        .set("__browsifyHostGetPowerInfo", get_pwr_fn)
        .map_err(|e| e.to_string())?;

    let is_running_fn = Function::new(ctx.clone(), |id: String| -> bool {
        HOST_CALLBACKS
            .get()
            .map(|cb| (cb.is_app_running)(&id))
            .unwrap_or(false)
    })
    .map_err(|e| e.to_string())?;
    globals
        .set("__browsifyHostIsAppRunning", is_running_fn)
        .map_err(|e| e.to_string())?;

    Ok(())
}

impl HybridEngine {
    /// Evaluates a `.finicky.js`, `.finicky.ts`, `.browsify.js`, or `.browsify.ts` configuration.
    ///
    /// Performs **Safe Marker-Based Hybrid Lowering**:
    /// - Declarative rules and explicit `finicky.*` helper markers compile into native Rust steps.
    /// - Dynamic JS callbacks remain as `HybridRewrite::Js(idx)` / `HybridHandler::Js(idx)` steps
    ///   inside an ordered interleaved pipeline.
    /// - If 0 dynamic JS steps exist, the QuickJS `Runtime` and `Context` are dropped immediately!
    pub fn from_js_source(source: &str, fallback_rules: &[Rule]) -> Result<Self, String> {
        let preprocessed = preprocess_js_ts_config(source);
        let rt = Runtime::new().map_err(|e| format!("Failed to init QuickJS runtime: {}", e))?;
        let ctx = Context::full(&rt).map_err(|e| format!("Failed to init QuickJS context: {}", e))?;

        let inspect_json: String = ctx.with(|ctx| {
            bind_host_functions(&ctx)?;
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

        let mut options = ConfigOptions::default();
        if let Some(v) = inspected.options.keep_running {
            options.keep_running = v;
        }
        if let Some(v) = inspected.options.hide_icon {
            options.hide_icon = v;
        }
        if let Some(v) = inspected.options.log_requests {
            options.log_requests = v;
        }
        if let Some(v) = inspected.options.check_for_updates {
            options.check_for_updates = v;
        }

        let mut rewrites = Vec::with_capacity(inspected.rewrites.len());
        let mut static_rewrites = 0usize;
        let mut any_js_step = false;

        for (idx, r) in inspected.rewrites.iter().enumerate() {
            if r.is_static {
                if let Ok(compiled) = lower_rewrite(r) {
                    rewrites.push(HybridRewrite::Native(compiled));
                    static_rewrites += 1;
                    continue;
                }
            }
            any_js_step = true;
            rewrites.push(HybridRewrite::Js(idx));
        }

        let mut handlers = Vec::with_capacity(inspected.handlers.len());
        let mut static_handlers = 0usize;

        for (idx, h) in inspected.handlers.iter().enumerate() {
            if h.is_static {
                if let Ok(compiled) = lower_handler(h) {
                    handlers.push(HybridHandler::Native(compiled));
                    static_handlers += 1;
                    continue;
                }
            }
            any_js_step = true;
            handlers.push(HybridHandler::Js(idx));
        }

        let default_browser = match &inspected.default_browser.spec {
            Some(spec) => HybridDefaultBrowser::Native(spec.clone()),
            None => {
                any_js_step = true;
                HybridDefaultBrowser::Js
            }
        };

        let all_static = !any_js_step;
        let uses_window_title = any_js_step && source.contains("windowTitle");

        let default_browser_name = match &default_browser {
            HybridDefaultBrowser::Native(spec) => {
                if spec.name.is_empty() {
                    "None".to_string()
                } else {
                    spec.name.clone()
                }
            }
            HybridDefaultBrowser::Js => "Dynamic Function".to_string(),
        };

        let default_browser_profile = match &default_browser {
            HybridDefaultBrowser::Native(spec) => spec.profile.clone(),
            HybridDefaultBrowser::Js => String::new(),
        };

        let mut referenced_profiles = Vec::new();
        if let HybridDefaultBrowser::Native(spec) = &default_browser {
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
            all_static,
            total_rewrites: inspected.rewrites.len(),
            static_rewrites,
            total_handlers: inspected.handlers.len(),
            static_handlers,
            default_browser_name,
            default_browser_profile,
            explicit_default_browser: inspected.default_browser.explicit,
            explicit_keep_running: inspected.options.keep_running.is_some(),
            explicit_hide_icon: inspected.options.hide_icon.is_some(),
            explicit_log_requests: inspected.options.log_requests.is_some(),
            explicit_check_for_updates: inspected.options.check_for_updates.is_some(),
            uses_window_title,
            options,
            referenced_profiles,
        };

        let fallback_handlers = compile_json_rules(fallback_rules);

        let vm = if all_static {
            drop(ctx);
            drop(rt);
            None
        } else {
            Some(JsVmState { _rt: rt, ctx })
        };

        Ok(Self {
            vm,
            rewrites,
            handlers,
            fallback_handlers,
            default_browser,
            report,
            inspected,
        })
    }

    pub fn uses_window_title(&self) -> bool {
        self.report.uses_window_title
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
                if b.profile.is_empty() && b.container.is_none() && b.app_id.is_none() {
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
                    out.push_str(&format!("browser = {{ {} }}\n\n", fields.join(", ")));
                }
            }
        }

        out
    }

    /// Exports static wildcard handlers from this JS/TS config into a Finicky-compatible `RulesFile` (`rules.json`).
    pub fn to_rules_file(&self) -> RulesFile {
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
                let profile = if let Some(c) = &b.container {
                    if b.profile.is_empty() {
                        format!("container:{}", c)
                    } else {
                        b.profile.clone()
                    }
                } else {
                    b.profile.clone()
                };
                rules.push(Rule {
                    match_patterns: pats,
                    browser: b.name.clone(),
                    profile,
                });
            }
        }
        RulesFile {
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
        self.vm.is_some()
    }

    pub fn report(&self) -> &CompilationReport {
        &self.report
    }

    pub fn set_fallback_rules(&mut self, rules: &[Rule]) {
        self.fallback_handlers = compile_json_rules(rules);
    }

    /// Applies `rules.json` handlers and inherits `defaultBrowser`/`options` when omitted in the JS config.
    pub fn apply_rules_file_fallbacks(&mut self, rf: &RulesFile) {
        self.fallback_handlers = compile_json_rules(&rf.rules);
        if !self.report.explicit_default_browser && !rf.default_browser.is_empty() {
            let mut db = BrowserConfig::from_spec_str(&rf.default_browser, "");
            if !rf.default_profile.is_empty() {
                if let Some(c) = rf.default_profile.strip_prefix("container:") {
                    db.container = (!c.is_empty()).then(|| c.to_string());
                    db.profile.clear();
                } else {
                    db.profile = rf.default_profile.clone();
                }
            }
            self.report.default_browser_name = db.name.clone();
            self.report.default_browser_profile = db.profile.clone();
            self.default_browser = HybridDefaultBrowser::Native(db);
        }
        if let Some(opts) = &rf.options {
            if !self.report.explicit_keep_running {
                if let Some(v) = opts.keep_running {
                    self.report.options.keep_running = v;
                }
            }
            if !self.report.explicit_hide_icon {
                if let Some(v) = opts.hide_icon {
                    self.report.options.hide_icon = v;
                }
            }
            if !self.report.explicit_log_requests {
                if let Some(v) = opts.log_requests {
                    self.report.options.log_requests = v;
                }
            }
            if !self.report.explicit_check_for_updates {
                if let Some(v) = opts.check_for_updates {
                    self.report.options.check_for_updates = v;
                }
            }
        }
    }

    /// Resolves `raw_url` through the ordered hybrid pipeline (interleaving Native Rust steps
    /// and QuickJS function callbacks while preserving `options.originalUrl`).
    pub fn resolve(
        &self,
        raw_url: &str,
        opener: Option<&OpenerInfo>,
        open_in_background: bool,
    ) -> Result<BrowserConfig, String> {
        let decoded = decode_protocol_url(raw_url);
        let original_parsed =
            Url::parse(&decoded).map_err(|e| format!("Invalid URL '{}': {}", decoded, e))?;
        let original_href = original_parsed.as_str().to_string();
        let mut current_parsed = original_parsed;

        let mut lazy_opener_json: Option<String> = None;

        // 1. Ordered Rewrite Pipeline (Native + JS steps)
        for step in &self.rewrites {
            match step {
                HybridRewrite::Native(rw) => {
                    let href = current_parsed.as_str().to_string();
                    if rw.matcher.matches(&current_parsed, &href, opener) {
                        rw.transform.apply(&mut current_parsed);
                    }
                }
                HybridRewrite::Js(idx) => {
                    let Some(vm) = &self.vm else {
                        return Err("Internal error: JS rewrite step without VM".to_string());
                    };
                    let current_href = current_parsed.as_str().to_string();
                    let opener_json = get_or_init_opener_json(&mut lazy_opener_json, opener);
                    let next_href: String = vm.ctx.with(|ctx| {
                        let globals: Object = ctx.globals();
                        let rw_fn: Function = globals
                            .get("__browsifyEvalRewriteStep")
                            .map_err(|e| format!("Missing __browsifyEvalRewriteStep: {}", e))?;
                        rw_fn
                            .call((*idx, current_href.as_str(), original_href.as_str(), opener_json))
                            .map_err(|e| format!("Failed evaluating JS rewrite #{}: {}", idx + 1, e))
                    })?;
                    current_parsed = Url::parse(&next_href)
                        .map_err(|e| format!("Invalid rewritten URL '{}': {}", next_href, e))?;
                }
            }
        }

        let final_href = current_parsed.as_str().to_string();

        // 2. Ordered Primary Handler Pipeline (Native + JS steps)
        for step in &self.handlers {
            match step {
                HybridHandler::Native(h) => {
                    if h.matcher.matches(&current_parsed, &final_href, opener) {
                        let mut out = h.target.clone();
                        out.url = final_href;
                        if out.open_in_background.is_none() {
                            out.open_in_background = Some(open_in_background);
                        }
                        return Ok(out);
                    }
                }
                HybridHandler::Js(idx) => {
                    let Some(vm) = &self.vm else {
                        return Err("Internal error: JS handler step without VM".to_string());
                    };
                    let opener_json = get_or_init_opener_json(&mut lazy_opener_json, opener);
                    let matched_json: String = vm.ctx.with(|ctx| {
                        let globals: Object = ctx.globals();
                        let h_fn: Function = globals
                            .get("__browsifyEvalHandlerStep")
                            .map_err(|e| format!("Missing __browsifyEvalHandlerStep: {}", e))?;
                        h_fn.call((*idx, final_href.as_str(), original_href.as_str(), opener_json))
                            .map_err(|e| {
                                format!("Failed evaluating URL in JS config: {}", e)
                            })
                    })?;
                    if !matched_json.is_empty() {
                        let mut cfg: BrowserConfig = serde_json::from_str(&matched_json)
                            .map_err(|e| format!("Invalid browser config from JS: {}", e))?;
                        if cfg.open_in_background.is_none() {
                            cfg.open_in_background = Some(open_in_background);
                        }
                        return Ok(cfg);
                    }
                }
            }
        }

        // 3. Fallback UI Rules File (`rules.json`) Handlers
        for h in &self.fallback_handlers {
            if h.matcher.matches(&current_parsed, &final_href, opener) {
                let mut out = h.target.clone();
                out.url = final_href;
                if out.open_in_background.is_none() {
                    out.open_in_background = Some(open_in_background);
                }
                return Ok(out);
            }
        }

        // 4. Default Browser (Native or JS)
        match &self.default_browser {
            HybridDefaultBrowser::Native(spec) => {
                let mut out = spec.clone();
                out.url = final_href;
                if out.open_in_background.is_none() {
                    out.open_in_background = Some(open_in_background);
                }
                Ok(out)
            }
            HybridDefaultBrowser::Js => {
                let Some(vm) = &self.vm else {
                    return Err("Internal error: dynamic defaultBrowser without VM".to_string());
                };
                let opener_json = get_or_init_opener_json(&mut lazy_opener_json, opener);
                let res_json: String = vm.ctx.with(|ctx| {
                    let globals: Object = ctx.globals();
                    let def_fn: Function = globals
                        .get("__browsifyEvalDefaultBrowser")
                        .map_err(|e| format!("Missing __browsifyEvalDefaultBrowser: {}", e))?;
                    def_fn
                        .call((final_href.as_str(), original_href.as_str(), opener_json))
                        .map_err(|e| format!("Failed evaluating defaultBrowser in JS: {}", e))
                })?;
                let mut cfg: BrowserConfig = serde_json::from_str(&res_json)
                    .map_err(|e| format!("Invalid defaultBrowser returned from JS: {}", e))?;
                if cfg.open_in_background.is_none() {
                    cfg.open_in_background = Some(open_in_background);
                }
                Ok(cfg)
            }
        }
    }
}

fn get_or_init_opener_json<'a>(
    cache: &'a mut Option<String>,
    opener: Option<&OpenerInfo>,
) -> &'a str {
    if cache.is_none() {
        *cache = Some(match opener {
            Some(op) => serde_json::to_string(op).unwrap_or_default(),
            None => String::new(),
        });
    }
    cache.as_deref().unwrap_or("")
}

fn lower_rewrite(r: &InspectedRewrite) -> Result<CompiledRewrite, String> {
    let m = lower_matcher(r.matcher.as_ref().ok_or("Missing rewrite matcher")?)?;
    let t_spec = r.transform.as_ref().ok_or("Missing rewrite transform")?;
    let transform = match t_spec.kind.as_str() {
        "stripParams" => CompiledTransform::StripParams(t_spec.params.clone().unwrap_or_default()),
        "forceHttps" => CompiledTransform::ForceHttps,
        "replaceHost" => CompiledTransform::ReplaceHost(t_spec.host.clone().unwrap_or_default()),
        "replaceUrl" => CompiledTransform::ReplaceUrl(t_spec.url.clone().unwrap_or_default()),
        _ => return Err("Dynamic rewrite transform".to_string()),
    };
    Ok(CompiledRewrite {
        matcher: m,
        transform,
    })
}

fn lower_handler(h: &InspectedHandler) -> Result<CompiledHandler, String> {
    let m = lower_matcher(h.matcher.as_ref().ok_or("Missing handler matcher")?)?;
    let b = h.browser.clone().ok_or("Missing handler browser")?;
    Ok(CompiledHandler {
        matcher: m,
        target: b,
    })
}

fn apply_js_regex_flags(pattern: &str, flags: &str) -> Result<String, String> {
    let mut inline = String::new();
    for ch in flags.chars() {
        match ch {
            'i' | 'm' | 's' => {
                if !inline.contains(ch) {
                    inline.push(ch);
                }
            }
            // 'g', 'u', 'd' do not change boolean `.test()` semantics on a single string in Rust regex
            'g' | 'u' | 'd' => {}
            other => {
                return Err(format!("Unsupported JS RegExp flag '{}'", other));
            }
        }
    }
    if inline.is_empty() {
        Ok(pattern.to_string())
    } else {
        Ok(format!("(?{}){}", inline, pattern))
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
            let flags = m.flags.as_deref().unwrap_or("");
            let full_pat = apply_js_regex_flags(pat, flags)?;
            let re =
                Regex::new(&full_pat).map_err(|e| format!("Unsupported regex '{}': {}", pat, e))?;
            Ok(CompiledMatcher::Regex(re))
        }
        "hostnames" => {
            let mut list = Vec::new();
            for hp in m.patterns.as_deref().unwrap_or(&[]) {
                if hp.kind == "regex" {
                    let full_pat = apply_js_regex_flags(&hp.pattern, &hp.flags)?;
                    let re = Regex::new(&full_pat)
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
                let flags = m.flags.as_deref().unwrap_or("");
                let full_pat = apply_js_regex_flags(p, flags).unwrap_or_else(|_| p.clone());
                out.push_str(&format!("match_regex = {:?}\n", full_pat));
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
    finicky.forceHttps(),
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
    {
      match: finicky.matchOpener("com.tinyspeck.slackmacgap"),
      browser: "Firefox",
    },
  ],
};
"#;
        let engine = HybridEngine::from_js_source(js, &[]).unwrap();
        assert!(
            !engine.has_js_vm(),
            "Expected static JS config to lower to pure Rust and drop QuickJS VM"
        );
        assert!(engine.report().all_static);

        let r1 = engine
            .resolve(
                "http://figma.com/file/123?utm_source=slack",
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
    fn test_conditional_js_closure_is_never_miscompiled_as_static() {
        // Regression test for Q3: a conditional closure containing `url.host = ...`
        // must NOT be regex-sniffed into an unconditional static ReplaceHost!
        let js = r#"
export default {
  defaultBrowser: "Safari",
  rewrite: [
    finicky.stripParams(["utm_*"]),
    {
      match: "*",
      url: (url, options) => {
        if (url.pathname.startsWith("/old") && options.originalUrl.search.includes("utm_source")) {
          url.host = "archive.example.com";
        }
        return url;
      },
    },
  ],
  handlers: [
    { match: "github.com/*", browser: "Firefox" },
  ],
};
"#;
        let engine = HybridEngine::from_js_source(js, &[]).unwrap();
        assert!(engine.has_js_vm());
        assert_eq!(engine.report().static_rewrites, 1);
        assert_eq!(engine.report().total_rewrites, 2);
        assert_eq!(engine.report().static_handlers, 1);

        // Non-matching path keeps original host while still stripping utm_source via native step 0
        let normal = engine
            .resolve("https://example.com/new?utm_source=test&keep=1", None, false)
            .unwrap();
        assert_eq!(normal.url, "https://example.com/new?keep=1");

        // Matching path sees both stripped `url` AND pre-rewrite `options.originalUrl`!
        let archived = engine
            .resolve("https://example.com/old/page?utm_source=test", None, false)
            .unwrap();
        assert_eq!(archived.url, "https://archive.example.com/old/page");
    }

    #[test]
    fn test_regex_flags_and_modifier_keys_fn_field_in_mixed_config() {
        let js = r#"
export default {
  defaultBrowser: () => "Safari",
  handlers: [
    { match: /EXAMPLE\.COM/i, browser: "Firefox" },
    {
      match: () => finicky.getModifierKeys().fn === false,
      browser: "Google Chrome",
    },
  ],
};
"#;
        let engine = HybridEngine::from_js_source(js, &[]).unwrap();
        assert!(engine.has_js_vm());

        // Case-insensitive `/EXAMPLE\.COM/i` lowered to native Rust must still match `https://example.com`
        let r1 = engine.resolve("https://example.com/path", None, false).unwrap();
        assert_eq!(r1.name, "Firefox");

        // `finicky.getModifierKeys().fn === false` must evaluate to true (not undefined!)
        let r2 = engine.resolve("https://other.org/path", None, false).unwrap();
        assert_eq!(r2.name, "Google Chrome");
    }

    #[test]
    fn test_ipv6_and_whatwg_url_parity_in_js() {
        let js = r#"
export default {
  defaultBrowser: "Safari",
  handlers: [
    {
      match: (url) => url.hostname === "[::1]" && url.port === "8080",
      browser: "Firefox",
    },
  ],
};
"#;
        let engine = HybridEngine::from_js_source(js, &[]).unwrap();
        let res = engine
            .resolve("http://[::1]:8080/dashboard", None, false)
            .unwrap();
        assert_eq!(res.name, "Firefox");
    }

    #[test]
    fn test_finicky_config_test_suite_parity() {
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
        // Notice: first 3 handlers are compiled to Native Rust, 4th is JS!
        assert_eq!(engine.report().static_handlers, 3);
        assert_eq!(engine.report().total_handlers, 4);

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
