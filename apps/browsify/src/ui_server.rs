use crate::config_loader::BrowsifyEngine;
use browsify_core::{ProfileResolver, RulesFile};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_LOG_BUFFER: usize = 200;
const MAX_RECENT_URLS: usize = 10;
const IDLE_SHUTDOWN_GRACE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: u128,
    pub level: String,
    pub msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRoutedUrl {
    pub input_url: String,
    pub final_url: String,
    pub browser: String,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub has_update: bool,
    pub update_check_enabled: bool,
    pub download_url: String,
    pub release_url: String,
}

struct GlobalTelemetryState {
    logs: VecDeque<LogEntry>,
    recent_urls: VecDeque<RecentRoutedUrl>,
    sse_clients: Vec<TcpStream>,
    update_info: Option<UpdateInfo>,
    active_ui_url: Option<String>,
}

static TELEMETRY: OnceLock<Mutex<GlobalTelemetryState>> = OnceLock::new();

fn telemetry() -> &'static Mutex<GlobalTelemetryState> {
    TELEMETRY.get_or_init(|| {
        Mutex::new(GlobalTelemetryState {
            logs: VecDeque::with_capacity(MAX_LOG_BUFFER),
            recent_urls: VecDeque::with_capacity(MAX_RECENT_URLS),
            sse_clients: Vec::new(),
            update_info: None,
            active_ui_url: None,
        })
    })
}

pub fn log_event(level: &str, msg: impl Into<String>) {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let entry = LogEntry {
        time: now_ms,
        level: level.to_string(),
        msg: msg.into(),
    };
    let sse_payload = serde_json::json!({
        "type": "log",
        "message": serde_json::to_string(&entry).unwrap_or_default()
    })
    .to_string();

    if let Ok(mut guard) = telemetry().lock() {
        if guard.logs.len() >= MAX_LOG_BUFFER {
            guard.logs.pop_front();
        }
        guard.logs.push_back(entry);
        broadcast_sse_locked(&mut guard, &sse_payload);
    }
}

#[allow(dead_code)]
pub fn record_routed_url(input_url: &str, final_url: &str, browser: &str, profile: &str) {
    if let Ok(mut guard) = telemetry().lock() {
        guard.recent_urls.retain(|r| r.input_url != input_url);
        guard.recent_urls.push_front(RecentRoutedUrl {
            input_url: input_url.to_string(),
            final_url: final_url.to_string(),
            browser: browser.to_string(),
            profile: profile.to_string(),
        });
        while guard.recent_urls.len() > MAX_RECENT_URLS {
            guard.recent_urls.pop_back();
        }
    }
}

#[allow(dead_code)]
pub fn get_recent_routed_urls() -> Vec<RecentRoutedUrl> {
    telemetry()
        .lock()
        .map(|g| g.recent_urls.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn set_update_info(info: UpdateInfo) {
    let payload = serde_json::json!({
        "type": "updateInfo",
        "message": &info
    })
    .to_string();
    if let Ok(mut guard) = telemetry().lock() {
        guard.update_info = Some(info);
        broadcast_sse_locked(&mut guard, &payload);
    }
}

pub fn is_ui_server_active() -> bool {
    telemetry()
        .lock()
        .map(|g| g.active_ui_url.is_some())
        .unwrap_or(false)
}

pub fn notify_config_reloaded(engine: &BrowsifyEngine) {
    let msg = build_config_message(engine);
    let payload = serde_json::json!({
        "type": "config",
        "message": msg
    })
    .to_string();
    if let Ok(mut guard) = telemetry().lock() {
        broadcast_sse_locked(&mut guard, &payload);
    }
}

fn broadcast_sse_locked(guard: &mut GlobalTelemetryState, json_payload: &str) {
    let frame = format!("data: {}\n\n", json_payload);
    guard
        .sse_clients
        .retain_mut(|stream| stream.write_all(frame.as_bytes()).is_ok() && stream.flush().is_ok());
}

/// Spawns the on-demand UI server in a background thread if not already active, or opens the existing URL.
#[allow(dead_code)]
pub fn spawn_or_open_ui_server(
    engine: Arc<Mutex<BrowsifyEngine>>,
    profile_resolver: Arc<ProfileResolver>,
) {
    if let Ok(guard) = telemetry().lock() {
        if let Some(existing_url) = &guard.active_ui_url {
            open_url_in_system_browser(existing_url);
            return;
        }
    }
    thread::spawn(move || {
        let _ = run_ui_server(engine, profile_resolver, 0, true);
    });
}

/// Serves the token-authenticated Browsify UI over `127.0.0.1:<port>` with SSE log streaming
/// and automatic 30-second idle shutdown once all browser tabs disconnect (0 MB idle RAM).
pub fn run_ui_server(
    engine: Arc<Mutex<BrowsifyEngine>>,
    profile_resolver: Arc<ProfileResolver>,
    port: u16,
    open_browser: bool,
) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("Failed to bind UI server on 127.0.0.1:{}: {}", port, e))?;
    let local_addr = listener.local_addr().map_err(|e| e.to_string())?;
    let session_token = generate_session_token();
    let url = format!("http://{}/?token={}", local_addr, session_token);

    if let Ok(mut guard) = telemetry().lock() {
        guard.active_ui_url = Some(url.clone());
    }

    println!("Browsify UI listening on {}", url);
    log_event("info", format!("On-demand UI server started on {}", local_addr));

    if open_browser {
        open_url_in_system_browser(&url);
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let had_sse_client = Arc::new(AtomicBool::new(false));
    let active_sse_count = Arc::new(AtomicUsize::new(0));
    let last_activity = Arc::new(Mutex::new(Instant::now()));

    // Background idle reaper & SSE heartbeat thread:
    // Shuts down the server 30s after the last UI tab closes.
    {
        let stop_flag = Arc::clone(&stop_flag);
        let had_sse_client = Arc::clone(&had_sse_client);
        let active_sse_count = Arc::clone(&active_sse_count);
        let last_activity = Arc::clone(&last_activity);
        thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(3));
                let mut alive_count = 0usize;
                if let Ok(mut guard) = telemetry().lock() {
                    guard.sse_clients.retain_mut(|stream| {
                        stream.write_all(b": keepalive\n\n").is_ok() && stream.flush().is_ok()
                    });
                    alive_count = guard.sse_clients.len();
                }
                active_sse_count.store(alive_count, Ordering::Relaxed);
                if alive_count > 0 {
                    if let Ok(mut t) = last_activity.lock() {
                        *t = Instant::now();
                    }
                    continue;
                }
                if had_sse_client.load(Ordering::Relaxed) {
                    let idle_elapsed = last_activity
                        .lock()
                        .map(|t| t.elapsed())
                        .unwrap_or(Duration::ZERO);
                    if idle_elapsed >= IDLE_SHUTDOWN_GRACE {
                        stop_flag.store(true, Ordering::Relaxed);
                        let _ = TcpStream::connect(local_addr);
                        break;
                    }
                }
            }
        });
    }

    let dist_dir = resolve_ui_dist_dir();

    for stream in listener.incoming() {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
        let Ok(mut stream) = stream else {
            continue;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buf = [0u8; 65536];
        let Ok(n) = stream.read(&mut buf) else {
            continue;
        };
        if n == 0 {
            continue;
        }

        if let Ok(mut t) = last_activity.lock() {
            *t = Instant::now();
        }

        let req = String::from_utf8_lossy(&buf[..n]);
        let mut lines = req.lines();
        let Some(first_line) = lines.next() else {
            continue;
        };
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let method = parts[0];
        let raw_target = parts[1];
        let (path, query) = raw_target.split_once('?').unwrap_or((raw_target, ""));

        if path.starts_with("/api/") {
            if !validate_loopback_and_token(&req, query, &session_token) {
                let _ = write_http_response(
                    &mut stream,
                    "403 Forbidden",
                    "application/json; charset=utf-8",
                    br#"{"error":"Unauthorized loopback request"}"#,
                );
                continue;
            }
        }

        if method == "GET" && path == "/api/events" {
            let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n: connected\n\n";
            if stream.write_all(headers.as_bytes()).is_ok() && stream.flush().is_ok() {
                let _ = stream.set_read_timeout(None);
                had_sse_client.store(true, Ordering::Relaxed);
                if let Ok(mut guard) = telemetry().lock() {
                    guard.sse_clients.push(stream);
                    active_sse_count.store(guard.sse_clients.len(), Ordering::Relaxed);
                }
            }
            continue;
        }

        if method == "POST" && path == "/api/rpc" {
            let body = req
                .split("\r\n\r\n")
                .nth(1)
                .or_else(|| req.split("\n\n").nth(1))
                .unwrap_or("");
            let response_json = handle_rpc(body, &engine, &profile_resolver);
            let _ = write_http_response(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                response_json.as_bytes(),
            );
            continue;
        }

        let clean_path = path.trim_start_matches('/');
        if let Some(ref dir) = dist_dir {
            let is_spa_route = clean_path.is_empty() || !clean_path.contains('.');
            let target = if is_spa_route {
                dir.join("index.html")
            } else {
                dir.join(clean_path)
            };
            if let Ok(bytes) = fs::read(&target) {
                if is_spa_route {
                    let html = String::from_utf8_lossy(&bytes);
                    let injected = inject_session_token(&html, &session_token);
                    let _ = write_http_response(
                        &mut stream,
                        "200 OK",
                        "text/html; charset=utf-8",
                        injected.as_bytes(),
                    );
                } else {
                    let mime = guess_mime(&target);
                    let _ = write_http_response(&mut stream, "200 OK", mime, &bytes);
                }
                continue;
            }
        }

        let fallback_html = inject_session_token(FALLBACK_SPA_HTML, &session_token);
        let _ = write_http_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            fallback_html.as_bytes(),
        );
    }

    if let Ok(mut guard) = telemetry().lock() {
        guard.active_ui_url = None;
        guard.sse_clients.clear();
    }
    stop_flag.store(true, Ordering::Relaxed);
    Ok(())
}

fn validate_loopback_and_token(raw_req: &str, query: &str, expected_token: &str) -> bool {
    let mut header_token: Option<&str> = None;
    let mut host_ok = false;
    let mut origin_ok = true;

    for line in raw_req.lines().skip(1) {
        if line.is_empty() || line == "\r" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            let val = v.trim();
            if key.eq_ignore_ascii_case("x-browsify-token") {
                header_token = Some(val);
            } else if key.eq_ignore_ascii_case("host") {
                host_ok = val.starts_with("127.0.0.1:")
                    || val.starts_with("localhost:")
                    || val == "127.0.0.1"
                    || val == "localhost";
            } else if key.eq_ignore_ascii_case("origin") {
                origin_ok = val.starts_with("http://127.0.0.1:")
                    || val.starts_with("http://localhost:")
                    || val == "http://127.0.0.1"
                    || val == "http://localhost";
            }
        }
    }

    let query_token = query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == "token" {
            Some(v)
        } else {
            None
        }
    });

    let token_matches = header_token == Some(expected_token) || query_token == Some(expected_token);
    host_ok && origin_ok && token_matches
}

fn generate_session_token() -> String {
    let mut bytes = [0u8; 16];
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut bytes);
    } else {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0x9e3779b97f4a7c15);
        bytes = nanos.to_le_bytes();
    }
    let mut hex = String::with_capacity(32);
    for b in bytes {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

fn inject_session_token(html: &str, token: &str) -> String {
    let script = format!(
        "<script>window.__BROWSIFY_TOKEN__ = {:?};</script>",
        token
    );
    if let Some(idx) = html.find("</head>") {
        let mut out = String::with_capacity(html.len() + script.len());
        out.push_str(&html[..idx]);
        out.push_str(&script);
        out.push_str(&html[idx..]);
        out
    } else {
        format!("{}{}", script, html)
    }
}

fn resolve_ui_dist_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // 1. Inside `Browsify.app/Contents/MacOS/browsify` -> `../Resources/ui`
            let bundle_ui = exe_dir.join("../Resources/ui");
            if bundle_ui.join("index.html").is_file() {
                return Some(bundle_ui);
            }
            // 2. Inside `target/release/browsify` or `target/debug/browsify` -> `../../apps/browsify-ui/dist`
            let target_rel = exe_dir.join("../../apps/browsify-ui/dist");
            if target_rel.join("index.html").is_file() {
                return Some(target_rel);
            }
        }
    }
    let cwd_dist = PathBuf::from("apps/browsify-ui/dist");
    if cwd_dist.join("index.html").is_file() {
        return Some(cwd_dist);
    }
    None
}

fn open_url_in_system_browser(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

fn build_config_message(eng: &BrowsifyEngine) -> serde_json::Value {
    let sum = eng.summary();
    let cfg_path = sum
        .config_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    serde_json::json!({
        "handlers": sum.handlers_count,
        "rewrites": sum.rewrites_count,
        "defaultBrowser": sum.default_browser,
        "configPath": cfg_path,
        "isJSConfig": sum.has_primary_config,
        "hasPrimaryConfig": sum.has_primary_config,
        "explicitDefaultBrowser": sum.explicit_default_browser,
        "pureNativeFastPath": sum.pure_native_fast_path,
        "format": sum.format,
        "isDefaultBrowser": crate::macos::is_default_browser(),
        "options": {
            "keepRunning": sum.options.keep_running,
            "hideIcon": sum.options.hide_icon,
            "logRequests": sum.options.log_requests,
            "checkForUpdates": sum.options.check_for_updates
        }
    })
}

fn handle_rpc(
    body: &str,
    engine: &Arc<Mutex<BrowsifyEngine>>,
    profile_resolver: &Arc<ProfileResolver>,
) -> String {
    let Ok(msg) = serde_json::from_str::<serde_json::Value>(body) else {
        return "[]".to_string();
    };
    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match msg_type {
        "getInitState" => {
            let eng = engine.lock().unwrap();
            let sum = eng.summary();
            let mut events = vec![
                serde_json::json!({
                    "type": "version",
                    "message": format!("v{}", env!("CARGO_PKG_VERSION"))
                }),
                serde_json::json!({
                    "type": "buildInfo",
                    "message": format!("({})", sum.format)
                }),
                serde_json::json!({
                    "type": "config",
                    "message": build_config_message(&eng)
                }),
            ];
            if let Ok(guard) = telemetry().lock() {
                if let Some(ref uinfo) = guard.update_info {
                    events.push(serde_json::json!({
                        "type": "updateInfo",
                        "message": uinfo
                    }));
                }
                for entry in &guard.logs {
                    if let Ok(s) = serde_json::to_string(entry) {
                        events.push(serde_json::json!({
                            "type": "log",
                            "message": s
                        }));
                    }
                }
            }
            serde_json::Value::Array(events).to_string()
        }
        "getRules" => {
            let rules_path = RulesFile::default_path(profile_resolver.home_dir());
            let rf = RulesFile::load_from_path(&rules_path).unwrap_or_default();
            let path_str = rules_path.exists().then(|| rules_path.display().to_string());
            serde_json::json!({
                "type": "rules",
                "message": {
                    "defaultBrowser": rf.default_browser,
                    "defaultProfile": rf.default_profile,
                    "options": rf.options,
                    "rules": rf.rules,
                    "path": path_str
                }
            })
            .to_string()
        }
        "saveRules" => {
            let Some(payload) = msg.get("payload") else {
                return serde_json::json!({
                    "type": "saveRulesError",
                    "message": { "error": "Missing payload" }
                })
                .to_string();
            };
            match serde_json::from_value::<RulesFile>(payload.clone()) {
                Ok(rf) => {
                    let rules_path = RulesFile::default_path(profile_resolver.home_dir());
                    if let Err(e) = rf.save_to_path(&rules_path) {
                        return serde_json::json!({
                            "type": "saveRulesError",
                            "message": { "error": e }
                        })
                        .to_string();
                    }
                    let mut eng = engine.lock().unwrap();
                    eng.update_rules(&rf);
                    let resp = serde_json::json!([
                        {
                            "type": "rules",
                            "message": {
                                "defaultBrowser": rf.default_browser,
                                "defaultProfile": rf.default_profile,
                                "options": rf.options,
                                "rules": rf.rules,
                                "path": rules_path.display().to_string()
                            }
                        },
                        {
                            "type": "config",
                            "message": build_config_message(&eng)
                        }
                    ])
                    .to_string();
                    drop(eng);
                    log_event(
                        "info",
                        format!("Saved {} rules to {}", rf.rules.len(), rules_path.display()),
                    );
                    crate::macos::trigger_update_check_if_enabled(
                        engine,
                        profile_resolver.home_dir(),
                    );
                    resp
                }
                Err(e) => serde_json::json!({
                    "type": "saveRulesError",
                    "message": { "error": e.to_string() }
                })
                .to_string(),
            }
        }
        "testUrl" => {
            let url = msg.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let eng = engine.lock().unwrap();
            match eng.resolve(url, None, false) {
                Ok(cfg) => {
                    let plan = profile_resolver.build_launch_plan(
                        &cfg,
                        false,
                        eng.summary().options.on_missing_profile,
                    );
                    let display_profile = if let Some(c) = &cfg.container {
                        if cfg.profile.is_empty() {
                            format!("container:{}", c)
                        } else {
                            format!("{} (container:{})", cfg.profile, c)
                        }
                    } else {
                        cfg.profile.clone()
                    };
                    serde_json::json!({
                        "type": "testUrlResult",
                        "message": {
                            "url": plan.final_url,
                            "browser": plan.browser_name,
                            "openInBackground": plan.open_in_background,
                            "profile": display_profile,
                            "args": plan.args,
                            "warning": plan.warning
                        }
                    })
                    .to_string()
                }
                Err(e) => serde_json::json!({
                    "type": "testUrlResult",
                    "message": { "error": e }
                })
                .to_string(),
            }
        }
        "getInstalledBrowsers" => {
            let names: Vec<&str> = profile_resolver
                .list_installed_browsers()
                .into_iter()
                .map(|b| b.app_name)
                .collect();
            serde_json::json!({
                "type": "installedBrowsers",
                "message": names
            })
            .to_string()
        }
        "getBrowserProfiles" => {
            let browser = msg.get("browser").and_then(|v| v.as_str()).unwrap_or("");
            let profiles = profile_resolver.list_profiles(browser);
            serde_json::json!({
                "type": "browserProfiles",
                "message": {
                    "browser": browser,
                    "profiles": profiles
                }
            })
            .to_string()
        }
        "setDefaultBrowser" => {
            let ok = crate::macos::set_as_default_browser();
            let eng = engine.lock().unwrap();
            log_event(
                if ok { "info" } else { "warn" },
                if ok {
                    "Requested macOS to set Browsify as the default browser"
                } else {
                    "Could not set Browsify as the default browser outside an .app bundle"
                },
            );
            serde_json::json!({
                "type": "config",
                "message": build_config_message(&eng)
            })
            .to_string()
        }
        "clearLogs" => {
            if let Ok(mut guard) = telemetry().lock() {
                guard.logs.clear();
            }
            "[]".to_string()
        }
        _ => "[]".to_string(),
    }
}

fn guess_mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn write_http_response(
    stream: &mut impl Write,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

const FALLBACK_SPA_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Browsify — URL Tester & Rules Manager</title>
<style>
  body { font-family: system-ui, -apple-system, sans-serif; background: #0d1117; color: #e6edf3; max-width: 780px; margin: 32px auto; padding: 0 16px; }
  .card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 16px; margin-bottom: 16px; }
  input, button { padding: 8px 12px; border-radius: 6px; border: 1px solid #30363d; background: #0d1117; color: #e6edf3; font-size: 14px; }
  button { background: #238636; border-color: #2ea043; cursor: pointer; font-weight: 600; }
  pre { background: #0d1117; padding: 12px; border-radius: 6px; overflow-x: auto; font-size: 13px; }
</style>
</head>
<body>
  <h2>Browsify — On-Demand Localhost UI</h2>
  <div class="card">
    <h3>Test URL Routing & Profile Resolution</h3>
    <div style="display:flex;gap:8px;margin-top:8px;">
      <input id="urlInput" style="flex:1" value="https://github.com/myorg/repo?utm_source=slack" />
      <button onclick="runTest()">Test URL</button>
    </div>
    <pre id="resultBox">Click "Test URL" to inspect resolved browser, profile, and launch flags.</pre>
  </div>
  <script>
    async function runTest() {
      const url = document.getElementById("urlInput").value;
      const res = await fetch("/api/rpc?token=" + encodeURIComponent(window.__BROWSIFY_TOKEN__ || ""), {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Browsify-Token": window.__BROWSIFY_TOKEN__ || ""
        },
        body: JSON.stringify({ type: "testUrl", url })
      });
      const data = await res.json();
      document.getElementById("resultBox").textContent = JSON.stringify(data.message, null, 2);
    }
    if (window.__BROWSIFY_TOKEN__) {
      new EventSource("/api/events?token=" + encodeURIComponent(window.__BROWSIFY_TOKEN__));
    }
  </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_loopback_and_token_csrf_protection() {
        let token = "a1b2c3d4e5f60718293a4b5c6d7e8f90";

        // Valid request with X-Browsify-Token and loopback Origin
        let valid_req = format!(
            "POST /api/rpc HTTP/1.1\r\nHost: 127.0.0.1:54321\r\nOrigin: http://127.0.0.1:54321\r\nX-Browsify-Token: {}\r\n\r\n",
            token
        );
        assert!(validate_loopback_and_token(&valid_req, "", token));

        // Valid SSE request with query token
        let valid_sse = "GET /api/events HTTP/1.1\r\nHost: 127.0.0.1:54321\r\n\r\n";
        assert!(validate_loopback_and_token(
            valid_sse,
            &format!("token={}", token),
            token
        ));

        // Reject missing token
        let no_token = "POST /api/rpc HTTP/1.1\r\nHost: 127.0.0.1:54321\r\n\r\n";
        assert!(!validate_loopback_and_token(no_token, "", token));

        // Reject cross-origin CSRF attempt even with token
        let evil_origin = format!(
            "POST /api/rpc HTTP/1.1\r\nHost: 127.0.0.1:54321\r\nOrigin: https://evil.example.com\r\nX-Browsify-Token: {}\r\n\r\n",
            token
        );
        assert!(!validate_loopback_and_token(&evil_origin, "", token));
    }
}
