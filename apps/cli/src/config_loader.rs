use browsify_core::{
    resolve_short_url, BrowserConfig, ConfigOptions, NativeRouter, OpenerInfo, ProfileResolver,
    ResolvedRoute, TomlConfig,
};

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Editable preferences and the format of pre-TOML settings imports.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_browser: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    pub options: browsify_core::RulesOptions,
}

#[derive(Debug, Clone)]
pub struct ConfigSummary {
    pub config_path: Option<PathBuf>,
    pub has_primary_config: bool,
    pub explicit_default_browser: bool,
    pub format: String,
    pub uses_window_title: bool,
    pub handlers_count: usize,
    pub rewrites_count: usize,
    pub default_browser: String,
    pub default_profile: String,
    pub options: ConfigOptions,
    pub referenced_profiles: Vec<(String, String)>,
}

impl ConfigSummary {
    /// Describes the immutable routing configuration currently in use.
    fn for_native(
        format: String,
        config_path: Option<PathBuf>,
        has_primary_config: bool,
        router: &NativeRouter,
    ) -> Self {
        let total_h = router.handlers.len() + router.fallback_handlers.len();
        let total_r = router.rewrites.len();
        Self {
            config_path,
            has_primary_config,
            explicit_default_browser: router.has_explicit_default_browser,
            format,
            uses_window_title: router.uses_window_title(),
            handlers_count: total_h,
            rewrites_count: total_r,
            default_browser: router.default_browser.name.clone(),
            default_profile: display_profile(&router.default_browser),
            options: router.options.clone(),
            referenced_profiles: collect_router_profiles(router),
        }
    }
}

pub struct BrowsifyEngine {
    pub pending_import: Option<crate::migration::Preview>,
    config_path: PathBuf,
    config_mtime: Option<u128>,
    router: NativeRouter,
    summary: ConfigSummary,
    read_only: bool,
}

impl BrowsifyEngine {
    pub fn load(
        home: &Path,
        custom: Option<&Path>,
        legacy_rules: Option<&Path>,
        skip: bool,
    ) -> Result<Self, String> {
        if legacy_rules.is_some() {
            return Err(
                "--rules was retired. Import the JSON file with browsify migrate --config PATH."
                    .into(),
            );
        }
        let path = custom
            .map(Path::to_owned)
            .unwrap_or_else(|| home.join(".config/browsify/config.toml"));
        if !path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("toml"))
        {
            return Err(
                "Routing requires TOML. Import this file with browsify migrate --config PATH."
                    .into(),
            );
        }
        let text = if skip {
            String::new()
        } else {
            match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound && custom.is_none() => {
                    String::new()
                }
                Err(e) => return Err(format!("{}: {e}", path.display())),
            }
        };
        Self::from_text(&path, &text, skip)
    }

    fn from_text(path: &Path, text: &str, read_only: bool) -> Result<Self, String> {
        let router = NativeRouter::from_toml_config(&TomlConfig::from_toml_str(text)?)?;
        let summary = ConfigSummary::for_native(
            "TOML (Native)".into(),
            Some(path.to_owned()),
            !read_only && path.exists(),
            &router,
        );
        Ok(Self {
            pending_import: None,
            config_path: path.to_owned(),
            config_mtime: file_mtime(path),
            router,
            summary,
            read_only,
        })
    }

    pub fn commit(&mut self, revision: &str, text: &str) -> Result<(), String> {
        if self.read_only {
            return Err("Editing is unavailable with --no-config.".into());
        }
        let file = crate::config_file::ConfigFile::load(&self.config_path)?;
        if file.revision() != revision {
            return Err(crate::config_file::conflict());
        }
        // Compile before publishing. After replacement, switching the running router cannot fail.
        let mut candidate = Self::from_text(&self.config_path, text, false)?;
        file.replace(text.as_bytes())?;
        candidate.summary.has_primary_config = true;
        candidate.config_mtime = file_mtime(&self.config_path);
        *self = candidate;
        Ok(())
    }

    pub fn summary(&self) -> &ConfigSummary {
        &self.summary
    }
    pub fn primary_config_path(&self) -> Option<&Path> {
        (!self.read_only).then_some(&self.config_path)
    }
    pub fn check_files_changed(&self) -> bool {
        !self.read_only && file_mtime(&self.config_path) != self.config_mtime
    }
    pub fn reload(&mut self) -> Result<(), String> {
        if self.read_only {
            return Ok(());
        }
        let source = match fs::read_to_string(&self.config_path) {
            Ok(source) => source,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e.to_string()),
        };
        *self = Self::from_text(&self.config_path, &source, false)?;
        Ok(())
    }
    pub fn validate_profiles(&self, resolver: &ProfileResolver) -> Vec<String> {
        let mut warnings = Vec::new();
        for (browser, profile) in &self.summary.referenced_profiles {
            if let Err(err) = resolver.validate_profile(browser, profile) {
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
        self.resolve_with_trace(url, opener, open_in_background)
            .map(|r| r.target)
    }

    pub fn resolve_with_trace(
        &self,
        url: &str,
        opener: Option<&OpenerInfo>,
        open_in_background: bool,
    ) -> Result<ResolvedRoute, String> {
        let input_url = if self.summary.options.resolve_short_urls {
            resolve_short_url(url, &[])
        } else {
            url.to_string()
        };
        self.router
            .resolve_with_trace(&input_url, opener, open_in_background)
    }
}

fn display_profile(target: &BrowserConfig) -> String {
    target
        .container
        .as_ref()
        .map(|c| format!("container:{c}"))
        .unwrap_or_else(|| target.profile.clone())
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
    for h in router
        .handlers
        .iter()
        .chain(router.fallback_handlers.iter())
    {
        if !h.target.name.is_empty() && !h.target.profile.is_empty() {
            let pair = (h.target.name.clone(), h.target.profile.clone());
            if !out.contains(&pair) {
                out.push(pair);
            }
        }
    }
    out
}
