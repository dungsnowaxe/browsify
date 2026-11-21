use crate::browser::{
    lookup_browser_meta, AppType, BrowserConfig, BrowserFamily, BrowserMeta, ProfileSelector,
    BROWSERS_REGISTRY,
};
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LaunchAction {
    #[default]
    OpenApplication,
    DiaProfile {
        profile: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedLaunchPlan {
    #[serde(default)]
    pub action: LaunchAction,
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

#[derive(Debug, serde::Serialize)]
pub struct ProfileChoice {
    pub id: String,
    pub label: String,
    pub aliases: Vec<String>,
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
        if meta.id == crate::dia::BUNDLE_ID {
            return crate::dia::list_profiles().unwrap_or_default();
        }
        if meta.id == crate::zen::ZEN_BUNDLE_ID {
            let names = self
                .zen_workspace_profile_dir()
                .ok()
                .and_then(|dir| crate::zen::list_workspaces(&dir).ok())
                .unwrap_or_default();
            return names
                .into_iter()
                .map(|w| format!("workspace:{}", w.name))
                .collect();
        }
        match meta.family {
            BrowserFamily::Chromium => {
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

    /// Preserve discovery errors for interactive consumers instead of showing an empty list.
    pub fn try_list_profiles(&self, identifier: &str) -> Result<Vec<String>, String> {
        if lookup_browser_meta(identifier).is_some_and(|m| m.id == crate::dia::BUNDLE_ID) {
            crate::dia::list_profiles()
        } else if lookup_browser_meta(identifier).is_some_and(|m| m.id == crate::zen::ZEN_BUNDLE_ID)
        {
            let dir = self.zen_workspace_profile_dir()?;
            crate::zen::list_workspaces(&dir).map(|spaces| {
                spaces
                    .into_iter()
                    .map(|w| format!("workspace:{}", w.name))
                    .collect()
            })
        } else {
            Ok(self.list_profiles(identifier))
        }
    }

    /// Choices use stable selectors, independently of what a browser calls a profile.
    pub fn profile_choices(&self, identifier: &str) -> Result<Vec<ProfileChoice>, String> {
        if let Some(meta) = lookup_browser_meta(identifier) {
            if meta.id == crate::zen::ZEN_BUNDLE_ID {
                let workspaces = crate::zen::list_workspaces(&self.zen_workspace_profile_dir()?)?;
                return Ok(workspaces.into_iter().map(|workspace| ProfileChoice {
                    id: format!("workspace:{}", workspace.uuid),
                    aliases: vec![format!("workspace:{}", workspace.name)],
                    label: workspace.name,
                }).collect());
            }
            if meta.family == BrowserFamily::Chromium && meta.id != crate::dia::BUNDLE_ID {
                let entries = self.load_chromium_profiles(&self.local_state_path(meta.config_dir_relative)).unwrap_or_default();
                return Ok(entries.iter().map(|p| {
                    let duplicate = entries.iter().filter(|other| other.display_name.eq_ignore_ascii_case(&p.display_name)).count() > 1;
                    let label = if p.display_name.is_empty() { p.dir_key.clone() }
                        else if duplicate { format!("{} ({})", p.display_name, p.dir_key) }
                        else { p.display_name.clone() };
                    ProfileChoice { id: format!("directory:{}", p.dir_key), label, aliases: vec![p.display_name.clone(), p.dir_key.clone()] }
                }).collect());
            }
        }
        let names = self.try_list_profiles(identifier)?;
        Ok(names.iter().filter(|name| names.iter().filter(|other| other == name).count() == 1)
            .map(|name| ProfileChoice { id: name.clone(), label: name.clone(), aliases: Vec::new() }).collect())
    }

    /// Lists known browser applications, never inferring installation from profile data.
    pub fn list_installed_browsers(&self) -> Vec<&'static BrowserMeta> {
        installed_browsers_in(&[
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
            PathBuf::from("/System/Volumes/Preboot/Cryptexes/App/System/Applications"),
            self.home_dir.join("Applications"),
        ])
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
                action: LaunchAction::OpenApplication,
                browser_name: String::new(),
                app_type: AppType::None,
                open_in_background,
                creates_new_instance: false,
                activate_after_handoff: false,
                args: Vec::new(),
                final_url: config.url.clone(),
                warning: None,
            };
        }

        if lookup_browser_meta(&config.name).is_some_and(|m| m.id == crate::dia::BUNDLE_ID)
            && (!config.profile.is_empty() || config.container.is_some())
        {
            let incompatible = !config.args.is_empty()
                || config.container.is_some()
                || matches!(
                    ProfileSelector::parse(&config.profile),
                    ProfileSelector::Container(_)
                )
                || config.app_mode == Some(true)
                || config.app_id.is_some();
            return ResolvedLaunchPlan {
                action: LaunchAction::DiaProfile { profile: config.profile.clone() },
                browser_name: config.name.clone(),
                app_type: if incompatible { AppType::None } else { config.app_type },
                open_in_background,
                creates_new_instance: false,
                activate_after_handoff: false,
                args: Vec::new(),
                final_url: config.url.clone(),
                warning: incompatible.then(|| "Dia profile targeting cannot be combined with custom arguments, containers or app mode. No URL will be opened.".into()),
            };
        }

        // Zen `workspace:` selectors use the browser's native `--space` flag.
        // Zen still navigates when the workspace name is unknown, so a missing
        // target always suppresses the launch instead of following the
        // configured missing-profile policy.
        if lookup_browser_meta(&config.name).is_some_and(|m| m.id == crate::zen::ZEN_BUNDLE_ID)
            && workspace_selector(&config.profile).is_some()
        {
            return self.build_zen_workspace_plan(config, open_in_background);
        }

        let mut final_url = config.url.clone();
        let mut args = Vec::new();
        let mut warning = None;
        let meta = lookup_browser_meta(&config.name);

        let (effective_profile, effective_container) = match ProfileSelector::parse(&config.profile)
        {
            ProfileSelector::Container(c) => (
                "",
                if !c.is_empty() {
                    Some(c)
                } else {
                    config.container.as_deref()
                },
            ),
            ProfileSelector::Profile(p) => (p, config.container.as_deref()),
        };

        // 1. Handle Firefox Multi-Account Container URL rewrite if requested
        if let Some(container_name) = effective_container {
            if !container_name.is_empty() {
                final_url = format!(
                    "ext+container:name={}&url={}",
                    percent_encode_component(container_name),
                    percent_encode_component(&final_url)
                );
            }
        }

        // 2. Resolve profile arguments
        if !effective_profile.is_empty() {
            match self.resolve_profile_args(&config.name, effective_profile) {
                Ok(profile_args) => {
                    args.extend(profile_args);
                }
                Err(msg) => {
                    warning = Some(msg);
                    if missing_policy == MissingProfilePolicy::Suppress {
                        return ResolvedLaunchPlan {
                            action: LaunchAction::OpenApplication,
                            browser_name: config.name.clone(),
                            app_type: AppType::None,
                            open_in_background,
                            creates_new_instance: false,
                            activate_after_handoff: false,
                            args: Vec::new(),
                            final_url,
                            warning,
                        };
                    }
                }
            }
        }

        // 3. Handle Chromium PWA `--app-id` or `--app`
        let is_chromium = meta
            .map(|m| matches!(m.family, BrowserFamily::Chromium))
            .unwrap_or(true);

        if is_chromium {
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
            action: LaunchAction::OpenApplication,
            browser_name: config.name.clone(),
            app_type: config.app_type,
            open_in_background,
            creates_new_instance,
            activate_after_handoff,
            args,
            final_url,
            warning,
        }
    }

    /// Resolves a Zen `workspace:` selector into the browser's native
    /// `--space` flag. The plan carries the selector verbatim; the launcher
    /// validates it against Zen's session store at dispatch and resolves it to
    /// the stable workspace UUID. A missing or ambiguous workspace suppresses
    /// the launch there regardless of the missing-profile policy: Zen still
    /// navigates when a workspace name is unknown, so falling back would leak
    /// the URL into the active workspace.
    fn build_zen_workspace_plan(
        &self,
        config: &BrowserConfig,
        open_in_background: bool,
    ) -> ResolvedLaunchPlan {
        let selector = workspace_selector(&config.profile).unwrap_or_default();
        let suppressed = |warning: String| ResolvedLaunchPlan {
            action: LaunchAction::OpenApplication,
            browser_name: config.name.clone(),
            app_type: AppType::None,
            open_in_background,
            creates_new_instance: false,
            activate_after_handoff: false,
            args: Vec::new(),
            final_url: config.url.clone(),
            warning: Some(warning),
        };

        if selector.is_empty() {
            return suppressed(
                "Zen workspace selector 'workspace:' needs a workspace name or UUID.".into(),
            );
        }
        if config.container.is_some() {
            return suppressed(
                "Zen workspace targeting cannot be combined with a container. No URL will be opened.".into(),
            );
        }
        if let Some(flag) = config.args.iter().find(|a| {
            a.as_str() == "--space"
                || a.starts_with("--space=")
                || a.as_str() == "--new-window"
                || a.as_str() == "--private-window"
                || a.starts_with("--private-window=")
        }) {
            return suppressed(format!(
                "Zen workspace targeting cannot be combined with the custom argument '{}'. No URL will be opened.",
                flag
            ));
        }

        let mut args = config.args.clone();
        args.push("--space".to_string());
        args.push(selector.to_string());
        ResolvedLaunchPlan {
            action: LaunchAction::OpenApplication,
            browser_name: config.name.clone(),
            app_type: config.app_type,
            open_in_background,
            creates_new_instance: true,
            activate_after_handoff: !open_in_background,
            args,
            final_url: config.url.clone(),
            warning: None,
        }
    }

    /// The Zen profile directory a `workspace:` selector applies to: an
    /// explicit `--profile <dir>` or `-P <name>` custom argument when present,
    /// otherwise the default profile from Zen's `profiles.ini`.
    fn zen_profile_dir(&self, config: &BrowserConfig) -> Result<PathBuf, String> {
        let root = self.home_dir.join("Library/Application Support").join("zen");
        crate::zen::profile_dir(&root, &config.args)
    }

    /// The default Zen profile directory, for discovery and validation without
    /// a Browser Target's custom arguments.
    fn zen_workspace_profile_dir(&self) -> Result<PathBuf, String> {
        let root = self.home_dir.join("Library/Application Support").join("zen");
        crate::zen::profile_dir(&root, &[])
    }

    pub fn resolve_profile_args(
        &self,
        identifier: &str,
        profile: &str,
    ) -> Result<Vec<String>, String> {
        if let ProfileSelector::Container(c) = ProfileSelector::parse(profile) {
            if !c.is_empty() {
                return Ok(Vec::new());
            }
        }

        let Some(meta) = lookup_browser_meta(identifier) else {
            return Err(format!(
                "Browser '{}' does not support profile resolution",
                identifier
            ));
        };

        if meta.id == crate::dia::BUNDLE_ID {
            return Err("Dia profiles use native automation, not Chromium directory flags".into());
        }

        if meta.id == crate::zen::ZEN_BUNDLE_ID && workspace_selector(profile).is_some() {
            return Err(
                "Zen workspaces resolve through the dedicated --space launch path, not profile flags"
                    .into(),
            );
        }

        match meta.family {
            BrowserFamily::Chromium => {
                let path = self.local_state_path(meta.config_dir_relative);
                if let Some(key) = profile.strip_prefix("directory:") {
                    return self.load_chromium_profiles(&path).and_then(|entries| {
                        entries.into_iter().find(|p| p.dir_key == key)
                    }).map(|p| vec![format!("--profile-directory={}", p.dir_key)])
                      .ok_or_else(|| format!("The selected profile is no longer available for {}.", meta.app_name));
                }
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
                if let Some(hit) = entries
                    .iter()
                    .find(|p| !p.user_name.is_empty() && p.user_name.eq_ignore_ascii_case(profile))
                {
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

    pub fn validate_profile(&self, identifier: &str, profile: &str) -> Result<(), String> {
        if lookup_browser_meta(identifier).is_some_and(|m| m.id == crate::dia::BUNDLE_ID) {
            crate::dia::validate_profile(profile)
        } else if lookup_browser_meta(identifier).is_some_and(|m| m.id == crate::zen::ZEN_BUNDLE_ID)
            && workspace_selector(profile).is_some()
        {
            let dir = self.zen_workspace_profile_dir()?;
            crate::zen::resolve_workspace(&dir, workspace_selector(profile).unwrap_or_default())
                .map(|_| ())
        } else {
            self.resolve_profile_args(identifier, profile).map(|_| ())
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

/// The payload of a `workspace:` Profile Selector, when present.
pub fn workspace_selector(profile: &str) -> Option<&str> {
    profile.strip_prefix("workspace:")
}

fn expand_home(value: &str, home_dir: &Path) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/") {
        home_dir.join(rest)
    } else {
        PathBuf::from(value)
    }
}

/// Parses `profiles.ini` into `(name, path, is_relative, is_default)` tuples
/// plus the install-section `Default=` path when present.
fn parse_firefox_profiles_ini(text: &str) -> (Vec<(String, String, bool, bool)>, Option<String>) {
    let mut profiles = Vec::new();
    let mut current: Option<(String, String, bool, bool)> = None;
    let mut install_default = None;
    let mut in_install_section = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Some(entry) = current.take() {
                profiles.push(entry);
            }
            in_install_section = section.trim().to_ascii_lowercase().starts_with("install");
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        let value = value.trim();
        match (in_install_section, key) {
            (true, "Default") => install_default = Some(value.to_string()),
            (false, "Name") => {
                current.get_or_insert_with(default_ini_entry).0 = value.to_string();
            }
            (false, "Path") => {
                current.get_or_insert_with(default_ini_entry).1 = value.to_string();
            }
            (false, "IsRelative") => {
                current.get_or_insert_with(default_ini_entry).2 = value == "1";
            }
            (false, "Default") => {
                if value == "1" {
                    current.get_or_insert_with(default_ini_entry).3 = true;
                }
            }
            _ => {}
        }
    }
    if let Some(entry) = current.take() {
        profiles.push(entry);
    }
    (profiles, install_default)
}

fn default_ini_entry() -> (String, String, bool, bool) {
    (String::new(), String::new(), true, false)
}

fn extract_plist_string_key(xml: &str, key: &str) -> Option<String> {    let needle = format!("<key>{}</key>", key);
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

fn installed_browsers_in(roots: &[PathBuf]) -> Vec<&'static BrowserMeta> {
    BROWSERS_REGISTRY
        .iter()
        .filter(|browser| {
            roots
                .iter()
                .any(|root| root.join(format!("{}.app", browser.app_name)).is_dir())
        })
        .collect()
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
    fn installed_browsers_require_applications_not_leftover_profiles() {
        let home = make_temp_home();
        let applications = home.join("Applications");
        fs::create_dir_all(&applications).unwrap();
        assert!(installed_browsers_in(&[applications.clone()]).is_empty());
        fs::create_dir(applications.join("Firefox.app")).unwrap();
        assert_eq!(
            installed_browsers_in(&[applications.clone()])
                .iter()
                .map(|browser| browser.app_name)
                .collect::<Vec<_>>(),
            ["Firefox"]
        );
        fs::remove_dir(applications.join("Firefox.app")).unwrap();
        assert!(installed_browsers_in(&[applications]).is_empty());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn dia_native_plan_preserves_data_and_rejects_conflicting_contexts() {
        let resolver = ProfileResolver::new("/nonexistent-fixture-home");
        let mut cfg = BrowserConfig::from_spec_str(
            "company.thebrowser.dia",
            "https://example.test/a?x=1&y=%22",
        );
        cfg.profile = "Team \"A\"\nΔ".into();
        let plan = resolver.build_launch_plan(&cfg, true, MissingProfilePolicy::WarnAndFallback);
        assert_eq!(
            plan.action,
            LaunchAction::DiaProfile {
                profile: cfg.profile.clone()
            }
        );
        assert_eq!(plan.final_url, cfg.url);
        assert!(plan.open_in_background);
        assert!(plan.args.is_empty());
        assert!(!plan.creates_new_instance);
        let roundtrip: ResolvedLaunchPlan =
            serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
        assert_eq!(roundtrip, plan);
        for conflict in 0..5 {
            let mut invalid = cfg.clone();
            match conflict {
                0 => invalid.args.push("--profile-directory=Default".into()),
                1 => invalid.container = Some("Personal".into()),
                2 => invalid.profile = "container:Personal".into(),
                3 => invalid.app_mode = Some(true),
                _ => invalid.app_id = Some("app".into()),
            }
            let rejected =
                resolver.build_launch_plan(&invalid, false, MissingProfilePolicy::WarnAndFallback);
            assert_eq!(rejected.app_type, AppType::None);
            assert!(rejected.warning.is_some());
            assert_eq!(rejected.final_url, cfg.url);
        }
        cfg.profile.clear();
        let default = resolver.build_launch_plan(&cfg, false, MissingProfilePolicy::Suppress);
        assert_eq!(default.action, LaunchAction::OpenApplication);
        assert_eq!(default.app_type, AppType::BundleId);
    }

    #[test]
    fn dia_profiles_are_not_chromium_directory_selectors() {
        let home = make_temp_home();
        let resolver = ProfileResolver::new(&home);
        assert!(resolver.resolve_profile_args("Dia", "Profile 1").is_err());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn zen_workspace_plan_carries_native_space_flag_and_rejects_conflicts() {
        let home = make_temp_home();
        let resolver = ProfileResolver::new(&home);
        let mut cfg = BrowserConfig::from_spec_str("Zen", "https://example.test/a?x=1&y=%22");
        cfg.profile = "workspace:Research \"Δ\"".into();
        for policy in [MissingProfilePolicy::WarnAndFallback, MissingProfilePolicy::Suppress] {
            let plan = resolver.build_launch_plan(&cfg, true, policy);
            assert_eq!(plan.action, LaunchAction::OpenApplication);
            assert_eq!(plan.args, vec!["--space", "Research \"Δ\""]);
            assert_eq!(plan.final_url, cfg.url);
            assert!(plan.creates_new_instance);
            assert!(plan.warning.is_none());
            assert_eq!(plan.app_type, AppType::AppName);
        }

        // Empty selector, container and conflicting custom flags suppress.
        let mut invalid = cfg.clone();
        invalid.profile = "workspace:".into();
        let rejected = resolver.build_launch_plan(&invalid, false, MissingProfilePolicy::WarnAndFallback);
        assert_eq!(rejected.app_type, AppType::None);
        assert!(rejected.warning.is_some());
        for conflict in 0..3 {
            let mut invalid = cfg.clone();
            match conflict {
                0 => invalid.args.push("--space=Other".into()),
                1 => invalid.container = Some("Personal".into()),
                _ => invalid.args.push("--new-window".into()),
            }
            let rejected =
                resolver.build_launch_plan(&invalid, false, MissingProfilePolicy::WarnAndFallback);
            assert_eq!(rejected.app_type, AppType::None);
            assert!(rejected.warning.is_some());
            assert_eq!(rejected.final_url, cfg.url);
            assert!(rejected.args.is_empty());
        }

        // Custom --profile arguments are preserved for dispatch-time validation.
        let mut explicit = cfg.clone();
        explicit.args.push("--profile".into());
        explicit.args.push("/tmp/browsify-zen-test-profile".into());
        let plan = resolver.build_launch_plan(&explicit, true, MissingProfilePolicy::WarnAndFallback);
        assert_eq!(plan.args.first().map(|s| s.as_str()), Some("--profile"));
        assert!(plan.args.contains(&"--space".to_string()));

        // Untargeted Zen keeps the plain application-open path.
        let mut untargeted = cfg.clone();
        untargeted.profile.clear();
        let plain = resolver.build_launch_plan(&untargeted, false, MissingProfilePolicy::Suppress);
        assert_eq!(plain.action, LaunchAction::OpenApplication);
        assert_eq!(plain.app_type, AppType::AppName);
        assert!(plain.args.is_empty());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn zen_workspace_resolution_uses_uuid_then_unique_case_insensitive_name() {
        let home = make_temp_home();
        let zen_root = home.join("Library/Application Support/zen");
        let profile_dir = zen_root.join("Profiles/testws");
        fs::create_dir_all(&profile_dir).unwrap();
        let spaces = serde_json::json!({"spaces": [
            {"uuid": "uuid-a", "name": "Alpha"},
            {"uuid": "uuid-b", "name": "beta"},
            {"uuid": "uuid-c", "name": "beta"}
        ]});
        fs::write(
            profile_dir.join("zen-sessions.jsonlz4"),
            build_mozlz4(&spaces.to_string()),
        )
        .unwrap();
        fs::write(
            zen_root.join("profiles.ini"),
            "[Profile0]\nName=main\nIsRelative=1\nPath=Profiles/testws\nDefault=1\n",
        )
        .unwrap();

        let resolver = ProfileResolver::new(&home);
        assert_eq!(
            resolver.try_list_profiles("Zen").unwrap(),
            vec!["workspace:Alpha", "workspace:beta", "workspace:beta"]
        );
        assert!(resolver.validate_profile("Zen", "workspace:uuid-b").is_ok());
        assert!(resolver.validate_profile("Zen", "workspace:ALPHA").is_ok());
        assert!(resolver.validate_profile("Zen", "workspace:beta").is_err()); // ambiguous
        assert!(resolver
            .validate_profile("Zen", "workspace:missing")
            .is_err());

        let mut cfg = BrowserConfig::from_spec_str("Zen", "https://example.test/");
        cfg.profile = "workspace:alpha".into();
        let plan = resolver.build_launch_plan(&cfg, false, MissingProfilePolicy::Suppress);
        assert_eq!(plan.args, vec!["--space", "alpha"]);
        fs::remove_dir_all(home).unwrap();
    }

    /// Builds a minimal `mozLz40\0` archive with literals-only LZ4 framing so
    /// the resolver's decompressor can be exercised without a dependency.
    fn build_mozlz4(json: &str) -> Vec<u8> {
        let data = json.as_bytes();
        let mut out = Vec::with_capacity(data.len() + data.len() / 255 + 16);
        out.extend_from_slice(b"mozLz40\0");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        let n = data.len();
        if n < 15 {
            out.push((n as u8) << 4);
        } else {
            out.push(15 << 4);
            let mut rem = n - 15;
            while rem >= 255 {
                out.push(255);
                rem -= 255;
            }
            out.push(rem as u8);
        }
        out.extend_from_slice(data);
        out
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
        // A picker selection uses the directory, not the potentially ambiguous display name.
        let choices = resolver.profile_choices("com.google.Chrome").unwrap();
        let work = choices.iter().find(|choice| choice.label == "Work").unwrap();
        assert_eq!(resolver.resolve_profile_args("com.google.Chrome", &work.id).unwrap(), ["--profile-directory=Profile 1"]);
        assert!(resolver.resolve_profile_args("com.google.Chrome", "directory:Profile 999").is_err());

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

        // Also test `profile: "container:Personal"` prefix syntax (used in rules.json / UI)
        let mut cfg_prefix = BrowserConfig::from_spec_str("Firefox", "https://mozilla.org");
        cfg_prefix.profile = "container:Personal".to_string();
        let plan_prefix =
            resolver.build_launch_plan(&cfg_prefix, false, MissingProfilePolicy::WarnAndFallback);
        assert!(plan_prefix.args.is_empty());
        assert_eq!(
            plan_prefix.final_url,
            "ext+container:name=Personal&url=https%3A%2F%2Fmozilla.org"
        );

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
        let suppress_plan = resolver.build_launch_plan(&cfg, false, MissingProfilePolicy::Suppress);
        assert_eq!(suppress_plan.app_type, AppType::None);
        assert!(suppress_plan.warning.is_some());

        let _ = fs::remove_dir_all(&home);
    }
}
