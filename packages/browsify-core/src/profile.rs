use crate::browser::{lookup_browser_meta, AppType, BrowserConfig, BrowserFamily};
use crate::sqlite_profiles::read_firefox_group_profiles;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingProfilePolicy {
    /// Warn and launch the browser without the profile argument (matches Finicky v4).
    WarnAndFallback,
    /// Suppress launching the browser so URLs never leak into the wrong profile.
    Suppress,
}

impl Default for MissingProfilePolicy {
    fn default() -> Self {
        MissingProfilePolicy::WarnAndFallback
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedLaunchPlan {
    pub browser_name: String,
    pub app_type: AppType,
    pub open_in_background: bool,
    pub creates_new_instance: bool,
    /// Activate the main running application explicitly after `openApplicationAtURL`
    /// completes (fixes Finicky issues #501 and #550 when `open_in_background == false`
    /// and `creates_new_instance == true`).
    pub activate_after_handoff: bool,
    pub args: Vec<String>,
    pub final_url: String,
    pub arc_space: Option<String>,
    pub warning: Option<String>,
}

#[derive(Deserialize)]
struct ChromiumLocalState {
    profile: ChromiumProfileSection,
}

#[derive(Deserialize)]
struct ChromiumProfileSection {
    info_cache: HashMap<String, ChromiumProfileEntry>,
}

#[derive(Deserialize)]
struct ChromiumProfileEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    user_name: String,
    #[serde(default)]
    gaia_name: String,
}

#[derive(Clone, Debug)]
pub struct ChromiumProfileMeta {
    pub dir_key: String,
    pub display_name: String,
    pub user_name: String,
}

#[derive(Clone)]
struct CachedChromiumProfiles {
    mtime_nanos: u128,
    size: u64,
    profiles: Vec<ChromiumProfileMeta>,
}

#[derive(Clone)]
struct CachedFirefoxProfiles {
    mtime_nanos: u128,
    size: u64,
    ini_names: Vec<String>,
}

pub struct ProfileResolver {
    home_dir: PathBuf,
    chromium_cache: RwLock<HashMap<PathBuf, CachedChromiumProfiles>>,
    firefox_cache: RwLock<HashMap<PathBuf, CachedFirefoxProfiles>>,
}

impl ProfileResolver {
    pub fn new(home_dir: impl Into<PathBuf>) -> Self {
        Self {
            home_dir: home_dir.into(),
            chromium_cache: RwLock::new(HashMap::new()),
            firefox_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn from_env() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self::new(home)
    }

    pub fn home_dir(&self) -> &Path {
        &self.home_dir
    }

    /// Lists all discovered profile display names for `identifier` (app name or bundle ID).
    pub fn list_profiles(&self, identifier: &str) -> Vec<String> {
        let Some(meta) = lookup_browser_meta(identifier) else {
            return Vec::new();
        };
        match meta.family {
            BrowserFamily::Chromium | BrowserFamily::Arc => {
                let path = self.local_state_path(meta.config_dir_relative);
                let Some(entries) = self.load_chromium_profiles(&path) else {
                    return Vec::new();
                };
                let mut names: Vec<String> = entries
                    .into_iter()
                    .map(|p| {
                        if p.display_name.is_empty() {
                            p.dir_key
                        } else {
                            p.display_name
                        }
                    })
                    .collect();
                names.sort();
                names.dedup();
                names
            }
            BrowserFamily::Firefox => {
                let path = self.firefox_ini_path(meta.config_dir_relative);
                let mut names = self.load_firefox_profiles(&path).unwrap_or_default();
                for (group_name, _) in self.load_firefox_group_profiles(meta.config_dir_relative) {
                    if !names.contains(&group_name) {
                        names.push(group_name);
                    }
                }
                names
            }
            BrowserFamily::Safari => Vec::new(),
        }
    }

    /// Lists discovered Arc Spaces from `~/Library/Application Support/Arc/StorableSidebar.json`.
    pub fn list_arc_spaces(&self) -> Vec<String> {
        let path = self
            .home_dir
            .join("Library/Application Support/Arc/StorableSidebar.json");
        let Ok(content) = fs::read_to_string(&path) else {
            return Vec::new();
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
            return Vec::new();
        };
        let mut spaces = Vec::new();
        collect_arc_space_titles(&json, &mut spaces);
        spaces.sort();
        spaces.dedup();
        spaces
    }

    /// Resolves a Chromium PWA `app_id` (either a 32-char Chromium extension ID or a PWA `.app` display name
    /// in `~/Applications/Chrome Apps.localized/*.app`).
    pub fn resolve_pwa_app_id(&self, app_id_or_name: &str) -> String {
        if app_id_or_name.len() == 32
            && app_id_or_name
                .bytes()
                .all(|b| matches!(b, b'a'..=b'p' | b'0'..=b'9'))
        {
            return app_id_or_name.to_string();
        }
        let app_dirs = [
            "Applications/Chrome Apps.localized",
            "Applications/Brave Browser Apps.localized",
            "Applications/Microsoft Edge Apps.localized",
            "Applications",
        ];
        for rel in app_dirs {
            let candidate = self
                .home_dir
                .join(rel)
                .join(format!("{}.app/Contents/Info.plist", app_id_or_name));
            if let Ok(plist_str) = fs::read_to_string(&candidate) {
                if let Some(id) = extract_plist_string_key(&plist_str, "CrAppModeShortcutID") {
                    return id;
                }
            }
        }
        app_id_or_name.to_string()
    }

    /// Resolves a `BrowserConfig` into a concrete `ResolvedLaunchPlan`, handling:
    /// - Chromium & Arc `Local State` profile lookup (by display name, case-insensitive name, email, or dir key)
    /// - Firefox `profiles.ini` (`-P`) and explicit profile directories (`--profile`)
    /// - Firefox Multi-Account Containers (`ext+container:name=...&url=...`)
    /// - Chromium PWAs (`--app-id=...` or `--app=...`)
    /// - Missing profile fallback vs. suppression policy
    pub fn build_launch_plan(
        &self,
        config: &BrowserConfig,
        default_background: bool,
        missing_policy: MissingProfilePolicy,
    ) -> ResolvedLaunchPlan {
        let open_in_background = config.open_in_background.unwrap_or(default_background);

        if config.app_type == AppType::None || config.name.is_empty() {
            return ResolvedLaunchPlan {
                browser_name: String::new(),
                app_type: AppType::None,
                open_in_background,
                creates_new_instance: false,
                activate_after_handoff: false,
                args: Vec::new(),
                final_url: config.url.clone(),
                arc_space: None,
                warning: None,
            };
        }

        let mut final_url = config.url.clone();
        let mut args = Vec::new();
        let mut warning = None;
        let meta = lookup_browser_meta(&config.name);

        // 1. Handle Firefox Multi-Account Container URL rewrite if requested
        if let Some(container_name) = &config.container {
            if !container_name.is_empty() {
                final_url = format!(
                    "ext+container:name={}&url={}",
                    percent_encode_component(container_name),
                    percent_encode_component(&final_url)
                );
            }
        }

        // 2. Resolve profile arguments
        if !config.profile.is_empty() {
            match self.resolve_profile_args(&config.name, &config.profile) {
                Ok(profile_args) => {
                    args.extend(profile_args);
                }
                Err(msg) => {
                    warning = Some(msg);
                    if missing_policy == MissingProfilePolicy::Suppress {
                        return ResolvedLaunchPlan {
                            browser_name: config.name.clone(),
                            app_type: AppType::None,
                            open_in_background,
                            creates_new_instance: false,
                            activate_after_handoff: false,
                            args: Vec::new(),
                            final_url,
                            arc_space: config.space.clone(),
                            warning,
                        };
                    }
                }
            }
        }

        // 3. Handle Chromium PWA `--app-id` or `--app`
        let is_chromium_or_arc = meta
            .map(|m| matches!(m.family, BrowserFamily::Chromium | BrowserFamily::Arc))
            .unwrap_or(true);

        if is_chromium_or_arc {
            if let Some(app_id) = &config.app_id {
                if !app_id.is_empty() {
                    let resolved_id = self.resolve_pwa_app_id(app_id);
                    args.push(format!("--app-id={}", resolved_id));
                }
            } else if config.app_mode == Some(true) {
                args.push(format!("--app={}", final_url));
            }
        }

        // 4. Append user custom args
        for custom in &config.args {
            if custom != "--args" {
                args.push(custom.clone());
            }
        }

        let creates_new_instance = !args.is_empty();
        // Fix for Finicky #501 & #550: when `creates_new_instance` is true and the user
        // wants the browser in the foreground (`!open_in_background`), the transient helper
        // instance forwards the URL over Chromium's SingletonSocket and exits, dropping focus.
        // We mark `activate_after_handoff = true` so the launcher explicitly activates the
        // primary browser process right after handoff.
        let activate_after_handoff = creates_new_instance && !open_in_background;

        ResolvedLaunchPlan {
            browser_name: config.name.clone(),
            app_type: config.app_type,
            open_in_background,
            creates_new_instance,
            activate_after_handoff,
            args,
            final_url,
            arc_space: config.space.clone(),
            warning,
        }
    }

    pub fn resolve_profile_args(
        &self,
        identifier: &str,
        profile: &str,
    ) -> Result<Vec<String>, String> {
        let Some(meta) = lookup_browser_meta(identifier) else {
            return Err(format!(
                "Browser '{}' does not support profile resolution",
                identifier
            ));
        };

        match meta.family {
            BrowserFamily::Chromium | BrowserFamily::Arc => {
                let path = self.local_state_path(meta.config_dir_relative);
                let Some(entries) = self.load_chromium_profiles(&path) else {
                    // If Local State doesn't exist yet, still allow canonical directory names ("Default", "Profile 1")
                    if is_canonical_chromium_dir(profile) {
                        return Ok(vec![format!("--profile-directory={}", profile)]);
                    }
                    return Err(format!(
                        "Could not read Chromium Local State at {}",
                        path.display()
                    ));
                };

                // Priority 1: Exact match on display name (e.g. "Work")
                if let Some(hit) = entries.iter().find(|p| p.display_name == profile) {
                    return Ok(vec![format!("--profile-directory={}", hit.dir_key)]);
                }
                // Priority 2: Case-insensitive match on display name (e.g. "work")
                if let Some(hit) = entries
                    .iter()
                    .find(|p| p.display_name.eq_ignore_ascii_case(profile))
                {
                    return Ok(vec![format!("--profile-directory={}", hit.dir_key)]);
                }
                // Priority 3: Signed-in email / user_name match (e.g. "alice@corp.com")
                if let Some(hit) = entries.iter().find(|p| {
                    !p.user_name.is_empty() && p.user_name.eq_ignore_ascii_case(profile)
                }) {
                    return Ok(vec![format!("--profile-directory={}", hit.dir_key)]);
                }
                // Priority 4: Directory key match ("Default", "Profile 1")
                if let Some(hit) = entries.iter().find(|p| p.dir_key == profile) {
                    return Ok(vec![format!("--profile-directory={}", hit.dir_key)]);
                }

                let available: Vec<&str> = entries
                    .iter()
                    .map(|p| {
                        if p.display_name.is_empty() {
                            p.dir_key.as_str()
                        } else {
                            p.display_name.as_str()
                        }
                    })
                    .collect();
                Err(format!(
                    "Profile '{}' not found for {}. Available profiles: {}",
                    profile,
                    meta.app_name,
                    available.join(", ")
                ))
            }
            BrowserFamily::Firefox => {
                // Support explicit directory paths (`~/...` or `/...`) via `--profile <path>`
                if profile.starts_with('/') || profile.starts_with("~/") {
                    let expanded = if let Some(rest) = profile.strip_prefix("~/") {
                        self.home_dir.join(rest)
                    } else {
                        PathBuf::from(profile)
                    };
                    if expanded.is_dir() {
                        return Ok(vec![
                            "--profile".to_string(),
                            expanded.to_string_lossy().into_owned(),
                        ]);
                    }
                }

                let path = self.firefox_ini_path(meta.config_dir_relative);
                let ini_names = self.load_firefox_profiles(&path).unwrap_or_default();
                let group_profiles = self.load_firefox_group_profiles(meta.config_dir_relative);

                if let Some((_, dir_path)) = group_profiles
                    .iter()
                    .find(|(n, _)| n == profile || n.eq_ignore_ascii_case(profile))
                {
                    return Ok(vec![
                        "--profile".to_string(),
                        dir_path.to_string_lossy().into_owned(),
                    ]);
                }

                if let Some(hit) = ini_names.iter().find(|n| *n == profile) {
                    return Ok(vec!["-P".to_string(), hit.clone()]);
                }
                if let Some(hit) = ini_names.iter().find(|n| n.eq_ignore_ascii_case(profile)) {
                    return Ok(vec!["-P".to_string(), hit.clone()]);
                }

                if ini_names.is_empty() && group_profiles.is_empty() {
                    return Err(format!(
                        "Could not read Firefox profiles.ini at {}",
                        path.display()
                    ));
                }

                let mut available = ini_names;
                for (gn, _) in group_profiles {
                    if !available.contains(&gn) {
                        available.push(gn);
                    }
                }
                Err(format!(
                    "Profile '{}' not found in Firefox profiles. Available profiles: {}",
                    profile,
                    available.join(", ")
                ))
            }
            BrowserFamily::Safari => Err("Safari does not support CLI profile flags".to_string()),
        }
    }

    fn local_state_path(&self, config_dir_relative: &str) -> PathBuf {
        self.home_dir
            .join("Library/Application Support")
            .join(config_dir_relative)
            .join("Local State")
    }

    fn firefox_ini_path(&self, config_dir_relative: &str) -> PathBuf {
        self.home_dir
            .join("Library/Application Support")
            .join(config_dir_relative)
            .join("profiles.ini")
    }

    fn load_chromium_profiles(&self, path: &Path) -> Option<Vec<ChromiumProfileMeta>> {
        let meta = fs::metadata(path).ok()?;
        let mtime = meta
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let size = meta.len();

        if let Some(cached) = self.chromium_cache.read().unwrap().get(path) {
            if cached.mtime_nanos == mtime && cached.size == size {
                return Some(cached.profiles.clone());
            }
        }

        let bytes = fs::read(path).ok()?;
        let parsed: ChromiumLocalState = serde_json::from_slice(&bytes).ok()?;
        let mut profiles: Vec<ChromiumProfileMeta> = parsed
            .profile
            .info_cache
            .into_iter()
            .map(|(dir_key, entry)| {
                let display_name = if !entry.name.is_empty() {
                    entry.name
                } else {
                    entry.gaia_name
                };
                ChromiumProfileMeta {
                    dir_key,
                    display_name,
                    user_name: entry.user_name,
                }
            })
            .collect();
        profiles.sort_by(|a, b| a.dir_key.cmp(&b.dir_key));

        self.chromium_cache.write().unwrap().insert(
            path.to_path_buf(),
            CachedChromiumProfiles {
                mtime_nanos: mtime,
                size,
                profiles: profiles.clone(),
            },
        );
        Some(profiles)
    }

    fn load_firefox_profiles(&self, path: &Path) -> Option<Vec<String>> {
        let meta = fs::metadata(path).ok()?;
        let mtime = meta
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let size = meta.len();

        if let Some(cached) = self.firefox_cache.read().unwrap().get(path) {
            if cached.mtime_nanos == mtime && cached.size == size {
                return Some(cached.ini_names.clone());
            }
        }

        let content = fs::read_to_string(path).ok()?;
        let mut names = Vec::new();
        for line in content.lines() {
            if let Some(name) = line.trim().strip_prefix("Name=") {
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }

        self.firefox_cache.write().unwrap().insert(
            path.to_path_buf(),
            CachedFirefoxProfiles {
                mtime_nanos: mtime,
                size,
                ini_names: names.clone(),
            },
        );
        Some(names)
    }

    fn load_firefox_group_profiles(&self, config_dir_relative: &str) -> Vec<(String, PathBuf)> {
        let root = self
            .home_dir
            .join("Library/Application Support")
            .join(config_dir_relative);
        let group_dir = root.join("Profile Groups");
        let Ok(entries) = fs::read_dir(&group_dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("sqlite") {
                if let Ok(rows) = read_firefox_group_profiles(&path, &root) {
                    out.extend(rows);
                }
            }
        }
        out
    }
}

fn collect_arc_space_titles(val: &serde_json::Value, out: &mut Vec<String>) {
    match val {
        serde_json::Value::Object(map) => {
            if map.contains_key("containerIDs") {
                if let Some(title) = map.get("title").and_then(|t| t.as_str()) {
                    if !title.is_empty() {
                        out.push(title.to_string());
                    }
                }
            }
            for v in map.values() {
                collect_arc_space_titles(v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_arc_space_titles(v, out);
            }
        }
        _ => {}
    }
}

fn extract_plist_string_key(xml: &str, key: &str) -> Option<String> {
    let needle = format!("<key>{}</key>", key);
    let idx = xml.find(&needle)?;
    let after = &xml[idx + needle.len()..];
    let s_start = after.find("<string>")? + "<string>".len();
    let s_end = after[s_start..].find("</string>")?;
    Some(after[s_start..s_start + s_end].trim().to_string())
}

fn is_canonical_chromium_dir(s: &str) -> bool {
    s == "Default"
        || s.strip_prefix("Profile ")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

fn percent_encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_home() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("browsify_profile_test_{}", nanos));
        let chrome_dir = dir.join("Library/Application Support/Google/Chrome");
        let ff_dir = dir.join("Library/Application Support/Firefox");
        fs::create_dir_all(&chrome_dir).unwrap();
        fs::create_dir_all(&ff_dir).unwrap();

        let local_state = r#"{
          "profile": {
            "info_cache": {
              "Default": { "name": "Personal", "user_name": "me@gmail.com" },
              "Profile 1": { "name": "Work", "user_name": "alice@acme.io" },
              "Profile 2": { "name": "Client A", "user_name": "contractor@client.com" }
            }
          }
        }"#;
        fs::write(chrome_dir.join("Local State"), local_state).unwrap();

        let profiles_ini = "[Profile0]\nName=default-release\nPath=Profiles/abc.default-release\n\n[Profile1]\nName=dev-edition\nPath=Profiles/xyz.dev\n";
        fs::write(ff_dir.join("profiles.ini"), profiles_ini).unwrap();
        dir
    }

    #[test]
    fn test_chromium_profile_by_name_email_and_dir() {
        let home = make_temp_home();
        let resolver = ProfileResolver::new(&home);

        // Exact display name
        assert_eq!(
            resolver
                .resolve_profile_args("Google Chrome", "Work")
                .unwrap(),
            vec!["--profile-directory=Profile 1"]
        );
        // Case-insensitive display name
        assert_eq!(
            resolver
                .resolve_profile_args("com.google.Chrome", "work")
                .unwrap(),
            vec!["--profile-directory=Profile 1"]
        );
        // Email / user_name match
        assert_eq!(
            resolver
                .resolve_profile_args("Google Chrome", "alice@acme.io")
                .unwrap(),
            vec!["--profile-directory=Profile 1"]
        );
        // Directory key match
        assert_eq!(
            resolver
                .resolve_profile_args("Google Chrome", "Profile 2")
                .unwrap(),
            vec!["--profile-directory=Profile 2"]
        );

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_firefox_profile_and_containers() {
        let home = make_temp_home();
        let resolver = ProfileResolver::new(&home);

        let mut cfg = BrowserConfig::from_spec_str("Firefox", "https://github.com/myorg");
        cfg.profile = "default-release".to_string();
        cfg.container = Some("Work".to_string());

        let plan = resolver.build_launch_plan(&cfg, false, MissingProfilePolicy::WarnAndFallback);
        assert_eq!(plan.args, vec!["-P", "default-release"]);
        assert_eq!(
            plan.final_url,
            "ext+container:name=Work&url=https%3A%2F%2Fgithub.com%2Fmyorg"
        );
        assert!(plan.creates_new_instance);
        assert!(plan.activate_after_handoff);

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_missing_profile_policy_warn_vs_suppress() {
        let home = make_temp_home();
        let resolver = ProfileResolver::new(&home);

        let mut cfg = BrowserConfig::from_spec_str("Google Chrome", "https://internal.corp");
        cfg.profile = "NonExistent".to_string();

        // WarnAndFallback: logs warning and opens without profile flag
        let warn_plan =
            resolver.build_launch_plan(&cfg, false, MissingProfilePolicy::WarnAndFallback);
        assert_eq!(warn_plan.app_type, AppType::AppName);
        assert!(warn_plan.args.is_empty());
        assert!(warn_plan.warning.is_some());

        // Suppress: refuses to launch
        let suppress_plan =
            resolver.build_launch_plan(&cfg, false, MissingProfilePolicy::Suppress);
        assert_eq!(suppress_plan.app_type, AppType::None);
        assert!(suppress_plan.warning.is_some());

        let _ = fs::remove_dir_all(&home);
    }
}
