//! Route Dia's native profiles independently of Chromium directory selectors.
//! Query only the running application's scripting interface, never its private stores.

pub const BUNDLE_ID: &str = "company.thebrowser.dia";

pub fn list_profiles() -> Result<Vec<String>, String> {
    let json = run_script("JavaScript", include_str!("dia_profiles.js"), &[])?;
    serde_json::from_str(&json).map_err(|_| "Dia returned invalid profile metadata".into())
}

pub fn validate_profile(profile: &str) -> Result<(), String> {
    match list_profiles()?
        .iter()
        .filter(|name| *name == profile)
        .count()
    {
        1 => Ok(()),
        0 => Err("Dia profile not found. Use its exact name and keep a Dia window open.".into()),
        _ => Err("Dia profile name is ambiguous. Give the profiles unique names.".into()),
    }
}

pub fn open_profile(profile: &str, url: &str, background: bool) -> Result<(), String> {
    let result = run_script(
        "AppleScript",
        include_str!("dia_open.applescript"),
        &[profile, url, if background { "false" } else { "true" }],
    )?;
    match result.trim() {
        "opened" => Ok(()),
        "not-running" | "no-window" => Err("Open a Dia window before routing to a profile.".into()),
        "missing-profile" => {
            Err("Dia profile not found. Use its exact name; no URL was opened.".into())
        }
        "ambiguous-profile" => Err("Dia profile name is ambiguous; no URL was opened.".into()),
        _ => Err("Dia did not acknowledge the targeted tab. Do not retry automatically.".into()),
    }
}

#[cfg(target_os = "macos")]
fn run_script(language: &str, script: &str, args: &[&str]) -> Result<String, String> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    // Dia can reject overlapping Apple-event tab creation. Coordinate CLI and daemon
    // processes, but do not claim this prevents changes made in Dia's own UI.
    let cache = std::path::PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unavailable")?)
        .join("Library/Caches/Browsify");
    std::fs::create_dir_all(&cache).map_err(|_| "Could not create Dia handoff lock directory")?;
    let _handoff_lock = acquire_handoff_lock(&cache.join("dia-handoff.lock"))?;

    // All user data travels in argv, never in generated source or a shell command.
    let mut child = Command::new("/usr/bin/osascript")
        .args(["-l", language, "-e", script, "--"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not start Dia automation".to_string())?;
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            outcome => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(if outcome.is_err() {
                    "Could not observe Dia automation completion. Do not retry automatically."
                } else {
                    "Dia automation timed out. Delivery is uncertain; do not retry automatically."
                }
                .into());
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|_| "Could not read Dia automation result".to_string())?;
    if !output.status.success() {
        // Apple-event errors may contain the destination URL. Keep them out of logs.
        return Err("Dia automation failed. Keep Dia open and check macOS Automation permission for Browsify. Delivery may be uncertain; do not retry automatically.".into());
    }
    String::from_utf8(output.stdout).map_err(|_| "Invalid Dia automation response".into())
}

#[cfg(not(target_os = "macos"))]
fn run_script(_: &str, _: &str, _: &[&str]) -> Result<String, String> {
    Err("Dia profile automation requires macOS".into())
}

#[cfg(target_os = "macos")]
fn acquire_handoff_lock(path: &std::path::Path) -> Result<std::fs::File, String> {
    use std::fs::{OpenOptions, TryLockError};
    use std::os::unix::fs::OpenOptionsExt;
    use std::time::{Duration, Instant};
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| "Could not open Dia handoff lock")?;
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(TryLockError::WouldBlock) => {
                return Err("Dia handoff queue is busy; this request was not sent.".into())
            }
            Err(TryLockError::Error(_)) => {
                return Err("Could not lock Dia handoff queue; this request was not sent.".into())
            }
        }
    }
}
