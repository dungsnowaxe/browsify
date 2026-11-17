//! Signed, opt-out background updates. No downloaded code runs before verification.
use crate::config_loader::BrowsifyEngine;
use crate::ui_server::{log_event, set_update_info, LogLevel, UpdateInfo};
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DAY: u64 = 86_400;
const MAX_ARCHIVE: u64 = 128 * 1024 * 1024;
const RELEASES: &str = "https://api.github.com/repos/dungsnowaxe/browsify/releases/latest";
const PUBLIC_KEY: &str = match option_env!("BROWSIFY_UPDATE_PUBLIC_KEY") {
    Some(key) => key,
    None => "",
};

#[derive(Default, Serialize, Deserialize)]
struct Preferences {
    last_check: u64,
    skipped: Option<String>,
    remind_after: u64,
    #[serde(default)]
    deferred_version: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Release {
    version: String,
    url: String,
    archive: String,
    signature: String,
}

#[derive(Default)]
struct State {
    enabled: bool,
    generation: u64,
    busy: bool,
    activities: usize,
    manual: bool,
    restarting: bool,
    handoff: Option<PathBuf>,
    release: Option<Release>,
    staged: Option<StagedUpdate>,
    initialized: bool,
    info: Option<UpdateInfo>,
}

#[derive(Serialize, Deserialize)]
struct SavedUpdate {
    directory: PathBuf,
    release: Release,
    manual: bool,
}

struct StagedUpdate {
    manual: bool,
    directory: PathBuf,
    record: PathBuf,
    keep_files: bool,
}
impl StagedUpdate {
    fn path(&self) -> &Path {
        &self.directory
    }
    fn keep(mut self) -> PathBuf {
        self.keep_files = true;
        let _ = fs::remove_file(&self.record);
        self.directory.clone()
    }
}
impl Drop for StagedUpdate {
    fn drop(&mut self) {
        if !self.keep_files {
            let _ = fs::remove_dir_all(&self.directory);
            let _ = fs::remove_file(&self.record);
        }
    }
}
fn stage_root(home: &Path) -> PathBuf {
    home.join("Library/Caches/Browsify/updates")
}
fn stage_record(home: &Path) -> PathBuf {
    stage_root(home).join("ready.json")
}

impl State {
    fn restore(&mut self, home: &Path) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        let record = stage_record(home);
        let Some(saved) = fs::read(&record)
            .ok()
            .and_then(|b| serde_json::from_slice::<SavedUpdate>(&b).ok())
        else {
            return;
        };
        // Never use a persisted path to clean up outside our owned staging directory.
        if saved.directory.parent() != Some(stage_root(home).as_path())
            || !saved
                .directory
                .file_name()
                .map(|n| n.to_string_lossy().starts_with("browsify-update-"))
                .unwrap_or(false)
        {
            let _ = fs::remove_file(record);
            return;
        }
        let stage = StagedUpdate {
            manual: saved.manual,
            directory: saved.directory,
            record,
            keep_files: false,
        };
        if (!self.enabled && !saved.manual)
            || !newer(&saved.release.version, env!("CARGO_PKG_VERSION"))
        {
            return;
        }
        if verify(
            &stage.path().join("update.tar.gz"),
            &stage.path().join("update.sig"),
            PUBLIC_KEY,
        )
        .is_err()
        {
            return;
        }
        self.release = Some(saved.release);
        self.manual = saved.manual;
        self.staged = Some(stage);
        publish(self, "ready", None);
    }

    fn begin_check(&mut self, home: &Path, manual: bool) -> Option<u64> {
        if self.busy || self.restarting {
            return None;
        }
        if !manual && (!self.enabled || now().saturating_sub(preferences(home).last_check) < DAY) {
            return None;
        }
        self.busy = true;
        self.manual = manual;
        self.generation += 1;
        publish(self, "checking", None);
        Some(self.generation)
    }

    fn configure(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            if !enabled && !self.restarting {
                if self.staged.as_ref().map(|s| !s.manual).unwrap_or(false) {
                    self.staged = None;
                }
                if !self.manual {
                    self.generation += 1;
                    self.busy = false;
                    if self.staged.is_some() {
                        publish(self, "ready", None);
                    } else {
                        self.release = None;
                        publish(self, "disabled", None);
                    }
                } else if let Some(info) = self.info.clone() {
                    let status = if info.status == "ready" && self.staged.is_none() {
                        "available"
                    } else {
                        &info.status
                    };
                    publish(self, status, info.error);
                }
            } else if let Some(info) = self.info.clone() {
                publish(self, &info.status, info.error);
            }
        } else if self.info.is_none() && !enabled {
            publish(self, "disabled", None);
        }
    }
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();
fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State::default()))
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn preferences_path(home: &Path) -> PathBuf {
    home.join("Library/Application Support/Browsify/update-preferences.json")
}
fn preferences(home: &Path) -> Preferences {
    fs::read(preferences_path(home))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}
fn save_preferences(home: &Path, prefs: &Preferences) -> Result<(), String> {
    let path = preferences_path(home);
    let parent = path.parent().unwrap();
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut file, prefs).map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
fn publish(s: &mut State, phase: &str, error: Option<String>) {
    let mut info = match &s.release {
        Some(r) => UpdateInfo::available(r.version.clone(), true, r.url.clone(), r.url.clone()),
        None if !s.enabled => UpdateInfo::disabled(),
        None => UpdateInfo::no_update(format!("v{}", env!("CARGO_PKG_VERSION"))),
    };
    info.update_check_enabled = s.enabled;
    info.status = phase.to_owned();
    info.error = error;
    s.info = Some(info.clone());
    set_update_info(info);
}

pub fn is_busy() -> bool {
    let s = state().lock().unwrap();
    s.busy || s.restarting || s.activities > 0
}

/// Called on startup and configuration changes. Disabling invalidates running automatic work.
pub fn trigger_update_check_if_enabled(engine: &Arc<Mutex<BrowsifyEngine>>, home: &Path) {
    let enabled = engine
        .lock()
        .map(|e| e.summary().options.check_for_updates)
        .unwrap_or(false);
    {
        let mut s = state().lock().unwrap();
        s.configure(enabled);
        s.restore(home);
    }
    if enabled {
        start_check(home, false);
    }
}

pub fn check_manually(home: &Path) {
    start_check(home, true);
}

struct UpdateActivity;
impl Drop for UpdateActivity {
    fn drop(&mut self) {
        state().lock().unwrap().activities -= 1;
    }
}

fn start_check(home: &Path, manual: bool) {
    let mut s = state().lock().unwrap();
    s.restore(home);
    let Some(generation) = s.begin_check(home, manual) else {
        return;
    };
    s.activities += 1;
    let home = home.to_owned();
    thread::spawn(move || {
        let _activity = UpdateActivity;
        let result = fetch_release(generation);
        let mut s = state().lock().unwrap();
        if s.generation != generation {
            return;
        }
        s.busy = false;
        let mut prefs = preferences(&home);
        if !manual {
            prefs.last_check = now();
        }
        let _ = save_preferences(&home, &prefs);
        match result {
            Ok(release) => {
                let same =
                    s.release.as_ref().map(|r| &r.version) == release.as_ref().map(|r| &r.version);
                if !same {
                    s.staged = None;
                }
                s.release = release;
                if s.release.is_none() {
                    publish(&mut s, "upToDate", None);
                    drop(s);
                    if manual {
                        notice(
                            "You're up to date",
                            "You have the latest stable Browsify release.",
                        );
                    }
                    return;
                }
                let r = s.release.clone().unwrap();
                let ready = s.staged.is_some();
                publish(&mut s, if ready { "ready" } else { "available" }, None);
                let suppressed = !manual
                    && (prefs.skipped.as_deref() == Some(&r.version)
                        || (prefs.deferred_version.as_deref() == Some(&r.version)
                            && prefs.remind_after > now()));
                drop(s);
                if suppressed {
                    return;
                }
                if ready {
                    prompt_ready(&home, generation, &r);
                } else if manual {
                    let answer = dialog_with_generation(
                        &format!("Browsify {} is available", r.version),
                        "Download the update? Installation will require your approval.",
                        &["Later", "Release Notes", "Download Update"],
                        Some(generation),
                    );
                    if answer.as_deref() == Some("Release Notes") {
                        open_notes(&r);
                    }
                    if answer.as_deref() == Some("Download Update") {
                        download(&home, true);
                    }
                } else {
                    download(&home, false);
                }
            }
            Err(error) => {
                publish(&mut s, "error", Some(error.clone()));
                drop(s);
                log_event(LogLevel::Warn, format!("Update check failed: {error}"));
                if manual {
                    notice("Unable to check for updates", &error);
                }
            }
        }
    });
}

fn active(generation: u64) -> bool {
    state().lock().unwrap().generation == generation
}

/// Download only HTTPS, with bounded size/time and polling cancellation (including reaping curl).
fn fetch(url: &str, path: &Path, max_size: u64, generation: u64) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Update URL must use HTTPS".into());
    }
    let mut child = Command::new("/usr/bin/curl")
        .args([
            "-fLsS",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "10",
            "--max-time",
            "300",
            "--max-filesize",
            &max_size.to_string(),
            "-H",
            "Accept: application/vnd.github+json",
            "-A",
            concat!("Browsify/", env!("CARGO_PKG_VERSION")),
            "-o",
        ])
        .arg(path)
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    loop {
        if !active(generation)
            || fs::metadata(path)
                .map(|m| m.len() > max_size)
                .unwrap_or(false)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Update transfer cancelled or exceeded size limit".into());
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err("Update transfer failed. Check your connection and try again.".into())
            };
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn newer(candidate: &str, current: &str) -> bool {
    match (
        semver::Version::parse(candidate.trim_start_matches('v')),
        semver::Version::parse(current.trim_start_matches('v')),
    ) {
        (Ok(a), Ok(b)) => a.pre.is_empty() && a > b,
        _ => false,
    }
}
fn parse_release(bytes: &[u8]) -> Result<Option<Release>, String> {
    let json: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let tag = json["tag_name"].as_str().ok_or("Release has no version")?;
    if json["draft"] == true || json["prerelease"] == true || !newer(tag, env!("CARGO_PKG_VERSION"))
    {
        return Ok(None);
    }
    let assets = json["assets"].as_array().ok_or("Release has no assets")?;
    let asset = |name: &str| -> Result<String, String> {
        let url = assets
            .iter()
            .find(|a| a["name"] == name)
            .and_then(|a| a["browser_download_url"].as_str())
            .ok_or_else(|| {
                format!("Release is missing {name}; install it manually from GitHub Releases.")
            })?;
        let prefix = format!("https://github.com/dungsnowaxe/browsify/releases/download/{tag}/");
        if !url.starts_with(&prefix) {
            return Err("Unexpected release asset origin".into());
        }
        Ok(url.to_owned())
    };
    Ok(Some(Release {
        version: tag.to_owned(),
        url: format!("https://github.com/dungsnowaxe/browsify/releases/tag/{tag}"),
        archive: asset("Browsify-universal.tar.gz")?,
        signature: asset("Browsify-universal.tar.gz.sig")?,
    }))
}
fn fetch_release(generation: u64) -> Result<Option<Release>, String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = temp.path().join("release.json");
    fetch(RELEASES, &path, 1024 * 1024, generation)?;
    parse_release(&fs::read(path).map_err(|e| e.to_string())?)
}

fn verify(archive: &Path, signature: &Path, public_key: &str) -> Result<(), String> {
    let key: [u8; 32] = STANDARD
        .decode(public_key.trim())
        .map_err(|_| {
            "Release verification is not configured; install manually from GitHub Releases."
        })?
        .try_into()
        .map_err(|_| "Invalid update public key")?;
    let key = VerifyingKey::from_bytes(&key).map_err(|_| "Invalid update public key")?;
    let mut signature_bytes = Vec::new();
    fs::File::open(signature)
        .map_err(|e| e.to_string())?
        .take(65)
        .read_to_end(&mut signature_bytes)
        .map_err(|e| e.to_string())?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| "Invalid update signature")?;
    let mut bytes = Vec::new();
    fs::File::open(archive)
        .map_err(|e| e.to_string())?
        .take(MAX_ARCHIVE + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_ARCHIVE {
        return Err("Update archive is too large".into());
    }
    key.verify_strict(&bytes, &signature)
        .map_err(|_| "Update signature verification failed. Nothing was installed.".into())
}

pub fn download(home: &Path, manual: bool) {
    let mut s = state().lock().unwrap();
    if s.busy || s.restarting || (!manual && !s.enabled) {
        return;
    }
    let Some(release) = s.release.clone() else {
        return;
    };
    s.busy = true;
    s.manual = manual;
    s.generation += 1;
    let generation = s.generation;
    publish(&mut s, "downloading", None);
    s.activities += 1;
    let home = home.to_owned();
    thread::spawn(move || {
        let _activity = UpdateActivity;
        let result = (|| {
            installed_bundle()?;
            if PUBLIC_KEY.is_empty() {
                return Err("This build has no update verification key. Download and install manually from GitHub Releases.".into());
            }
            let root = stage_root(&home);
            fs::create_dir_all(&root).map_err(|e| e.to_string())?;
            let stage = tempfile::Builder::new()
                .prefix("browsify-update-")
                .tempdir_in(&root)
                .map_err(|e| e.to_string())?;
            fetch(
                &release.archive,
                &stage.path().join("update.tar.gz"),
                MAX_ARCHIVE,
                generation,
            )?;
            fetch(
                &release.signature,
                &stage.path().join("update.sig"),
                64,
                generation,
            )?;
            verify(
                &stage.path().join("update.tar.gz"),
                &stage.path().join("update.sig"),
                PUBLIC_KEY,
            )?;
            // Validate the full bundle before offering Restart, but extract afresh in the installer.
            let check = stage.path().join("check");
            extract(&stage.path().join("update.tar.gz"), &check)?;
            validate_bundle(&check.join("Browsify.app"), &release.version)?;
            fs::remove_dir_all(check).map_err(|e| e.to_string())?;
            Ok::<_, String>(stage)
        })();
        let mut s = state().lock().unwrap();
        if s.generation != generation {
            return;
        }
        s.busy = false;
        match result {
            Ok(stage) => {
                // Persist readiness atomically, so Later survives a normal app quit.
                s.staged = None;
                let record = stage_record(&home);
                let saved = SavedUpdate {
                    directory: stage.path().to_owned(),
                    release: release.clone(),
                    manual,
                };
                let saved_result = (|| {
                    let mut file = tempfile::NamedTempFile::new_in(stage_root(&home))
                        .map_err(|e| e.to_string())?;
                    serde_json::to_writer(&mut file, &saved).map_err(|e| e.to_string())?;
                    file.persist(&record).map_err(|e| e.to_string())?;
                    Ok::<_, String>(())
                })();
                if let Err(error) = saved_result {
                    publish(&mut s, "error", Some(error));
                    return;
                }
                s.staged = Some(StagedUpdate {
                    manual,
                    directory: stage.keep(),
                    record,
                    keep_files: false,
                });
                publish(&mut s, "ready", None);
                drop(s);
                prompt_ready(&home, generation, &release);
            }
            Err(error) => {
                publish(&mut s, "error", Some(error.clone()));
                drop(s);
                log_event(LogLevel::Warn, format!("Update download failed: {error}"));
                if manual {
                    notice("Unable to download update", &error);
                }
            }
        }
    });
}

fn prompt_ready(home: &Path, generation: u64, release: &Release) {
    if !active(generation) {
        return;
    }
    let answer = dialog_with_generation(
        &format!("Browsify {} is ready", release.version),
        "The signed update has been downloaded and verified. Restart Browsify to install it. Your rules and preferences will be preserved. Release notes are available in Preferences.",
        &["Skip This Version", "Later", "Restart to Update"], Some(generation),
    );
    if !active(generation) {
        return;
    }
    match answer.as_deref() {
        Some("Restart to Update") => {
            if let Err(error) = restart(home) {
                notice("Unable to install update", &error);
            }
        }
        Some("Skip This Version") => {
            let mut prefs = preferences(home);
            prefs.skipped = Some(release.version.clone());
            if let Err(error) = save_preferences(home, &prefs) {
                notice("Unable to save skipped release", &error);
                return;
            }
            let mut s = state().lock().unwrap();
            if s.generation == generation {
                s.staged = None;
                publish(&mut s, "skipped", None);
            }
        }
        _ => {
            let mut prefs = preferences(home);
            prefs.remind_after = now() + DAY;
            prefs.deferred_version = Some(release.version.clone());
            if let Err(error) = save_preferences(home, &prefs) {
                notice("Unable to save update reminder", &error);
            }
        }
    }
}

fn installed_bundle() -> Result<PathBuf, String> {
    if !crate::macos::is_update_host_ready() {
        return Err(
            "Open Browsify.app to download and install updates, or use the manual download link."
                .into(),
        );
    }
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let app = exe
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("Not running from an installed app bundle")?;
    if app.extension().and_then(|s| s.to_str()) != Some("app")
        || exe.strip_prefix(app).ok() != Some(Path::new("Contents/MacOS/browsify"))
        || app.starts_with("/Volumes")
        || app.to_string_lossy().contains("/AppTranslocation/")
    {
        return Err("Move Browsify.app to a writable Applications folder before updating, or install manually from GitHub Releases.".into());
    }
    // A private sibling directory tests actual write permission on the destination filesystem.
    let probe = tempfile::Builder::new().prefix(".browsify-write-test-").tempdir_in(app.parent().unwrap()).map_err(|_| "Browsify's installation folder is not writable. Download and install the update manually.".to_string())?;
    drop(probe);
    Ok(app.to_owned())
}

fn extract(archive: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    let decoder = flate2::read::GzDecoder::new(fs::File::open(archive).map_err(|e| e.to_string())?);
    let mut archive = tar::Archive::new(decoder);
    let mut total = 0u64;
    let mut count = 0;
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let kind = entry.header().entry_type();
        if !path.starts_with("Browsify.app")
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || !(kind.is_file() || kind.is_dir())
        {
            return Err("Unsafe update archive entry".into());
        }
        count += 1;
        total = total
            .checked_add(entry.size())
            .ok_or("Update archive overflow")?;
        if count > 20_000 || total > 512 * 1024 * 1024 {
            return Err("Expanded update is too large".into());
        }
        if !entry.unpack_in(destination).map_err(|e| e.to_string())? {
            return Err("Unsafe update archive path".into());
        }
        // Never preserve setuid/setgid bits from an archive.
        #[cfg(unix)]
        if kind.is_file() {
            use std::os::unix::fs::PermissionsExt;
            let target = destination.join(path);
            let mode = fs::metadata(&target)
                .map_err(|e| e.to_string())?
                .permissions()
                .mode()
                & 0o777;
            fs::set_permissions(target, fs::Permissions::from_mode(mode))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn validate_bundle(app: &Path, version: &str) -> Result<(), String> {
    let plist = app.join("Contents/Info.plist");
    for (key, expected) in [
        ("CFBundleIdentifier", "com.lavenxsnow.browsify"),
        (
            "CFBundleShortVersionString",
            version.trim_start_matches('v'),
        ),
    ] {
        let output = Command::new("/usr/libexec/PlistBuddy")
            .args(["-c", &format!("Print :{key}")])
            .arg(&plist)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim() != expected {
            return Err("Update bundle identity or version does not match the release".into());
        }
    }
    let exe = app.join("Contents/MacOS/browsify");
    if !exe.is_file() || !app.join("Contents/Resources/ui/index.html").is_file() {
        return Err("Incomplete update bundle".into());
    }
    Ok(())
}

/// Invoked only after explicit restart approval. The installer is the currently trusted binary.
pub fn restart(home: &Path) -> Result<(), String> {
    // Always acquire routing before updater state, including the main-thread exit callback.
    let _routing = ROUTING.lock().unwrap();
    let mut s = state().lock().unwrap();
    if s.restarting || s.busy {
        return Err("An update operation is already in progress".into());
    }
    let stage = s.staged.as_ref().ok_or("No verified update is ready")?;
    let app = installed_bundle()?;
    let release = s.release.as_ref().ok_or("No update selected")?;
    fs::write(
        stage.path().join("installer.json"),
        serde_json::to_vec(&InstallRequest {
            app,
            stage: stage.path().to_owned(),
            version: release.version.clone(),
            pid: std::process::id(),
            home: home.to_owned(),
            args: std::env::args().skip(1).collect(),
        })
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // Run outside the .app so LaunchServices cannot mistake the installer for the URL router.
    let helper = stage.path().join("installer");
    fs::copy(std::env::current_exe().map_err(|e| e.to_string())?, &helper)
        .map_err(|e| e.to_string())?;
    Command::new(helper)
        .arg("--internal-install-update")
        .arg(stage.path().join("installer.json"))
        .spawn()
        .map_err(|e| e.to_string())?;
    let handoff = stage.path().to_owned();
    s.restarting = true;
    s.handoff = Some(handoff);
    RESTARTING.store(true, Ordering::SeqCst);
    // Keep routing events durable while the helper prepares the replacement.
    let stage = s.staged.take().unwrap().keep();
    thread::spawn(move || {
        let mut exit_requested = false;
        for _ in 0..600 {
            if !exit_requested && stage.join("exit-ready").is_file() {
                crate::macos::request_update_exit();
                exit_requested = true;
            }
            if stage.join("failed").is_file() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        RESTARTING.store(false, Ordering::SeqCst);
        let mut s = state().lock().unwrap();
        s.restarting = false;
        s.handoff = None;
        publish(
            &mut s,
            "error",
            Some("Installation could not be prepared; the current app was not replaced.".into()),
        );
        drop(s);
        crate::macos::resume_update_links();
    });
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct InstallRequest {
    app: PathBuf,
    stage: PathBuf,
    version: String,
    pid: u32,
    home: PathBuf,
    args: Vec<String>,
}

/// Returns true for the internal helper invocation, before normal config/daemon startup.
pub fn run_installer(args: &[String]) -> bool {
    if args.first().map(String::as_str) != Some("--internal-install-update") {
        return false;
    }
    let result = args
        .get(1)
        .ok_or("Missing installer request".to_owned())
        .and_then(|p| {
            let request: InstallRequest =
                serde_json::from_slice(&fs::read(p).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let result = install(&request);
            if result.is_err() {
                let _ = fs::write(request.stage.join("failed"), b"failed");
                #[cfg(unix)]
                if unsafe { libc::kill(request.pid as i32, 0) } != 0
                    && request.app.join("Contents/MacOS/browsify").is_file()
                {
                    let _ = Command::new(request.app.join("Contents/MacOS/browsify"))
                        .args(&request.args)
                        .env("BROWSIFY_UPDATE_RESTART", "1")
                        .spawn();
                }
            }
            result
        });
    if let Err(error) = result {
        notice("Browsify update failed", &error);
    }
    true
}

fn install(r: &InstallRequest) -> Result<(), String> {
    // Recheck immutable trust and signed version rather than trusting a ready flag or extracted files.
    verify(
        &r.stage.join("update.tar.gz"),
        &r.stage.join("update.sig"),
        PUBLIC_KEY,
    )?;
    if !newer(&r.version, env!("CARGO_PKG_VERSION")) {
        return Err("Refusing to install an older release".into());
    }
    let parent = r.app.parent().ok_or("Invalid installation path")?;
    let work = tempfile::Builder::new()
        .prefix(".browsify-install-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    extract(&r.stage.join("update.tar.gz"), work.path())?;
    let replacement = work.path().join("Browsify.app");
    validate_bundle(&replacement, &r.version)?;
    fs::write(r.stage.join("exit-ready"), b"ready").map_err(|e| e.to_string())?;
    // Wait for the old daemon and its singleton lock to go away; never kill another process.
    #[cfg(unix)]
    {
        let mut exited = false;
        for _ in 0..100 {
            if unsafe { libc::kill(r.pid as i32, 0) } != 0 {
                exited = true;
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        if !exited {
            return Err("The running app did not exit; update cancelled".into());
        }
    }
    // From this point, retain recovery files even if a filesystem operation fails.
    let work = work.keep();
    let backup = work.join("previous.app");
    fs::rename(&r.app, &backup).map_err(|e| format!("Could not preserve the current app: {e}"))?;
    if let Err(error) = fs::rename(&replacement, &r.app) {
        fs::rename(&backup, &r.app).map_err(|restore| format!("Replacement failed ({error}); restore failed ({restore}). Previous app retained at {}", backup.display()))?;
        return Err(format!("Could not replace Browsify: {error}"));
    }
    let ack = r.stage.join("started");
    let launch = Command::new(r.app.join("Contents/MacOS/browsify"))
        .args(&r.args)
        .env("BROWSIFY_UPDATE_ACK", &ack)
        .env("BROWSIFY_UPDATE_RESTART", "1")
        .spawn();
    if let Ok(mut child) = launch {
        for _ in 0..200 {
            if ack.is_file() {
                let _ = fs::remove_dir_all(&r.stage);
                let _ = fs::remove_dir_all(&work);
                return Ok(());
            }
            if !matches!(child.try_wait(), Ok(None)) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
    let failed = work.join("failed.app");
    fs::rename(&r.app, &failed).map_err(|e| e.to_string())?;
    fs::rename(&backup, &r.app).map_err(|e| {
        format!(
            "Restore failed; previous app is at {}: {e}",
            backup.display()
        )
    })?;
    let _ = fs::remove_dir_all(&r.stage);
    let _ = fs::remove_dir_all(&work);
    Err("The new app did not start. The previous version has been restored.".into())
}

// Routing is serialized with restart preparation so no active launch is interrupted.
pub static ROUTING: Mutex<()> = Mutex::new(());

pub fn can_exit_for_update() -> bool {
    let s = state().lock().unwrap();
    s.restarting
        && s.handoff
            .as_ref()
            .map(|p| p.join("exit-ready").is_file() && !p.join("failed").is_file())
            .unwrap_or(false)
}
static RESTARTING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Deserialize)]
pub struct PendingLink {
    pub url: String,
    pub opener: Option<browsify_core::OpenerInfo>,
    pub background: bool,
}
fn pending_path(home: &Path) -> PathBuf {
    home.join("Library/Application Support/Browsify/update-pending-links.json")
}

/// Caller holds ROUTING. Persist before acknowledging receipt; on failure route normally.
pub fn queue_during_restart(
    home: &Path,
    url: &str,
    opener: &Option<browsify_core::OpenerInfo>,
    background: bool,
) -> bool {
    if !RESTARTING.load(Ordering::SeqCst) {
        return false;
    }
    let result = (|| {
        let path = pending_path(home);
        let mut links: Vec<PendingLink> = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
        } else {
            Vec::new()
        };
        links.push(PendingLink {
            url: url.to_owned(),
            opener: opener.clone(),
            background,
        });
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let mut file =
            tempfile::NamedTempFile::new_in(path.parent().unwrap()).map_err(|e| e.to_string())?;
        serde_json::to_writer(&mut file, &links).map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(path).map_err(|e| e.to_string())?;
        Ok::<_, String>(())
    })();
    if let Err(error) = result {
        log_event(
            LogLevel::Error,
            format!("Could not queue update-time link; routing normally: {error}"),
        );
        return false;
    }
    true
}

pub fn resume_links(
    engine: &Arc<Mutex<BrowsifyEngine>>,
    resolver: &Arc<browsify_core::ProfileResolver>,
    dry_run: bool,
) {
    let path = pending_path(resolver.home_dir());
    let links: Vec<PendingLink> = match fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
    {
        Some(links) => links,
        None => return,
    };
    for link in links {
        crate::macos::handle_daemon_url_event(
            engine,
            resolver,
            &link.url,
            link.opener,
            link.background,
            dry_run,
        );
    }
    let _ = fs::remove_file(path);
}

pub fn acknowledge_startup() {
    if let Some(path) = std::env::var_os("BROWSIFY_UPDATE_ACK") {
        let _ = fs::write(path, b"ready");
        std::env::remove_var("BROWSIFY_UPDATE_ACK");
    }
}

fn open_notes(release: &Release) {
    #[cfg(target_os = "macos")]
    let _ = Command::new("/usr/bin/open").arg(&release.url).spawn();
    #[cfg(not(target_os = "macos"))]
    let _ = release;
}
fn notice(title: &str, message: &str) {
    let _ = dialog(title, message, &["OK"]);
}

fn dialog(title: &str, message: &str, buttons: &[&str]) -> Option<String> {
    dialog_with_generation(title, message, buttons, None)
}

fn dialog_with_generation(
    title: &str,
    message: &str,
    buttons: &[&str],
    generation: Option<u64>,
) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        // Only fixed button labels enter AppleScript source. Remote text is passed as argv.
        let list = buttons
            .iter()
            .map(|b| format!("\"{b}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let script = format!("on run argv\nset answer to display dialog (item 2 of argv) with title (item 1 of argv) buttons {{{list}}} default button {}\nreturn button returned of answer\nend run", buttons.len());
        let mut child = Command::new("/usr/bin/osascript")
            .args(["-e", &script, "--", title, message])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        loop {
            if generation.map(|g| !active(g)).unwrap_or(false) {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            if child.try_wait().ok()?.is_some() {
                let output = child.wait_with_output().ok()?;
                return output
                    .status
                    .success()
                    .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned());
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (title, message, buttons, generation);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn disabling_updates_discards_automatic_packages_but_preserves_manual_ones() {
        for (manual, package_manual) in [(false, false), (false, true), (true, false), (true, true)]
        {
            let root = tempfile::tempdir().unwrap();
            let directory = root.path().join("package");
            let record = root.path().join("ready.json");
            fs::create_dir(&directory).unwrap();
            fs::write(directory.join("archive"), b"downloaded package").unwrap();
            fs::write(&record, b"ready").unwrap();
            let mut s = State {
                enabled: true,
                busy: true,
                manual,
                generation: 7,
                staged: Some(StagedUpdate {
                    manual: package_manual,
                    directory: directory.clone(),
                    record: record.clone(),
                    keep_files: false,
                }),
                ..State::default()
            };
            s.configure(false);
            assert!(!s.enabled);
            assert_eq!(directory.exists(), package_manual);
            assert_eq!(record.exists(), package_manual);
            assert_eq!(s.busy, manual);
            assert_eq!(s.generation, if manual { 7 } else { 8 });
        }
    }

    #[test]
    fn release_discovery_requires_stable_versions_and_expected_signed_assets() {
        let make = |origin: &str| {
            serde_json::json!({
                "tag_name": "v999.0.0", "draft": false, "prerelease": false,
                "assets": [
                    {"name": "Browsify-universal.tar.gz", "browser_download_url": format!("{origin}/Browsify-universal.tar.gz")},
                    {"name": "Browsify-universal.tar.gz.sig", "browser_download_url": format!("{origin}/Browsify-universal.tar.gz.sig")}
                ]
            })
        };
        let valid = make("https://github.com/dungsnowaxe/browsify/releases/download/v999.0.0");
        let release = parse_release(&serde_json::to_vec(&valid).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(release.version, "v999.0.0");
        assert!(parse_release(
            &serde_json::to_vec(&make("https://untrusted.example/update")).unwrap()
        )
        .is_err());
        let mut missing_signature = valid.clone();
        missing_signature["assets"].as_array_mut().unwrap().pop();
        assert!(parse_release(&serde_json::to_vec(&missing_signature).unwrap()).is_err());
        for flag in ["draft", "prerelease"] {
            let mut hidden = valid.clone();
            hidden[flag] = true.into();
            assert!(parse_release(&serde_json::to_vec(&hidden).unwrap())
                .unwrap()
                .is_none());
        }
    }

    #[test]
    fn restart_queue_persists_link_context_and_routes_normally_outside_restart() {
        // Match the production caller contract and isolate concurrent routing tests.
        let _routing = ROUTING.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let opener = Some(browsify_core::OpenerInfo {
            name: "Test opener".into(),
            bundle_id: "test.opener".into(),
            ..Default::default()
        });
        RESTARTING.store(true, Ordering::SeqCst);
        assert!(queue_during_restart(
            home.path(),
            "https://example.test/queued",
            &opener,
            true
        ));
        RESTARTING.store(false, Ordering::SeqCst);
        let saved: Vec<PendingLink> =
            serde_json::from_slice(&fs::read(pending_path(home.path())).unwrap()).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].url, "https://example.test/queued");
        assert_eq!(saved[0].opener, opener);
        assert!(saved[0].background);
        assert!(!queue_during_restart(
            home.path(),
            "https://example.test/live",
            &None,
            false
        ));
        let saved: Vec<PendingLink> =
            serde_json::from_slice(&fs::read(pending_path(home.path())).unwrap()).unwrap();
        assert_eq!(saved.len(), 1);
    }

    #[test]
    fn manual_checks_bypass_the_daily_interval_and_failures_are_not_up_to_date() {
        let home = tempfile::tempdir().unwrap();
        save_preferences(
            home.path(),
            &Preferences {
                last_check: now(),
                ..Preferences::default()
            },
        )
        .unwrap();
        for enabled in [false, true] {
            let mut s = State {
                enabled,
                ..State::default()
            };
            assert!(s.begin_check(home.path(), false).is_none());
            assert!(s.begin_check(home.path(), true).is_some());
            let info = s.info.as_ref().unwrap();
            assert_eq!(info.status, "checking");
            assert_eq!(info.update_check_enabled, enabled);
            publish(
                &mut s,
                "error",
                Some("Check your connection and try again.".into()),
            );
            let info = s.info.as_ref().unwrap();
            assert_eq!(info.status, "error");
            assert!(!info.has_update);
            assert_eq!(
                info.error.as_deref(),
                Some("Check your connection and try again.")
            );
        }
    }

    #[test]
    fn stable_versions_only_and_never_downgrade() {
        for (candidate, current, expected) in [
            ("v0.2.0", "0.1.0", true),
            ("v0.10.0", "0.9.0", true),
            ("v0.1.0", "0.1.0", false),
            ("v0.0.9", "0.1.0", false),
            ("0.1.0", "0.1.0-beta.1", true),
            ("v0.2.0-beta.1", "0.1.0", false),
            ("garbage", "0.1.0", false),
        ] {
            assert_eq!(
                newer(candidate, current),
                expected,
                "{candidate} vs {current}"
            );
        }
    }

    #[test]
    fn rejects_tampered_archives_and_wrong_signers() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("archive");
        let signature = temp.path().join("signature");
        let key = SigningKey::from_bytes(&[42; 32]);
        fs::write(&archive, b"signed release").unwrap();
        fs::write(&signature, key.sign(b"signed release").to_bytes()).unwrap();
        let public = STANDARD.encode(key.verifying_key().as_bytes());
        assert!(verify(&archive, &signature, &public).is_ok());
        fs::write(&archive, b"modified release").unwrap();
        assert!(verify(&archive, &signature, &public).is_err());
        fs::write(&archive, b"signed release").unwrap();
        let wrong = SigningKey::from_bytes(&[43; 32]);
        assert!(verify(
            &archive,
            &signature,
            &STANDARD.encode(wrong.verifying_key().as_bytes())
        )
        .is_err());
        assert!(verify(&archive, &signature, "").is_err());
    }

    #[test]
    fn archive_rejects_links_and_foreign_roots_but_extracts_valid_files() {
        for (path, link, valid) in [
            ("Browsify.app/Contents/file", false, true),
            ("elsewhere/file", false, false),
            ("Browsify.app/link", true, false),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let archive = temp.path().join("update.tar.gz");
            let gzip = flate2::write::GzEncoder::new(
                fs::File::create(&archive).unwrap(),
                flate2::Compression::default(),
            );
            let mut tar = tar::Builder::new(gzip);
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            if link {
                header.set_entry_type(tar::EntryType::Symlink);
                header.set_link_name("/tmp/escape").unwrap();
                header.set_size(0);
            } else {
                header.set_size(7);
            }
            header.set_cksum();
            tar.append_data(
                &mut header,
                path,
                if link { &b""[..] } else { &b"payload"[..] },
            )
            .unwrap();
            tar.into_inner().unwrap().finish().unwrap();
            let destination = temp.path().join("out");
            assert_eq!(extract(&archive, &destination).is_ok(), valid, "{path}");
            if valid {
                assert_eq!(fs::read(destination.join(path)).unwrap(), b"payload");
            }
        }
    }
}
