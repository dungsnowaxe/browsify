use browsify_core::{
    resolve_short_url, rules_file_to_toml, BrowserConfig, ConfigOptions, NativeRouter, OpenerInfo,
    ProfileResolver, RulesFile, TomlConfig,
};
use browsify_js::HybridEngine;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConfigSummary {
    pub config_path: Option<PathBuf>,
    pub rules_path: Option<PathBuf>,
    pub format: String,
    pub pure_native_fast_path: bool,
    pub js_vm_active: bool,
    pub handlers_count: usize,
    pub static_handlers_count: usize,
    pub rewrites_count: usize,
    pub static_rewrites_count: usize,
    pub default_browser: String,
    pub options: ConfigOptions,
    pub referenced_profiles: Vec<(String, String)>,
}

enum EngineBackend {
    Native(NativeRouter),
    Js(HybridEngine),
}

pub struct BrowsifyEngine {
    backend: EngineBackend,
    summary: ConfigSummary,
}

impl BrowsifyEngine {
    pub fn load(
        home_dir: &Path,
        custom_config_path: Option<&Path>,
        custom_rules_path: Option<&Path>,
        skip_primary_config: bool,
    ) -> Result<Self, String> {
        let rules_path = custom_rules_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| RulesFile::default_path(home_dir));
        let rules_file = RulesFile::load_from_path(&rules_path)?;
        let has_rules_file = rules_path.exists();

        let primary_path = if skip_primary_config {
            None
        } else if let Some(custom) = custom_config_path {
            if !custom.exists() {
                return Err(format!("Config file not found at {}", custom.display()));
            }
            Some(custom.to_path_buf())
        } else {
            discover_config_path(home_dir)
        };

        if let Some(cfg_path) = primary_path {
            let ext = cfg_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let source = fs::read_to_string(&cfg_path)
                .map_err(|e| format!("Failed reading {}: {}", cfg_path.display(), e))?;

            if ext == "toml" {
                let toml_cfg = TomlConfig::from_toml_str(&source)?;
                let mut router = NativeRouter::from_toml_config(&toml_cfg)?;
                router.set_fallback_rules(&rules_file.rules);
                let total_h = router.handlers.len() + router.fallback_handlers.len();
                let total_r = router.rewrites.len();
                let referenced_profiles = collect_router_profiles(&router);
                let summary = ConfigSummary {
                    config_path: Some(cfg_path),
                    rules_path: has_rules_file.then_some(rules_path),
                    format: "TOML (Native)".to_string(),
                    pure_native_fast_path: true,
                    js_vm_active: false,
                    handlers_count: total_h,
                    static_handlers_count: total_h,
                    rewrites_count: total_r,
                    static_rewrites_count: total_r,
                    default_browser: router.default_browser.name.clone(),
                    options: router.options.clone(),
                    referenced_profiles,
                };
                return Ok(Self {
                    backend: EngineBackend::Native(router),
                    summary,
                });
            }

            if ext == "json" {
                let rf: RulesFile = serde_json::from_str(&source)
                    .map_err(|e| format!("Failed parsing JSON config: {}", e))?;
                let router = NativeRouter::from_rules_file(&rf);
                let total_h = router.handlers.len();
                let referenced_profiles = collect_router_profiles(&router);
                let summary = ConfigSummary {
                    config_path: Some(cfg_path),
                    rules_path: has_rules_file.then_some(rules_path),
                    format: "JSON Rules (Native)".to_string(),
                    pure_native_fast_path: true,
                    js_vm_active: false,
                    handlers_count: total_h,
                    static_handlers_count: total_h,
                    rewrites_count: 0,
                    static_rewrites_count: 0,
                    default_browser: router.default_browser.name.clone(),
                    options: router.options.clone(),
                    referenced_profiles,
                };
                return Ok(Self {
                    backend: EngineBackend::Native(router),
                    summary,
                });
            }

            // `.js`, `.mjs`, `.ts` -> HybridEngine
            let engine = HybridEngine::from_js_source(&source, &rules_file.rules)?;
            let rep = engine.report().clone();
            let js_vm_active = engine.has_js_vm();
            let format = if js_vm_active {
                format!("JS/TS ({}) — Hybrid + QuickJS", ext.to_uppercase())
            } else {
                format!(
                    "JS/TS ({}) — 100% Lowered to Native Rust (QuickJS VM Dropped)",
                    ext.to_uppercase()
                )
            };
            let mut referenced_profiles = rep.referenced_profiles.clone();
            for r in &rules_file.rules {
                if !r.browser.is_empty() && !r.profile.is_empty() {
                    let pair = (r.browser.clone(), r.profile.clone());
                    if !referenced_profiles.contains(&pair) {
                        referenced_profiles.push(pair);
                    }
                }
            }
            let summary = ConfigSummary {
                config_path: Some(cfg_path),
                rules_path: has_rules_file.then_some(rules_path),
                format,
                pure_native_fast_path: !js_vm_active,
                js_vm_active,
                handlers_count: rep.total_handlers + rules_file.rules.len(),
                static_handlers_count: rep.static_handlers + rules_file.rules.len(),
                rewrites_count: rep.total_rewrites,
                static_rewrites_count: rep.static_rewrites,
                default_browser: rep.default_browser_name,
                options: rep.options,
                referenced_profiles,
            };
            return Ok(Self {
                backend: EngineBackend::Js(engine),
                summary,
            });
        }

        // No primary config found; use `rules.json` (or default Safari fallback)
        let router = NativeRouter::from_rules_file(&rules_file);
        let total_h = router.handlers.len();
        let referenced_profiles = collect_router_profiles(&router);
        let summary = ConfigSummary {
            config_path: has_rules_file.then(|| rules_path.clone()),
            rules_path: has_rules_file.then_some(rules_path),
            format: if has_rules_file {
                "rules.json (Native)".to_string()
            } else {
                "Default (Safari fallback)".to_string()
            },
            pure_native_fast_path: true,
            js_vm_active: false,
            handlers_count: total_h,
            static_handlers_count: total_h,
            rewrites_count: 0,
            static_rewrites_count: 0,
            default_browser: router.default_browser.name.clone(),
            options: router.options.clone(),
            referenced_profiles,
        };
        Ok(Self {
            backend: EngineBackend::Native(router),
            summary,
        })
    }

    pub fn summary(&self) -> &ConfigSummary {
        &self.summary
    }

    pub fn validate_profiles(&self, resolver: &ProfileResolver) -> Vec<String> {
        let mut warnings = Vec::new();
        for (browser, profile) in &self.summary.referenced_profiles {
            if let Err(err) = resolver.resolve_profile_args(browser, profile) {
                warnings.push(err);
            }
        }
        warnings
    }

    pub fn resolve(
        &self,
        url: &str,
        opener: Option<&OpenerInfo>,
        open_in_background: bool,
    ) -> Result<BrowserConfig, String> {
        let input_url = if self.summary.options.resolve_short_urls {
            resolve_short_url(url, &[])
        } else {
            url.to_string()
        };
        match &self.backend {
            EngineBackend::Native(router) => {
                router.resolve(&input_url, opener, open_in_background)
            }
            EngineBackend::Js(engine) => {
                engine.set_system_state(
                    crate::macos::current_modifier_keys(),
                    None,
                    None,
                    &[],
                );
                engine.resolve(&input_url, opener, open_in_background)
            }
        }
    }

    pub fn update_rules(&mut self, rf: &RulesFile) {
        match &mut self.backend {
            EngineBackend::Native(router) => {
                if self.summary.config_path.is_none()
                    || self.summary.config_path == self.summary.rules_path
                {
                    *router = NativeRouter::from_rules_file(rf);
                } else {
                    router.set_fallback_rules(&rf.rules);
                }
            }
            EngineBackend::Js(engine) => {
                engine.set_fallback_rules(&rf.rules);
            }
        }
    }
}

fn collect_router_profiles(router: &NativeRouter) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if !router.default_browser.name.is_empty() && !router.default_browser.profile.is_empty() {
        out.push((
            router.default_browser.name.clone(),
            router.default_browser.profile.clone(),
        ));
    }
    for h in router.handlers.iter().chain(router.fallback_handlers.iter()) {
        if !h.target.name.is_empty() && !h.target.profile.is_empty() {
            let pair = (h.target.name.clone(), h.target.profile.clone());
            if !out.contains(&pair) {
                out.push(pair);
            }
        }
    }
    out
}

pub fn discover_config_path(home_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        // 1. Native Browsify paths
        ".config/browsify/config.toml",
        ".config/browsify/browsify.toml",
        ".browsify.toml",
        ".config/browsify/config.js",
        ".config/browsify/config.ts",
        ".browsify.js",
        ".browsify.ts",
        // 2. Finicky v3/v4 drop-in compatibility paths
        ".finicky.js",
        ".finicky.ts",
        ".config/finicky.js",
        ".config/finicky.ts",
        ".config/finicky/finicky.js",
        ".config/finicky/finicky.ts",
    ];
    for rel in candidates {
        let p = home_dir.join(rel);
        if p.is_file() {
            return Some(fs::canonicalize(&p).unwrap_or(p));
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrateTarget {
    Check,
    ToToml,
    ToJson,
}

pub fn migrate_config(
    home_dir: &Path,
    custom_config_path: Option<&Path>,
    target: MigrateTarget,
) -> Result<(String, String), String> {
    let resolver = ProfileResolver::new(home_dir);

    if let Some(path) = custom_config_path
        .map(PathBuf::from)
        .or_else(|| discover_config_path(home_dir))
    {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let src = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let label = path.display().to_string();

        if ext == "json" {
            let rf: RulesFile = serde_json::from_str(&src).map_err(|e| e.to_string())?;
            return match target {
                MigrateTarget::ToToml => Ok((label, rules_file_to_toml(&rf))),
                MigrateTarget::ToJson => {
                    let j = serde_json::to_string_pretty(&rf).map_err(|e| e.to_string())?;
                    Ok((label, j + "\n"))
                }
                MigrateTarget::Check => Ok((
                    label.clone(),
                    format!(
                        "Config: {}\nStatus: 100% Native Rust Fast Path (~2.3 µs)\nRules:  {} static rules\n",
                        label,
                        rf.rules.len()
                    ),
                )),
            };
        }

        let engine = HybridEngine::from_js_source(&src, &[])?;
        return match target {
            MigrateTarget::ToToml => Ok((label.clone(), engine.to_toml(&label))),
            MigrateTarget::ToJson => {
                let rf = engine.to_rules_file();
                let j = serde_json::to_string_pretty(&rf).map_err(|e| e.to_string())?;
                Ok((label, j + "\n"))
            }
            MigrateTarget::Check => {
                let rep = engine.report();
                let mut report_str = format!(
                    "Migration & Fast-Path Report for {}\n\
                     ========================================================\n\
                     Execution Mode:  {}\n\
                     QuickJS VM RAM:  {}\n\
                     Rewrites:        {}/{} lowered to Native Rust (~2.3 µs)\n\
                     Handlers:        {}/{} lowered to Native Rust (~2.3 µs)\n\
                     Default Browser: {}\n",
                    label,
                    if rep.all_static {
                        "100% Native Rust Fast Path (~2.3 µs/URL)"
                    } else {
                        "Hybrid (Static rules in Rust + Dynamic callbacks in QuickJS ~42 µs/URL)"
                    },
                    if rep.all_static {
                        "0 KB (VM dropped immediately after load)"
                    } else {
                        "~550 KB anonymous dirty heap"
                    },
                    rep.static_rewrites,
                    rep.total_rewrites,
                    rep.static_handlers,
                    rep.total_handlers,
                    rep.default_browser_name
                );

                if !rep.referenced_profiles.is_empty() {
                    report_str.push_str("\nReferenced Browser Profiles:\n");
                    for (browser, profile) in &rep.referenced_profiles {
                        match resolver.resolve_profile_args(browser, profile) {
                            Ok(flags) => {
                                report_str.push_str(&format!(
                                    "  ✓ {} -> {:?} ({})\n",
                                    browser,
                                    profile,
                                    flags.join(" ")
                                ));
                            }
                            Err(warn) => {
                                report_str.push_str(&format!(
                                    "  ! {} -> {:?}: {}\n",
                                    browser, profile, warn
                                ));
                            }
                        }
                    }
                }
                Ok((label, report_str))
            }
        };
    }

    let rules_path = RulesFile::default_path(home_dir);
    if rules_path.exists() {
        let rf = RulesFile::load_from_path(&rules_path)?;
        return Ok((rules_path.display().to_string(), rules_file_to_toml(&rf)));
    }

    Err("No existing Finicky or Browsify configuration file found to migrate".to_string())
}
