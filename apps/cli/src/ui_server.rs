use crate::config_loader::BrowsifyEngine;
use browsify_core::ProfileResolver;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_LOG_BUFFER: usize = 200;
const IDLE_SHUTDOWN_GRACE: Duration = Duration::from_secs(30);
const MAX_HTTP_HEADERS: usize = 16 * 1024;
const MAX_HTTP_BODY: usize = 1024 * 1024;
const HTTP_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Severity of a UI log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: u128,
    pub level: String,
    pub msg: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub has_update: bool,
    pub update_check_enabled: bool,
    pub download_url: String,
    pub release_url: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: Option<String>,
}

impl UpdateInfo {
    /// Update checks are disabled in configuration; nothing to show.
    pub fn disabled() -> Self {
        Self {
            version: String::new(),
            has_update: false,
            update_check_enabled: false,
            download_url: String::new(),
            release_url: String::new(),
            status: "disabled".into(),
            error: None,
        }
    }

    /// A successful check found no newer release.
    pub fn no_update(version: String) -> Self {
        Self {
            version,
            has_update: false,
            update_check_enabled: true,
            download_url: String::new(),
            release_url: String::new(),
            status: "upToDate".into(),
            error: None,
        }
    }

    /// A check ran and produced release URLs.
    pub fn available(
        version: String,
        has_update: bool,
        download_url: String,
        release_url: String,
    ) -> Self {
        Self {
            version,
            has_update,
            update_check_enabled: true,
            download_url,
            release_url,
            status: "available".into(),
            error: None,
        }
    }
}

struct GlobalTelemetryState {
    logs: VecDeque<LogEntry>,
    sse_clients: Vec<TcpStream>,
    update_info: Option<UpdateInfo>,
    active_ui_url: Option<String>,
}

static TELEMETRY: OnceLock<Mutex<GlobalTelemetryState>> = OnceLock::new();

fn telemetry() -> &'static Mutex<GlobalTelemetryState> {
    TELEMETRY.get_or_init(|| {
        Mutex::new(GlobalTelemetryState {
            logs: VecDeque::with_capacity(MAX_LOG_BUFFER),
            sse_clients: Vec::new(),
            update_info: None,
            active_ui_url: None,
        })
    })
}

/// Builds a UI push event (`{"type":…,"message":…}`) shared by SSE and the init RPC.
fn event_value(event_type: &str, message: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "type": event_type, "message": message })
}

fn sse_event(event_type: &str, message: serde_json::Value) -> String {
    event_value(event_type, message).to_string()
}

pub fn log_event(level: LogLevel, msg: impl Into<String>) {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let entry = LogEntry {
        time: now_ms,
        level: level.as_str().to_string(),
        msg: msg.into(),
    };
    let sse_payload = sse_event(
        "log",
        serde_json::to_string(&entry).unwrap_or_default().into(),
    );

    if let Ok(mut guard) = telemetry().lock() {
        if guard.logs.len() >= MAX_LOG_BUFFER {
            guard.logs.pop_front();
        }
        guard.logs.push_back(entry);
        broadcast_sse_locked(&mut guard, &sse_payload);
    }
}

pub fn set_update_info(info: UpdateInfo) {
    let payload = sse_event(
        "updateInfo",
        serde_json::to_value(&info).unwrap_or_default(),
    );
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
    crate::macos::refresh_preferences();
    let msg = build_config_message(engine);
    let payload = sse_event("config", msg);
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
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn spawn_or_open_ui_server(
    engine: Arc<Mutex<BrowsifyEngine>>,
    profile_resolver: Arc<ProfileResolver>,
) {
    open_ui_at(engine, profile_resolver, "/");
}

pub fn open_ui_at(engine: Arc<Mutex<BrowsifyEngine>>, profile_resolver: Arc<ProfileResolver>, page: &'static str) {
    if let Ok(guard) = telemetry().lock() {
        if let Some(existing_url) = &guard.active_ui_url {
            open_url_in_system_browser(&existing_url.replacen("/?", &format!("{page}?"), 1));
            return;
        }
    }
    thread::spawn(move || {
        let _ = run_ui_server_at(engine, profile_resolver, 0, true, page);
    });
}

/// Serves the token-authenticated Browsify UI over `127.0.0.1:<port>` with SSE log streaming
/// and automatic 30-second idle shutdown once no UI tab remains connected (0 MB idle RAM),
/// even if a tab never managed to open the `/api/events` SSE stream.
pub fn run_ui_server(
    engine: Arc<Mutex<BrowsifyEngine>>,
    profile_resolver: Arc<ProfileResolver>,
    port: u16,
    open_browser: bool,
) -> Result<(), String> {
    run_ui_server_at(engine, profile_resolver, port, open_browser, "/")
}

fn run_ui_server_at(engine: Arc<Mutex<BrowsifyEngine>>, profile_resolver: Arc<ProfileResolver>, port: u16, open_browser: bool, page: &str) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("Failed to bind UI server on 127.0.0.1:{}: {}", port, e))?;
    let local_addr = listener.local_addr().map_err(|e| e.to_string())?;
    let session_token = generate_session_token()?;
    let url = format!("http://{}/?token={}", local_addr, session_token);

    if let Ok(mut guard) = telemetry().lock() {
        guard.active_ui_url = Some(url.clone());
    }

    println!("Browsify UI listening on {}", url);
    log_event(
        LogLevel::Info,
        format!("On-demand UI server started on {}", local_addr),
    );

    if open_browser {
        open_url_in_system_browser(&url.replacen("/?", &format!("{page}?"), 1));
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let last_activity = Arc::new(Mutex::new(Instant::now()));

    // Background idle reaper & SSE heartbeat thread:
    // Shuts down the server 30s after the last request or SSE disconnect — including
    // when no tab ever connected, so the daemon never retains an idle UI server.
    {
        let stop_flag = Arc::clone(&stop_flag);
        let last_activity = Arc::clone(&last_activity);
        thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(3));
                if let Ok(mut guard) = telemetry().lock() {
                    guard.sse_clients.retain_mut(|stream| {
                        stream.write_all(b": keepalive\n\n").is_ok() && stream.flush().is_ok()
                    });
                }
                // While any tab holds an SSE stream, keep refreshing the activity
                // timestamp so a later disconnect starts the full 30s grace period.
                if telemetry()
                    .lock()
                    .map(|g| !g.sse_clients.is_empty())
                    .unwrap_or(false)
                {
                    if let Ok(mut t) = last_activity.lock() {
                        *t = Instant::now();
                    }
                    continue;
                }
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
        });
    }

    let dist_dir = resolve_ui_dist_dir().and_then(|dir| fs::canonicalize(dir).ok());

    for stream in listener.incoming() {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
        let Ok(mut stream) = stream else {
            continue;
        };
        let _ = stream.set_write_timeout(Some(HTTP_READ_TIMEOUT));
        let (req, body) = match read_http_request(&mut stream) {
            Ok(request) => request,
            Err(status) => {
                let _ = write_http_response(
                    &mut stream,
                    status,
                    "application/json; charset=utf-8",
                    br#"{"error":"Invalid or incomplete HTTP request"}"#,
                );
                continue;
            }
        };

        if let Ok(mut t) = last_activity.lock() {
            *t = Instant::now();
        }

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
                if let Ok(mut guard) = telemetry().lock() {
                    guard.sse_clients.push(stream);
                }
            }
            continue;
        }

        if method == "POST" && path == "/api/rpc" {
            let response_json = handle_rpc(&body, &engine, &profile_resolver);
            let _ = write_http_response(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                response_json.as_bytes(),
            );
            continue;
        }

        let clean_path = path.trim_start_matches('/');
        // Reject traversal before SPA routing; canonical containment below also
        // prevents symlinks inside the asset directory from exposing local files.
        if method != "GET"
            || !path.starts_with('/')
            || Path::new(clean_path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            let _ = write_http_response(&mut stream, "404 Not Found", "text/plain", b"Not found");
            continue;
        }
        let is_document =
            clean_path.is_empty() || clean_path == "index.html" || !clean_path.contains('.');
        // Assets may be fetched without a token, but never hand out the token
        // itself to an unauthenticated document request.
        if is_document && !validate_loopback_and_token(&req, query, &session_token) {
            let _ = write_http_response(&mut stream, "403 Forbidden", "text/plain", b"Forbidden");
            continue;
        }
        if let Some(ref dir) = dist_dir {
            let target = dir.join(if is_document {
                "index.html"
            } else {
                clean_path
            });
            if let Ok(target) = fs::canonicalize(target) {
                if target.starts_with(dir) && target.is_file() {
                    if let Ok(bytes) = fs::read(&target) {
                        if is_document {
                            let html = String::from_utf8_lossy(&bytes);
                            let injected = inject_session_token(&html, &session_token);
                            let _ = write_document_response(&mut stream, &injected, &session_token);
                        } else {
                            let _ = write_http_response(
                                &mut stream,
                                "200 OK",
                                guess_mime(&target),
                                &bytes,
                            );
                        }
                        continue;
                    }
                }
            }
        } else if is_document {
            let fallback_html = inject_session_token(FALLBACK_SPA_HTML, &session_token);
            let _ = write_document_response(&mut stream, &fallback_html, &session_token);
            continue;
        }
        let _ = write_http_response(&mut stream, "404 Not Found", "text/plain", b"Not found");
    }

    if let Ok(mut guard) = telemetry().lock() {
        guard.active_ui_url = None;
        guard.sse_clients.clear();
    }
    stop_flag.store(true, Ordering::Relaxed);
    Ok(())
}

/// Read a complete, bounded HTTP/1 request. TCP reads need not align with
/// headers, the body, or UTF-8 characters. This server closes after one request
/// and deliberately rejects transfer encodings and ambiguous framing.
fn read_http_request(stream: &mut TcpStream) -> Result<(String, String), &'static str> {
    let started = Instant::now();
    let mut bytes = Vec::new();
    let mut buf = [0u8; 8192];
    let header_end = loop {
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            if end + 4 > MAX_HTTP_HEADERS {
                return Err("413 Payload Too Large");
            }
            break end + 4;
        }
        if bytes.len() >= MAX_HTTP_HEADERS {
            return Err("413 Payload Too Large");
        }
        let remaining = HTTP_READ_TIMEOUT
            .checked_sub(started.elapsed())
            .ok_or("400 Bad Request")?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| "400 Bad Request")?;
        let n = stream.read(&mut buf).map_err(|_| "400 Bad Request")?;
        if n == 0 {
            return Err("400 Bad Request");
        }
        bytes.extend_from_slice(&buf[..n]);
    };
    let headers = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| "400 Bad Request")?
        .to_string();
    let mut content_length = None;
    for line in headers
        .split("\r\n")
        .skip(1)
        .take_while(|line| !line.is_empty())
    {
        let (key, value) = line.split_once(':').ok_or("400 Bad Request")?;
        if key.eq_ignore_ascii_case("transfer-encoding") {
            return Err("400 Bad Request");
        }
        if key.eq_ignore_ascii_case("content-length") {
            let value = value.trim();
            if content_length.is_some()
                || value.is_empty()
                || !value.bytes().all(|b| b.is_ascii_digit())
            {
                return Err("400 Bad Request");
            }
            content_length = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| "413 Payload Too Large")?,
            );
        }
    }
    let body_length = content_length.unwrap_or(0);
    if body_length > MAX_HTTP_BODY {
        return Err("413 Payload Too Large");
    }
    let request_length = header_end + body_length;
    while bytes.len() < request_length {
        let remaining = HTTP_READ_TIMEOUT
            .checked_sub(started.elapsed())
            .ok_or("400 Bad Request")?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| "400 Bad Request")?;
        let size = buf.len().min(request_length - bytes.len());
        let n = stream
            .read(&mut buf[..size])
            .map_err(|_| "400 Bad Request")?;
        if n == 0 {
            return Err("400 Bad Request");
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    let body = std::str::from_utf8(&bytes[header_end..request_length])
        .map_err(|_| "400 Bad Request")?
        .to_string();
    Ok((headers, body))
}

fn validate_loopback_and_token(raw_req: &str, query: &str, expected_token: &str) -> bool {
    let mut header_token: Option<&str> = None;
    let mut cookie_token: Option<&str> = None;
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
            } else if key.eq_ignore_ascii_case("cookie") {
                cookie_token = val.split(';').find_map(|cookie| {
                    let (name, value) = cookie.trim().split_once('=')?;
                    (name == "browsify-session").then_some(value)
                });
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

    let token_matches = header_token == Some(expected_token)
        || query_token == Some(expected_token)
        || cookie_token == Some(expected_token);
    host_ok && origin_ok && token_matches
}

fn generate_session_token() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    let mut f = fs::File::open("/dev/urandom")
        .map_err(|e| format!("Failed to open /dev/urandom for session token: {}", e))?;
    f.read_exact(&mut bytes)
        .map_err(|e| format!("Failed to read session token entropy: {}", e))?;
    let mut hex = String::with_capacity(32);
    for b in bytes {
        hex.push_str(&format!("{:02x}", b));
    }
    Ok(hex)
}

fn inject_session_token(html: &str, token: &str) -> String {
    use sha2::{Digest,Sha256};
    let session=format!("{:x}",Sha256::digest(token.as_bytes()));
    let script = format!("<script>window.__BROWSIFY_SESSION__ = {session:?};window.__BROWSIFY_TOKEN__ = {token:?};</script>");
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
            // 2. Inside `target/release/browsify` or `target/debug/browsify` -> `../../apps/ui/dist`
            let target_rel = exe_dir.join("../../apps/ui/dist");
            if target_rel.join("index.html").is_file() {
                return Some(target_rel);
            }
        }
    }
    let cwd_dist = PathBuf::from("apps/ui/dist");
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
        "defaultProfile": sum.default_profile,

        "configPath": cfg_path,
        "hasPrimaryConfig": sum.has_primary_config,
        "explicitDefaultBrowser": sum.explicit_default_browser,
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

fn import_rpc(
    operation: &str,
    msg: &serde_json::Value,
    engine: &Arc<Mutex<BrowsifyEngine>>,
    home: &Path,
) -> Result<serde_json::Value, String> {
    let destination = engine
        .lock()
        .unwrap()
        .primary_config_path()
        .ok_or("Editing disabled by --no-config")?
        .to_owned();
    match operation {
        "findImport" => Ok(event_value(
            "importSources",
            serde_json::json!(crate::migration::sources(home, &destination, None, None)?),
        )),
        "previewImport" => {
            // Inspect legacy JavaScript outside the routing lock.
            let review = crate::migration::preview(home, &destination, None, None)?;
            let message =
                serde_json::json!({"preview":review,"id":review.id(),"report":review.report()});
            engine.lock().unwrap().pending_import = Some(review);
            Ok(event_value("importPreview", message))
        }
        _ => {
            let mut eng = engine.lock().unwrap();
            let review = eng
                .pending_import
                .as_ref()
                .ok_or("Review the import again before applying it.")?;
            if msg["id"].as_str() != Some(review.id().as_str()) {
                return Err("The preview changed. Review it again.".into());
            }
            let text = review.approval(msg["defaultBrowser"].as_str())?;
            let revision = review.revision.clone();
            eng.commit(&revision, &text)?;
            notify_config_reloaded(&eng);
            let result = event_value("importApplied", build_config_message(&eng));
            drop(eng);
            crate::update::trigger_update_check_if_enabled(engine, home);
            Ok(result)
        }
    }
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
                event_value("version", format!("v{}", env!("CARGO_PKG_VERSION")).into()),
                event_value("buildInfo", format!("({})", sum.format).into()),
                event_value("config", build_config_message(&eng)),
            ];
            if let Ok(guard) = telemetry().lock() {
                if let Some(ref uinfo) = guard.update_info {
                    events.push(event_value(
                        "updateInfo",
                        serde_json::to_value(uinfo).unwrap_or_default(),
                    ));
                }
                for entry in &guard.logs {
                    if let Ok(s) = serde_json::to_string(entry) {
                        events.push(event_value("log", s.into()));
                    }
                }
            }
            serde_json::Value::Array(events).to_string()
        }
        "findImport" | "previewImport" | "approveImport" => {
            import_rpc(msg_type, &msg, engine, profile_resolver.home_dir())
                .unwrap_or_else(|error| {
                    event_value("importError", serde_json::json!({ "error": error }))
                })
                .to_string()
        }
        "getRuleEditor" | "saveRuleEditor" => {
            let mut eng = engine.lock().unwrap();
            let saving = msg_type == "saveRuleEditor";
            let result = if saving {
                serde_json::from_value(msg["payload"].clone())
                    .map_err(|e| e.to_string())
                    .and_then(|request| crate::rule_editor::save(&mut eng, request))
            } else {
                if eng.check_files_changed() {
                    if let Err(error) = eng.reload() {
                        return event_value("ruleEditorError", serde_json::json!({"error": error}))
                            .to_string();
                    }
                }
                crate::rule_editor::snapshot(&eng)
            };
            match result {
                Ok(groups) => {
                    if saving {
                        notify_config_reloaded(&eng);
                        drop(eng);
                        crate::update::trigger_update_check_if_enabled(
                            engine,
                            profile_resolver.home_dir(),
                        );
                    }
                    event_value(
                        "ruleEditor",
                        serde_json::json!({"groups": groups, "saved": saving.then_some("primary")}),
                    )
                    .to_string()
                }
                Err(error) => event_value(
                    "ruleEditorError",
                    serde_json::json!({"error": error, "source": msg["payload"]["source"]}),
                )
                .to_string(),
            }
        }
        "editAdvancedRules" => {
            let path = engine.lock().unwrap().primary_config_path().map(Path::to_path_buf);
            match path.ok_or_else(|| "There is no advanced rules file.".to_owned()).and_then(|p| crate::macos::open_in_textedit(&p)) {
                Ok(()) => "[]".into(),
                Err(error) => event_value("ruleEditorError", serde_json::json!({"error": error})).to_string(),
            }
        }
        "getBrowserChoices" => event_value("browserChoices", serde_json::json!(crate::macos::installed_browsers(profile_resolver))).to_string(),
        "getProfileChoices" => {
            let browser = msg["browser"].as_str().unwrap_or("");
            match profile_resolver.profile_choices(browser) {
                Ok(profiles) => event_value("profileChoices", serde_json::json!({"browser": browser, "profiles": profiles})).to_string(),
                Err(_) => event_value("profileChoices", serde_json::json!({"browser": browser, "error": true, "profiles": []})).to_string(),
            }
        }
        "getLoginItem" | "setLoginItem" => {
            let enabled = msg["enabled"].as_bool();
            let result = if msg_type == "setLoginItem" && enabled.is_none() {
                Err("Choose whether to open at login.".into())
            } else { crate::macos::login_item(enabled.filter(|_| msg_type == "setLoginItem")) };
            match result {
                Ok(status) => event_value("loginItem", serde_json::json!({"status": status})).to_string(),
                Err(error) => event_value("loginItem", serde_json::json!({"error": error, "status": crate::macos::login_item(None).ok()})).to_string(),
            }
        }
        "saveSettings" | "saveRules" => event_value("saveRulesError",serde_json::json!({"error":"Reload the UI to edit the TOML configuration."})).to_string(),
        "checkForUpdates" => {
            crate::update::check_manually(profile_resolver.home_dir());
            serde_json::json!({"type": "updateAction", "message": "Checking for updates…"}).to_string()
        }
        "downloadUpdate" => {
            crate::update::download(profile_resolver.home_dir(), true);
            serde_json::json!({"type": "updateAction", "message": "Downloading update…"})
                .to_string()
        }
        "restartToUpdate" => match crate::update::restart(profile_resolver.home_dir()) {
            Ok(()) => {
                serde_json::json!({"type": "updateAction", "message": "Restarting…"}).to_string()
            }
            Err(error) => serde_json::json!({"type": "updateAction", "message": error}).to_string(),
        },
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
                        "requestId": msg.get("requestId"),
                        "message": {
                            "url": plan.final_url,
                            "browser": plan.browser_name,
                            "openInBackground": plan.open_in_background,
                            "profile": display_profile,
                            "args": plan.args,
                            "warning": plan.warning,
                            "action": plan.action
                        }
                    })
                    .to_string()
                }
                Err(e) => serde_json::json!({
                    "type": "testUrlResult",
                    "requestId": msg.get("requestId"),
                    "message": { "error": e }
                })
                .to_string(),
            }
        }
        "getInstalledBrowsers" => {
            let names = crate::macos::installed_browser_names(profile_resolver);
            serde_json::json!({
                "type": "installedBrowsers",
                "message": names
            })
            .to_string()
        }
        "getBrowserProfiles" => {
            let browser = msg.get("browser").and_then(|v| v.as_str()).unwrap_or("");
            let profiles = match profile_resolver.try_list_profiles(browser) {
                Ok(profiles) => profiles,
                Err(error) => return serde_json::json!({
                    "type": "browserProfilesError",
                    "message": { "browser": browser, "error": error }
                }).to_string(),
            };
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
                if ok { LogLevel::Info } else { LogLevel::Warn },
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

fn write_document_response(
    stream: &mut impl Write,
    html: &str,
    token: &str,
) -> std::io::Result<()> {
    // Keep refreshes/deep links authenticated after client-side navigation drops
    // the bootstrap query. This session cookie is not readable by scripts.
    let headers =
        format!("Set-Cookie: browsify-session={token}; HttpOnly; SameSite=Strict; Path=/\r\n");
    write_http_response_with_headers(
        stream,
        "200 OK",
        "text/html; charset=utf-8",
        &headers,
        html.as_bytes(),
    )
}

fn write_http_response(
    stream: &mut impl Write,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write_http_response_with_headers(stream, status, content_type, "", body)
}

fn write_http_response_with_headers(
    stream: &mut impl Write,
    status: &str,
    content_type: &str,
    extra_headers: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n{}Connection: close\r\n\r\n",
        status,
        content_type,
        body.len(),
        extra_headers,
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
    fn settings_and_rules_commit_together_and_invalid_edits_preserve_the_file() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("primary.toml");
        fs::write(
            &path,
            "# keep my comment\n[options]\ncheck_for_updates=false\n",
        )
        .unwrap();
        let eng = Arc::new(Mutex::new(
            BrowsifyEngine::load(root.path(), Some(&path), None, false).unwrap(),
        ));
        let resolver = Arc::new(ProfileResolver::new(root.path()));
        let rpc = |m: serde_json::Value| -> serde_json::Value {
            serde_json::from_str(&handle_rpc(&m.to_string(), &eng, &resolver)).unwrap()
        };
        let group =
            rpc(serde_json::json!({"type":"getRuleEditor"}))["message"]["groups"][0].clone();
        let result = rpc(
            serde_json::json!({"type":"saveRuleEditor","payload":{"revision":group["revision"],"settings":{"defaultBrowser":"Firefox","options":{"checkForUpdates":false,"logRequests":true}},"rows":[{"rule":{"match":"*github.com/*","browser":"Safari"}}]}}),
        );
        assert_eq!(result["message"]["saved"], "primary", "{result}");
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("# keep my comment"));
        let route = |url| {
            rpc(serde_json::json!({"type":"testUrl","url":url}))["message"]["browser"].clone()
        };
        assert_eq!(route("https://github.com/project"), "Safari");
        assert_eq!(route("https://elsewhere.test"), "Firefox");
        assert!(eng.lock().unwrap().summary().options.log_requests);
        let rejected = rpc(
            serde_json::json!({"type":"saveRuleEditor","payload":{"revision":result["message"]["groups"][0]["revision"],"settings":{"defaultBrowser":""}}}),
        );
        assert_eq!(rejected["type"], "ruleEditorError");
        assert_eq!(fs::read_to_string(&path).unwrap(), after);
        assert_eq!(route("https://elsewhere.test"), "Firefox");
    }

    #[test]
    fn visual_editor_preserves_primary_precedence_and_rejects_stale_writes() {
        use serde_json::{json, Value};
        let source = r#"# hand-authored
    [options]
    check_for_updates = false

    # first handler
    [[handlers]]
    match = "*example.test/*"
    browser = "Safari" # keep this comment

    # exact hostname stays as authored
    [[handlers]]
    match_hostnames = ["other.test"]
    browser = "Firefox"

    [[rewrite]]
    match = "*example.test/*"
    force_https = true
    "#;
        let root = tempfile::tempdir().unwrap();
        let primary_path = root.path().join("primary.toml");
        fs::write(&primary_path, source).unwrap();
        let eng = Arc::new(Mutex::new(
            BrowsifyEngine::load(root.path(), Some(&primary_path), None, false).unwrap(),
        ));
        let resolver = Arc::new(ProfileResolver::new(root.path()));
        let rpc = |message: Value| -> Value {
            serde_json::from_str(&handle_rpc(&message.to_string(), &eng, &resolver)).unwrap()
        };
        let catalog = rpc(json!({"type":"getRuleEditor"}));
        assert_eq!(catalog["type"], "ruleEditor");
        let primary = &catalog["message"]["groups"][0];
        assert_eq!(primary["source"], "primary");
        assert_eq!(primary["rows"][0]["rule"]["browser"], "Safari");
        assert_eq!(primary["rows"][1]["rule"], Value::Null);
        assert_eq!(primary["rewrites"][0], "*example.test/*");
        let route = |url| rpc(json!({"type":"testUrl", "url":url}))["message"].clone();
        let edit = json!({"type":"saveRuleEditor", "payload":{
            "revision":primary["revision"], "rows":[
                {"id":0, "rule":{"match":"*example.test/*", "browser":"Microsoft Edge"}},
                {"id":1}
            ]
        }});
        let result = rpc(edit.clone());
        assert_eq!(result["message"]["saved"], "primary", "{result}");
        assert_eq!(route("https://example.test/")["browser"], "Microsoft Edge");
        let expected = source.replace("\"Safari\"", "\"Microsoft Edge\"");
        let after = fs::read_to_string(&primary_path).unwrap();
        assert_eq!(after, expected);
        let changed = format!("{after}\n# edited in TextEdit\n");
        fs::write(&primary_path, &changed).unwrap();
        let rejected = rpc(edit);
        assert_eq!(rejected["type"], "ruleEditorError");
        assert!(rejected["message"]["error"]
            .as_str()
            .unwrap()
            .contains("changed on disk"));
        assert_eq!(fs::read_to_string(&primary_path).unwrap(), changed);
    }

    #[test]
    fn test_session_token_is_128_bit_random_hex() {
        let a = generate_session_token().unwrap();
        let b = generate_session_token().unwrap();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

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
