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
    thread::spawn(move || {
        if let Some(info) = check_github_releases(&home) {
            set_update_info(info);
        } else {
            set_update_info(UpdateInfo::no_update(format!(
                "v{}",
                env!("CARGO_PKG_VERSION")
            )));
        }
    });
}

/// Queries the latest GitHub release (24h disk-cached, 3s timeout) and reports
/// whether a newer version than the running binary is available.
fn check_github_releases(home_dir: &Path) -> Option<UpdateInfo> {
    let cache_file = update_cache_path(home_dir);
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Ok(bytes) = fs::read(&cache_file) {
        if let Ok(cached) = serde_json::from_slice::<CachedUpdateCheck>(&bytes) {
            if now_secs.saturating_sub(cached.timestamp_secs) < Duration::from_secs(86_400).as_secs()
            {
                return Some(cached.update_info);
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
    let has_update = tag_name != current && !tag_name.is_empty();

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
