use crate::config_loader::BrowsifyEngine;
use crate::ui_server::{set_update_info, UpdateInfo};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedUpdateCheck {
    timestamp_secs: u64,
    update_info: UpdateInfo,
}

fn update_cache_path(home_dir: &Path) -> PathBuf {
    home_dir
        .join("Library/Caches/Browsify")
        .join("last_update_check.json")
}

/// Checks or updates the GitHub Releases update status whenever configuration is loaded or updated.
pub fn trigger_update_check_if_enabled(engine: &Arc<Mutex<BrowsifyEngine>>, home_dir: &Path) {
    let check_enabled = engine
        .lock()
        .map(|e| e.summary().options.check_for_updates)
        .unwrap_or(false);
    if !check_enabled {
        set_update_info(UpdateInfo::disabled());
        return;
    }
    let home = home_dir.to_path_buf();
    let engine = Arc::clone(engine);
    thread::spawn(move || {
        if let Some(info) = check_github_releases(&home, false) {
            if engine
                .lock()
                .map(|e| e.summary().options.check_for_updates)
                .unwrap_or(false)
            {
                set_update_info(info);
            }
        } else {
            crate::ui_server::log_event(
                crate::ui_server::LogLevel::Warn,
                "Could not check for updates. Try again later.",
            );
        }
    });
}

/// Queries the latest GitHub release (24h disk-cached, 3s timeout) and reports
/// whether a newer version than the running binary is available.
pub fn manual_update_check(home_dir: &Path) -> serde_json::Value {
    match check_github_releases(home_dir, true) {
        Some(info) => {
            set_update_info(info.clone());
            serde_json::json!({"type":"updateInfo", "message":info})
        }
        None => {
            serde_json::json!({"type":"updateCheckError", "message":{"error":"Could not check for updates. Check your connection and try again."}})
        }
    }
}

fn release_is_newer(release: &str, current: &str) -> Option<bool> {
    fn parts(version: &str) -> Option<([u64; 3], bool)> {
        let version = version.trim_start_matches('v').split('+').next()?;
        let (core, prerelease) = version
            .split_once('-')
            .map(|(core, _)| (core, true))
            .unwrap_or((version, false));
        let parts: Vec<u64> = core
            .split('.')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        Some((parts.try_into().ok()?, prerelease))
    }
    let (release, release_pre) = parts(release)?;
    let (current, current_pre) = parts(current)?;
    Some(release > current || (release == current && current_pre && !release_pre))
}

fn check_github_releases(home_dir: &Path, force: bool) -> Option<UpdateInfo> {
    let cache_file = update_cache_path(home_dir);
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Some(bytes) = (!force).then(|| fs::read(&cache_file).ok()).flatten() {
        if let Ok(cached) = serde_json::from_slice::<CachedUpdateCheck>(&bytes) {
            if now_secs.saturating_sub(cached.timestamp_secs) < Duration::from_secs(86_400).as_secs()
            {
                let mut info = cached.update_info;
                info.has_update = release_is_newer(&info.version, env!("CARGO_PKG_VERSION"))?;
                return Some(info);
            }
        }
    }

    let output = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "3",
            "-H",
            "Accept: application/vnd.github+json",
            "-A",
            &format!("Browsify/{}", env!("CARGO_PKG_VERSION")),
            "https://api.github.com/repos/snowaxe/browsify/releases/latest",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let tag_name = json.get("tag_name")?.as_str()?.trim_start_matches('v');
    let html_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/snowaxe/browsify/releases")
        .to_string();
    let current = env!("CARGO_PKG_VERSION").trim_start_matches('v');
    let has_update = release_is_newer(tag_name, current)?;

    let info = UpdateInfo::available(
        format!("v{}", tag_name),
        has_update,
        html_url.clone(),
        html_url,
    );

    if let Some(parent) = cache_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_vec(&CachedUpdateCheck {
        timestamp_secs: now_secs,
        update_info: info.clone(),
    }) {
        let _ = fs::write(&cache_file, serialized);
    }

    Some(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn manual_checks_bypass_cache_and_report_network_failures() {
        use std::os::unix::fs::PermissionsExt;
        if let Some(root) = std::env::var_os("BROWSIFY_TEST_UPDATE_FIXTURE") {
            let root = PathBuf::from(root);
            let cache = update_cache_path(&root);
            fs::create_dir_all(cache.parent().unwrap()).unwrap();
            let cached = CachedUpdateCheck {
                timestamp_secs: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                update_info: UpdateInfo::available(
                    "v0.1.0".into(),
                    false,
                    String::new(),
                    String::new(),
                ),
            };
            fs::write(&cache, serde_json::to_vec(&cached).unwrap()).unwrap();
            assert_eq!(
                check_github_releases(&root, false).unwrap().version,
                "v0.1.0"
            );
            let checked = manual_update_check(&root);
            assert_eq!(checked["type"], "updateInfo");
            assert_eq!(
                checked["message"]["version"], "v0.2.0",
                "manual checks must ignore fresh cached results"
            );
            fs::write(root.join("bin/curl"), "#!/bin/sh\nexit 7\n").unwrap();
            let failed = manual_update_check(&root);
            assert_eq!(
                failed["type"], "updateCheckError",
                "a failed check is not an up-to-date result"
            );
            return;
        }
        let root = std::env::temp_dir().join(format!(
            "browsify-update-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("bin")).unwrap();
        let curl = root.join("bin/curl");
        fs::write(&curl, "#!/bin/sh\nprintf '%s' '{\"tag_name\":\"v0.2.0\",\"html_url\":\"https://example.invalid/release\"}'\n").unwrap();
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o700)).unwrap();
        // Isolate the fake external command in a child test process, never mutating
        // PATH for parallel tests and never making a real network request.
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "update::tests::manual_checks_bypass_cache_and_report_network_failures",
                "--nocapture",
            ])
            .env("BROWSIFY_TEST_UPDATE_FIXTURE", &root)
            .env("PATH", root.join("bin"))
            .output()
            .unwrap();
        fs::remove_dir_all(root).unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn update_checks_only_offer_newer_release_versions() {
        for (release, current, expected) in [
            ("v0.2.0", "0.1.0", Some(true)),
            ("0.10.0", "0.9.0", Some(true)),
            ("0.1.0", "0.2.0", Some(false)),
            ("v0.1.0", "0.1.0", Some(false)),
            ("0.1.0", "0.1.0-beta.1", Some(true)),
            ("unexpected", "0.1.0", None),
        ] {
            assert_eq!(
                release_is_newer(release, current),
                expected,
                "{release} vs {current}"
            );
        }
    }
}
