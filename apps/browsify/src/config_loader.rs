use browsify_core::{
    resolve_short_url, rules_file_to_toml, BrowserConfig, ConfigOptions, NativeRouter, OpenerInfo,
    ProfileResolver, RulesFile, TomlConfig,
};
use browsify_js::HybridEngine;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone)]
pub struct ConfigSummary {
    pub config_path: Option<PathBuf>,
    #[allow(dead_code)]
    pub rules_path: Option<PathBuf>,
    pub has_primary_config: bool,
    pub explicit_default_browser: bool,
    pub format: String,
    pub pure_native_fast_path: bool,
    pub js_vm_active: bool,
    pub uses_window_title: bool,
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
    home_dir: PathBuf,
    custom_config_path: Option<PathBuf>,
    custom_rules_path: Option<PathBuf>,
    skip_primary_config: bool,
    primary_config_path: Option<PathBuf>,
    config_mtime: Option<u128>,
    rules_mtime: Option<u128>,
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
        let rules_mtime = file_mtime(&rules_path);

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
            let config_mtime = file_mtime(&cfg_path);
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
                router.apply_rules_file_fallbacks(
                    &rules_file,
                    toml_cfg.options.keep_running.is_some(),
                    toml_cfg.options.hide_icon.is_some(),
                    toml_cfg.options.log_requests.is_some(),
                    toml_cfg.options.check_for_updates.is_some(),
                );
                let total_h = router.handlers.len() + router.fallback_handlers.len();
                let total_r = router.rewrites.len();
                let referenced_profiles = collect_router_profiles(&router);
                let uses_window_title = router.uses_window_title();
                let explicit_default_browser = router.has_explicit_default_browser;
                let summary = ConfigSummary {
                    config_path: Some(cfg_path.clone()),
                    rules_path: has_rules_file.then_some(rules_path),
                    has_primary_config: true,
                    explicit_default_browser,
                    format: "TOML (Native)".to_string(),
                    pure_native_fast_path: true,
                    js_vm_active: false,
                    uses_window_title,
                    handlers_count: total_h,
                    static_handlers_count: total_h,
                    rewrites_count: total_r,
                    static_rewrites_count: total_r,
                    default_browser: router.default_browser.name.clone(),
                    options: router.options.clone(),
                    referenced_profiles,
                };
                return Ok(Self {
                    home_dir: home_dir.to_path_buf(),
                    custom_config_path: custom_config_path.map(PathBuf::from),
                    custom_rules_path: custom_rules_path.map(PathBuf::from),
                    skip_primary_config,
                    primary_config_path: Some(cfg_path),
                    config_mtime,
                    rules_mtime,
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
                    config_path: Some(cfg_path.clone()),
                    rules_path: has_rules_file.then_some(rules_path),
                    has_primary_config: false,
                    explicit_default_browser: router.has_explicit_default_browser,
                    format: "JSON Rules (Native)".to_string(),
                    pure_native_fast_path: true,
                    js_vm_active: false,
                    uses_window_title: false,
                    handlers_count: total_h,
                    static_handlers_count: total_h,
                    rewrites_count: 0,
                    static_rewrites_count: 0,
                    default_browser: router.default_browser.name.clone(),
                    options: router.options.clone(),
                    referenced_profiles,
                };
                return Ok(Self {
                    home_dir: home_dir.to_path_buf(),
                    custom_config_path: custom_config_path.map(PathBuf::from),
                    custom_rules_path: custom_rules_path.map(PathBuf::from),
                    skip_primary_config,
                    primary_config_path: Some(cfg_path),
                    config_mtime,
                    rules_mtime,
                    backend: EngineBackend::Native(router),
                    summary,
                });
            }

            // `.js`, `.mjs`, `.ts` -> HybridEngine
            let mut engine = HybridEngine::from_js_source(&source, &rules_file.rules)?;
            engine.apply_rules_file_fallbacks(&rules_file);
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
                config_path: Some(cfg_path.clone()),
                rules_path: has_rules_file.then_some(rules_path),
                has_primary_config: true,
                explicit_default_browser: rep.explicit_default_browser,
                format,
                pure_native_fast_path: !js_vm_active,
                js_vm_active,
                uses_window_title: rep.uses_window_title,
                handlers_count: rep.total_handlers + rules_file.rules.len(),
                static_handlers_count: rep.static_handlers + rules_file.rules.len(),
                rewrites_count: rep.total_rewrites,
                static_rewrites_count: rep.static_rewrites,
                default_browser: rep.default_browser_name,
                options: rep.options,
                referenced_profiles,
            };
            return Ok(Self {
                home_dir: home_dir.to_path_buf(),
                custom_config_path: custom_config_path.map(PathBuf::from),
                custom_rules_path: custom_rules_path.map(PathBuf::from),
                skip_primary_config,
                primary_config_path: Some(cfg_path),
                config_mtime,
                rules_mtime,
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
            has_primary_config: false,
            explicit_default_browser: router.has_explicit_default_browser,
            format: if has_rules_file {
                "rules.json (Native)".to_string()
            } else {
                "Default (Safari fallback)".to_string()
            },
            pure_native_fast_path: true,
            js_vm_active: false,
            uses_window_title: false,
            handlers_count: total_h,
            static_handlers_count: total_h,
            rewrites_count: 0,
            static_rewrites_count: 0,
            default_browser: router.default_browser.name.clone(),
            options: router.options.clone(),
            referenced_profiles,
        };
        Ok(Self {
            home_dir: home_dir.to_path_buf(),
            custom_config_path: custom_config_path.map(PathBuf::from),
            custom_rules_path: custom_rules_path.map(PathBuf::from),
            skip_primary_config,
            primary_config_path: None,
            config_mtime: None,
            rules_mtime,
            backend: EngineBackend::Native(router),
            summary,
        })
    }

    pub fn summary(&self) -> &ConfigSummary {
        &self.summary
    }

    #[allow(dead_code)]
    pub fn uses_window_title(&self) -> bool {
        self.summary.uses_window_title
    }

    /// Checks if either the Primary Config or `rules.json` has changed on disk since last load.
    pub fn check_files_changed(&self) -> bool {
        let rules_path = self
            .custom_rules_path
            .clone()
            .unwrap_or_else(|| RulesFile::default_path(&self.home_dir));
        if file_mtime(&rules_path) != self.rules_mtime {
            return true;
        }
        let primary_path = if self.skip_primary_config {
            None
        } else if let Some(custom) = &self.custom_config_path {
            Some(custom.clone())
        } else {
            discover_config_path(&self.home_dir)
        };
        match (&self.primary_config_path, &primary_path) {
            (Some(old_p), Some(new_p)) => {
                if old_p != new_p || file_mtime(new_p) != self.config_mtime {
                    return true;
                }
            }
            (None, Some(_)) | (Some(_), None) => return true,
            (None, None) => {}
        }
        false
    }

    /// Reloads the configuration from disk.
    pub fn reload(&mut self) -> Result<(), String> {
        let reloaded = Self::load(
            &self.home_dir,
            self.custom_config_path.as_deref(),
            self.custom_rules_path.as_deref(),
            self.skip_primary_config,
        )?;
        *self = reloaded;
        Ok(())
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
            EngineBackend::Js(engine) => engine.resolve(&input_url, opener, open_in_background),
        }
    }

    pub fn update_rules(&mut self, rf: &RulesFile) {
        match &mut self.backend {
            EngineBackend::Native(router) => {
                if !self.summary.has_primary_config {
                    *router = NativeRouter::from_rules_file(rf);
                    self.summary.default_browser = router.default_browser.name.clone();
                    self.summary.handlers_count = router.handlers.len();
                    self.summary.static_handlers_count = router.handlers.len();
                    self.summary.options = router.options.clone();
                } else {
                    router.apply_rules_file_fallbacks(rf, true, true, true, true);
                    self.summary.default_browser = router.default_browser.name.clone();
                    self.summary.handlers_count =
                        router.handlers.len() + router.fallback_handlers.len();
                    self.summary.static_handlers_count = self.summary.handlers_count;
                }
            }
            EngineBackend::Js(engine) => {
                engine.apply_rules_file_fallbacks(rf);
                self.summary.default_browser = engine.report().default_browser_name.clone();
                self.summary.options = engine.report().options.clone();
            }
        }
    }
}

fn file_mtime(path: &Path) -> Option<u128> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_primary_config_inherits_unset_defaults_from_rules_json() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("browsify_loader_test_{}", nanos));
        fs::create_dir_all(&dir).unwrap();

        let rules_path = dir.join("rules.json");
        let rf = RulesFile {
            default_browser: "Firefox".to_string(),
            default_profile: "container:Work".to_string(),
            options: Some(browsify_core::RulesOptions {
                keep_running: Some(false),
                hide_icon: Some(true),
                log_requests: Some(true),
                check_for_updates: Some(true),
            }),
            rules: vec![browsify_core::Rule {
                match_patterns: vec!["linear.app/*".to_string()],
                browser: "Google Chrome".to_string(),
                profile: "Work".to_string(),
            }],
        };
        rf.save_to_path(&rules_path).unwrap();

        // Primary TOML omits `default_browser` and `hide_icon`
        let toml_path = dir.join("config.toml");
        fs::write(
            &toml_path,
            r#"
[options]
keep_running = true

[[handlers]]
match = "github.com/*"
browser = "Safari"
"#,
        )
        .unwrap();

        let engine =
            BrowsifyEngine::load(&dir, Some(&toml_path), Some(&rules_path), false).unwrap();
        let sum = engine.summary();
        assert!(!sum.explicit_default_browser);
        assert_eq!(sum.default_browser, "Firefox");
        // Explicitly set in TOML -> true
        assert!(sum.options.keep_running);
        // Inherited from rules.json -> true
        assert!(sum.options.hide_icon);
        assert!(sum.options.log_requests);

        // Unmatched URL falls back to `rules.json` defaultBrowser ("Firefox" + container:Work)
        let def_res = engine.resolve("https://example.com", None, false).unwrap();
        assert_eq!(def_res.name, "Firefox");
        assert_eq!(def_res.container.as_deref(), Some("Work"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_detect_new_primary_config_after_starting_with_ui_rules() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("browsify_watch_transition_{}", nanos));
        fs::create_dir_all(&dir).unwrap();

        let rules_path = dir.join("rules.json");
        let rf = RulesFile {
            default_browser: "Firefox".to_string(),
            default_profile: String::new(),
            options: None,
            rules: Vec::new(),
        };
        rf.save_to_path(&rules_path).unwrap();

        let mut engine = BrowsifyEngine::load(&dir, None, Some(&rules_path), false).unwrap();
        assert!(!engine.summary().has_primary_config);
        assert_eq!(engine.summary().config_path.as_ref(), Some(&rules_path));
        assert!(!engine.check_files_changed());

        // Now create a Primary Config (`.finicky.js`) in `dir`
        fs::write(
            dir.join(".finicky.js"),
            r#"export default { defaultBrowser: "Google Chrome" };"#,
        )
        .unwrap();

        // Must detect that a new Primary Config appeared even though `summary.config_path` had `rules_path`!
        assert!(engine.check_files_changed());
        engine.reload().unwrap();
        assert!(engine.summary().has_primary_config);
        assert_eq!(engine.summary().default_browser, "Google Chrome");

        let _ = fs::remove_dir_all(&dir);
    }
}
